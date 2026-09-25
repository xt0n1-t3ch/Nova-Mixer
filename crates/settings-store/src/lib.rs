use audio_policy::percent_to_scalar;
use novamixer_contracts::{
    AppSettings, Application, Density, Group, HotkeyAction, HotkeyBinding, IdentityKind, Theme,
    UiPrefs,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, SystemTime},
};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{error, info, warn};

#[derive(Debug, Clone, PartialEq)]
pub struct LoadReport {
    pub settings: AppSettings,
    pub migration_ran: bool,
    pub recovered_backup: bool,
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    data_root: PathBuf,
    legacy_path: PathBuf,
}

impl SettingsStore {
    pub fn discover() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let exe_dir = exe.parent().unwrap_or(Path::new("."));
        // The marker name is owned by product.toml ([distribution.portable]).
        if exe_dir.join("portable.flag").is_file() {
            return Ok(Self::with_paths(exe_dir.join("data"), PathBuf::new()));
        }
        let roaming = dirs::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "APPDATA is unavailable"))?;
        Ok(Self::with_paths(
            roaming.join("NovaMixer").join("v2"),
            roaming.join("NovaMixer").join("config.json"),
        ))
    }

    pub fn with_paths(data_root: PathBuf, legacy_path: PathBuf) -> Self {
        Self {
            data_root,
            legacy_path,
        }
    }

    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    pub fn settings_path(&self) -> PathBuf {
        self.data_root.join("settings.json")
    }

    pub fn backup_path(&self) -> PathBuf {
        self.data_root.join("settings.backup.json")
    }

    pub fn load(&self) -> LoadReport {
        self.remove_stale_temp_files();
        let main = self.settings_path();
        if !main.exists() && self.legacy_path.is_file() {
            match self.migrate_legacy() {
                Ok(settings) => {
                    return LoadReport {
                        settings,
                        migration_ran: true,
                        recovered_backup: false,
                    }
                }
                Err(err) => {
                    error!(error = %err, path = %self.legacy_path.display(), "legacy settings migration failed")
                }
            }
        }
        match read_settings(&main) {
            Ok((settings, migrated)) => {
                if migrated {
                    let _guard = self.write_guard();
                    if let Err(error) = atomic_write_json(&main, &settings) {
                        error!(%error, path = %main.display(), "cannot write migrated settings");
                    }
                }
                LoadReport {
                    settings,
                    migration_ran: migrated,
                    recovered_backup: false,
                }
            }
            Err(main_err) => {
                if main.exists() {
                    warn!(error = %main_err, path = %main.display(), "settings file is unreadable; trying backup");
                }
                match read_settings(&self.backup_path()) {
                    Ok((settings, _)) => {
                        warn!(path = %self.backup_path().display(), "recovered settings from backup");
                        LoadReport {
                            settings,
                            migration_ran: false,
                            recovered_backup: true,
                        }
                    }
                    Err(backup_err) => {
                        if self.backup_path().exists() {
                            error!(error = %backup_err, path = %self.backup_path().display(), "settings backup is unreadable; using defaults");
                        }
                        LoadReport {
                            settings: AppSettings::default(),
                            migration_ran: false,
                            recovered_backup: false,
                        }
                    }
                }
            }
        }
    }

    /// Writes `settings.json`, first copying the previous good file to
    /// `settings.backup.json`.
    ///
    /// Saves are serialized per data folder: the debounced autosave, command
    /// handlers, and hotkey actions can all save in the same instant, and on
    /// Windows a replace that races another writer's open or replace of the same
    /// file fails with "Access is denied".
    pub fn save(&self, settings: &AppSettings) -> io::Result<()> {
        let _guard = self.write_guard();
        self.save_locked(settings)
    }

    fn save_locked(&self, settings: &AppSettings) -> io::Result<()> {
        fs::create_dir_all(&self.data_root)?;
        let path = self.settings_path();
        if path.is_file() && read_settings(&path).is_ok() {
            atomic_copy(&path, &self.backup_path())?;
        }
        atomic_write_json(&path, settings)
    }

    pub fn restore_backup(&self) -> io::Result<AppSettings> {
        let _guard = self.write_guard();
        let (settings, _) = read_settings(&self.backup_path())?;
        self.save_locked(&settings)?;
        Ok(settings)
    }

    fn write_guard(&self) -> std::sync::MutexGuard<'static, ()> {
        static LOCKS: OnceLock<Mutex<HashMap<PathBuf, &'static Mutex<()>>>> = OnceLock::new();
        let lock = *LOCKS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(self.data_root.clone())
            // One small lock per data folder for the life of the process.
            .or_insert_with(|| Box::leak(Box::new(Mutex::new(()))));
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Removes temp files abandoned by an interrupted or failed atomic write.
    ///
    /// Only names this store generates are touched
    /// (`.settings.json.<pid>.<uuid>.tmp`, `.settings.backup.json.<pid>.<uuid>.tmp`),
    /// and only once they are older than `STALE_TEMP_AGE`. A live write lasts
    /// milliseconds, so the age alone is conclusive. Process liveness is not used:
    /// process IDs are reused, and a running instance can itself own leaked files.
    fn remove_stale_temp_files(&self) {
        const STALE_TEMP_AGE: Duration = Duration::from_secs(5 * 60);
        let Ok(entries) = fs::read_dir(&self.data_root) else {
            return;
        };
        let now = SystemTime::now();
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !name.to_str().is_some_and(is_settings_temp_name) {
                continue;
            }
            let stale = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age >= STALE_TEMP_AGE);
            if !stale {
                continue;
            }
            match fs::remove_file(entry.path()) {
                Ok(()) => info!(path = %entry.path().display(), "removed stale settings temp file"),
                Err(error) => {
                    warn!(%error, path = %entry.path().display(), "cannot remove stale settings temp file")
                }
            }
        }
    }

    fn migrate_legacy(&self) -> io::Result<AppSettings> {
        let bytes = fs::read(&self.legacy_path)?;
        let legacy: LegacyConfig = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let mut settings = legacy.into_settings();
        migrate_identities(&mut settings);
        settings.validate();
        fs::create_dir_all(&self.data_root)?;
        let _guard = self.write_guard();
        fs::copy(
            &self.legacy_path,
            self.data_root.join("settings.legacy.json"),
        )?;
        atomic_write_json(&self.settings_path(), &settings)?;
        info!(source = %self.legacy_path.display(), destination = %self.settings_path().display(), "migrated legacy settings");
        Ok(settings)
    }
}

fn read_settings(path: &Path) -> io::Result<(AppSettings, bool)> {
    let bytes = fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    let schema_migrated = value.get("schema_version").and_then(|v| v.as_u64()) == Some(1);
    let mut settings = if schema_migrated {
        migrate_v1(value)?
    } else {
        serde_json::from_value(value).map_err(io::Error::other)?
    };
    let identities_migrated = migrate_identities(&mut settings);
    settings.validate();
    Ok((settings, schema_migrated || identities_migrated))
}

/// Applies the current `app_key` rule to stored path identities and collapses
/// entries that now name the same application (for example one per Squirrel
/// `app-<version>` folder). See `audio_policy::migrate_path_identities`.
fn migrate_identities(settings: &mut AppSettings) -> bool {
    let changed = audio_policy::migrate_path_identities(
        &mut settings.applications,
        &mut settings.groups,
        &mut settings.scenes,
        |path| Path::new(path).is_file(),
    );
    if changed {
        info!("collapsed stored application identities that now share one app_key");
    }
    changed
}

fn migrate_v1(mut value: serde_json::Value) -> io::Result<AppSettings> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| io::Error::other("settings root is not an object"))?;
    object.insert("schema_version".into(), 2.into());
    object
        .entry("scenes")
        .or_insert_with(|| serde_json::json!([]));
    object.entry("efficiency_mode").or_insert(false.into());
    if let Some(ui) = object.get_mut("ui_prefs").and_then(|v| v.as_object_mut()) {
        if let Some(show) = ui.remove("show_inactive") {
            ui.insert("show_offline".into(), show);
        }
        ui.entry("show_hidden").or_insert(false.into());
    }
    let mut applications = Vec::new();
    if let Some(groups) = object.get_mut("groups").and_then(|v| v.as_array_mut()) {
        for group in groups {
            if let Some(g) = group.as_object_mut() {
                let apps = g
                    .remove("apps")
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                let mut keys = Vec::new();
                for item in apps {
                    if let Some(key) = item.get("app_key").and_then(|v| v.as_str()) {
                        let executable_path = item.get("executable_path").and_then(|v| v.as_str());
                        let app_key = audio_policy::resolve_app_key(
                            None,
                            executable_path,
                            item.get("executable_name")
                                .and_then(|v| v.as_str())
                                .or(Some(key)),
                        );
                        keys.push(app_key.clone());
                        let mut app = Application::offline(
                            app_key,
                            if executable_path.is_some() {
                                IdentityKind::Path
                            } else {
                                IdentityKind::Filename
                            },
                            item.get("display_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(key)
                                .to_owned(),
                        );
                        app.executable_name = item
                            .get("executable_name")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned);
                        app.executable_path = item
                            .get("executable_path")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned);
                        app.group_id = g.get("id").and_then(|v| v.as_str()).map(str::to_owned);
                        applications.push(app);
                    }
                }
                g.insert("app_keys".into(), serde_json::json!(keys));
            }
        }
    }
    object.insert(
        "applications".into(),
        serde_json::to_value(applications).map_err(io::Error::other)?,
    );
    serde_json::from_value(value).map_err(io::Error::other)
}

fn atomic_write_json(path: &Path, settings: &AppSettings) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    write_through_temp(path, |file| {
        file.write_all(&bytes)?;
        file.write_all(b"\n")
    })
}

fn atomic_copy(source: &Path, destination: &Path) -> io::Result<()> {
    let mut source = File::open(source)?;
    write_through_temp(destination, |file| io::copy(&mut source, file).map(drop))
}

/// Writes a sibling temp file with `fill`, flushes it to disk, and swaps it into
/// `destination`. Whatever step fails, the temp file is removed and the error
/// is logged and returned.
fn write_through_temp(
    destination: &Path,
    fill: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let temp = temporary_path(destination);
    let result = File::create(&temp).and_then(|mut file| {
        fill(&mut file)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        replace_file(&temp, destination)
    });
    if let Err(error) = &result {
        error!(%error, path = %destination.display(), "atomic settings write failed");
        if let Err(cleanup) = fs::remove_file(&temp) {
            if cleanup.kind() != io::ErrorKind::NotFound {
                warn!(error = %cleanup, path = %temp.display(), "cannot remove settings temp file");
            }
        }
    }
    result
}

fn is_settings_temp_name(name: &str) -> bool {
    let Some(inner) = name
        .strip_prefix('.')
        .and_then(|name| name.strip_suffix(".tmp"))
    else {
        return false;
    };
    let mut parts = inner.rsplitn(3, '.');
    let (Some(id), Some(pid), Some(target)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    matches!(target, "settings.json" | "settings.backup.json")
        && pid.parse::<u32>().is_ok()
        && uuid::Uuid::parse_str(id).is_ok()
}

fn temporary_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    destination.with_file_name(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4()
    ))
}

/// Replaces `destination` with `temp` in one step.
///
/// Deleting the destination first and then renaming leaves a window with no
/// settings file at all, and loses the previous file outright if the rename then
/// fails. `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` performs the swap as a
/// single operation, so a reader sees either the old file or the new one.
#[cfg(windows)]
fn replace_file(temp: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let from = wide(temp);
    let to = wide(destination);
    // Another process (antivirus, the search indexer, a sync client) can hold
    // the destination open for a moment. Those failures are transient, so the
    // swap is retried briefly before the error is returned.
    let mut attempt = 0;
    loop {
        // SAFETY: both buffers are null-terminated and outlive the call.
        let result = unsafe {
            MoveFileExW(
                PCWSTR(from.as_ptr()),
                PCWSTR(to.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        let Err(error) = result else {
            return Ok(());
        };
        let code = error.code().0 as u32;
        let error = if code & 0xFFFF_0000 == 0x8007_0000 {
            io::Error::from_raw_os_error((code & 0xFFFF) as i32)
        } else {
            io::Error::other(error.message())
        };
        // ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION.
        let transient = matches!(error.raw_os_error(), Some(5 | 32 | 33));
        attempt += 1;
        if !transient || attempt >= 5 {
            return Err(error);
        }
        std::thread::sleep(Duration::from_millis(20 * attempt));
    }
}

#[cfg(not(windows))]
fn replace_file(temp: &Path, destination: &Path) -> io::Result<()> {
    // `rename` already replaces the destination atomically on POSIX.
    fs::rename(temp, destination)
}

pub struct DebouncedAutosave {
    sender: mpsc::UnboundedSender<AppSettings>,
    task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl DebouncedAutosave {
    pub fn new(store: SettingsStore) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            while let Some(mut pending) = receiver.recv().await {
                while let Ok(Some(newer)) =
                    tokio::time::timeout(Duration::from_millis(500), receiver.recv()).await
                {
                    pending = newer;
                }
                if let Err(err) = store.save(&pending) {
                    error!(error = %err, "debounced settings save failed");
                }
            }
        });
        Self {
            sender,
            task: Arc::new(Mutex::new(Some(task))),
        }
    }

    pub fn schedule(&self, settings: AppSettings) {
        if self.sender.send(settings).is_err() {
            error!("debounced settings worker has stopped");
        }
    }
}

impl Drop for DebouncedAutosave {
    fn drop(&mut self) {
        if Arc::strong_count(&self.task) == 1 {
            if let Some(task) = self
                .task
                .lock()
                .expect("autosave task mutex poisoned")
                .take()
            {
                task.abort();
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LegacyConfig {
    language: i32,
    volume_step_percent: f32,
    smart_volume: bool,
    launch_on_startup: bool,
    start_minimized: bool,
    minimize_to_tray: bool,
    auto_save: bool,
    sidebar_collapsed: bool,
    active_group_id: String,
    default_group_id: String,
    volume_up_key: i32,
    volume_down_key: i32,
    mute_key: i32,
    groups: Vec<LegacyGroup>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LegacyGroup {
    id: String,
    name: String,
    is_default: bool,
    volume_percent: f32,
    processes: Vec<LegacyProcess>,
    startup_volume_percent: Option<f32>,
    auto_mute_on_launch: bool,
    enable_hotkeys: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LegacyProcess {
    process_name: String,
    display_name: Option<String>,
    exe_path: Option<String>,
}

impl LegacyConfig {
    fn into_settings(self) -> AppSettings {
        let default_id = self.default_group_id;
        AppSettings {
            schema_version: 2,
            ui_prefs: UiPrefs {
                theme: Theme::Dark,
                language: if self.language == 1 { "es" } else { "en" }.into(),
                sidebar_collapsed: self.sidebar_collapsed,
                density: Density::Comfy,
                show_offline: true,
                show_hidden: false,
                show_system_sounds: true,
            },
            applications: self
                .groups
                .iter()
                .flat_map(|group| {
                    group.processes.iter().map(|process| {
                        let app_key = audio_policy::resolve_app_key(
                            None,
                            process.exe_path.as_deref(),
                            Some(&process.process_name),
                        );
                        let mut app = Application::offline(
                            app_key,
                            if process.exe_path.is_some() {
                                IdentityKind::Path
                            } else {
                                IdentityKind::Filename
                            },
                            process
                                .display_name
                                .clone()
                                .unwrap_or_else(|| process.process_name.clone()),
                        );
                        app.executable_name = Some(process.process_name.clone());
                        app.executable_path = process.exe_path.clone();
                        app.group_id = Some(group.id.clone());
                        app
                    })
                })
                .collect(),
            groups: self
                .groups
                .into_iter()
                .map(|group| Group {
                    is_default: group.is_default || group.id.eq_ignore_ascii_case(&default_id),
                    id: group.id,
                    name: group.name,
                    volume: percent_to_scalar(group.volume_percent),
                    app_keys: group
                        .processes
                        .into_iter()
                        .map(|process| {
                            audio_policy::resolve_app_key(
                                None,
                                process.exe_path.as_deref(),
                                Some(&process.process_name),
                            )
                        })
                        .collect(),
                    startup_volume: group.startup_volume_percent.map(percent_to_scalar),
                    auto_mute_on_launch: group.auto_mute_on_launch,
                    hotkeys_enabled: group.enable_hotkeys,
                })
                .collect(),
            scenes: vec![],
            active_group_id: nonempty_string(self.active_group_id),
            hotkeys: vec![
                hotkey(HotkeyAction::VolumeUp, self.volume_up_key),
                hotkey(HotkeyAction::VolumeDown, self.volume_down_key),
                hotkey(HotkeyAction::MuteToggle, self.mute_key),
            ],
            volume_step: percent_to_scalar(self.volume_step_percent),
            smart_volume: self.smart_volume,
            launch_on_startup: self.launch_on_startup,
            start_minimized: self.start_minimized,
            minimize_to_tray: self.minimize_to_tray,
            auto_save: self.auto_save,
            efficiency_mode: false,
        }
    }
}

fn nonempty_string(value: String) -> Option<String> {
    (!value.trim().is_empty() && value != "00000000-0000-0000-0000-000000000000").then_some(value)
}

fn hotkey(action: HotkeyAction, vk: i32) -> HotkeyBinding {
    let accelerator = match vk {
        175 => Some("AudioVolumeUp".into()),
        174 => Some("AudioVolumeDown".into()),
        173 => Some("AudioVolumeMute".into()),
        0 => None,
        value => {
            warn!(
                virtual_key = value,
                "legacy hotkey has no Tauri accelerator mapping"
            );
            None
        }
    };
    HotkeyBinding {
        action,
        accelerator,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrates_real_legacy_fixture_exactly() {
        let root = tempdir().unwrap();
        let legacy = root.path().join("config.json");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy-config.json"),
            &legacy,
        )
        .unwrap();
        let store = SettingsStore::with_paths(root.path().join("v2"), legacy);
        let report = store.load();
        assert!(report.migration_ran);
        assert_eq!(report.settings.groups.len(), 1);
        let group = &report.settings.groups[0];
        assert_eq!(group.name, "Main");
        assert!(group.is_default);
        assert_eq!(group.volume, 0.375);
        assert_eq!(
            group
                .app_keys
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec![
                "c:\\program files (x86)\\microsoft\\edge\\application\\msedge.exe",
                "c:\\users\\xt0n1\\appdata\\local\\thorium\\application\\thorium.exe",
                "c:\\users\\xt0n1\\appdata\\roaming\\spotify\\spotify.exe"
            ]
        );
        assert_eq!(
            report
                .settings
                .hotkeys
                .iter()
                .map(|item| item.accelerator.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("AudioVolumeUp"),
                Some("AudioVolumeDown"),
                Some("AudioVolumeMute")
            ]
        );
        assert_eq!(report.settings.volume_step, 0.05);
        assert!(
            report.settings.smart_volume
                && report.settings.minimize_to_tray
                && report.settings.auto_save
        );
        assert!(store.data_root().join("settings.legacy.json").is_file());
        assert!(store.settings_path().is_file());
    }

    #[test]
    fn migrates_snake_case_v1_once_and_uses_known_path() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1-settings.json"),
            store.settings_path(),
        )
        .unwrap();
        let first = store.load();
        assert!(first.migration_ran);
        assert_eq!(first.settings.schema_version, 2);
        assert_eq!(first.settings.applications.len(), 1);
        assert_eq!(
            first.settings.applications[0].identity_kind,
            IdentityKind::Path
        );
        assert_eq!(
            first.settings.applications[0].app_key,
            "c:\\users\\test\\appdata\\roaming\\spotify\\spotify.exe"
        );
        assert_eq!(
            first.settings.groups[0].app_keys,
            vec![first.settings.applications[0].app_key.clone()]
        );
        let raw: serde_json::Value =
            serde_json::from_slice(&fs::read(store.settings_path()).unwrap()).unwrap();
        assert_eq!(raw["schema_version"], 2);
        assert!(!store.load().migration_ran);
    }

    #[test]
    fn squirrel_version_entries_load_as_one_application() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        let base = "c:\\users\\xt0n1\\appdata\\local\\discord";
        let old_key = format!("{base}\\app-1.0.9256\\discord.exe");
        let new_key = format!("{base}\\app-1.0.9258\\discord.exe");
        let mut settings = AppSettings::default();
        let mut old = Application::offline(old_key.clone(), IdentityKind::Path, "Discord".into());
        old.executable_path = Some(old_key.clone());
        old.volume = 0.25;
        old.pinned = true;
        old.group_id = Some(settings.groups[0].id.clone());
        let mut new = Application::offline(new_key.clone(), IdentityKind::Path, "Discord".into());
        new.executable_path = Some(new_key.clone());
        new.remembered = false;
        settings.applications = vec![old, new];
        settings.groups[0].app_keys = vec![old_key.clone()];
        fs::write(
            store.settings_path(),
            serde_json::to_vec(&settings).unwrap(),
        )
        .unwrap();

        let report = store.load();

        let merged_key = format!("{base}\\app-*\\discord.exe");
        assert!(report.migration_ran);
        assert_eq!(report.settings.applications.len(), 1);
        let app = &report.settings.applications[0];
        assert_eq!(app.app_key, merged_key);
        assert_eq!(app.volume, 0.25);
        assert!(app.pinned && app.remembered);
        assert_eq!(app.group_id, Some(report.settings.groups[0].id.clone()));
        assert_eq!(report.settings.groups[0].app_keys, vec![merged_key.clone()]);
        // The collapsed form is written back, so the next start is a no-op.
        assert!(!store.load().migration_ran);
        assert_eq!(
            read_settings(&store.settings_path())
                .unwrap()
                .0
                .applications[0]
                .app_key,
            merged_key
        );
    }

    fn temp_files(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect()
    }

    #[test]
    fn failed_replace_removes_its_temp_file_and_returns_the_error() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        // A directory where settings.json belongs makes the final replace fail
        // after the temp file has been fully written.
        fs::create_dir_all(store.settings_path()).unwrap();
        assert!(store.save(&AppSettings::default()).is_err());
        assert_eq!(temp_files(root.path()), Vec::<String>::new());

        // The same for the backup copy, which is written first.
        fs::remove_dir(store.settings_path()).unwrap();
        store.save(&AppSettings::default()).unwrap();
        fs::create_dir_all(store.backup_path()).unwrap();
        assert!(store.save(&AppSettings::default()).is_err());
        assert_eq!(temp_files(root.path()), Vec::<String>::new());
    }

    #[test]
    fn concurrent_saves_leave_only_settings_and_backup() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        store.save(&AppSettings::default()).unwrap();
        let writers = 8;
        let barrier = Arc::new(std::sync::Barrier::new(writers));
        let handles: Vec<_> = (0..writers)
            .map(|writer| {
                let store = store.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut settings = AppSettings::default();
                    let mut failures = vec![];
                    for round in 0..25 {
                        settings.volume_step = (writer * 25 + round) as f32 / 1000.0;
                        barrier.wait();
                        if let Err(error) = store.save(&settings) {
                            failures.push(error.to_string());
                        }
                    }
                    failures
                })
            })
            .collect();
        let failures: Vec<String> = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(failures, Vec::<String>::new(), "every save must succeed");
        let mut names: Vec<String> = fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec!["settings.backup.json", "settings.json"]);
        assert!(read_settings(&store.settings_path()).is_ok());
        assert!(read_settings(&store.backup_path()).is_ok());
    }

    #[test]
    fn load_removes_only_stale_settings_temp_files() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        store.save(&AppSettings::default()).unwrap();
        let stale = [
            ".settings.json.9228.752e03e3-290a-4205-be3b-4ee7f607da4e.tmp",
            ".settings.backup.json.13900.270453cc-ce50-4d66-ab59-ae5d2d213b7b.tmp",
        ];
        let fresh = ".settings.json.4242.6c7f5289-05d0-483f-b0e9-adee3e8bcde8.tmp";
        let unrelated = ".other.json.9228.0d835604-b740-4c38-9ec8-e500a496e70f.tmp";
        let old = std::time::SystemTime::now() - Duration::from_secs(3600);
        for name in stale.iter().chain([&unrelated]) {
            let file = File::create(root.path().join(name)).unwrap();
            file.set_modified(old).unwrap();
        }
        File::create(root.path().join(fresh)).unwrap();

        store.load();

        let mut left = temp_files(root.path());
        left.sort();
        assert_eq!(left, vec![unrelated.to_owned(), fresh.to_owned()]);
    }

    #[test]
    fn corrupt_main_recovers_backup() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        fs::create_dir_all(root.path()).unwrap();
        fs::write(store.settings_path(), b"bad").unwrap();
        fs::write(
            store.backup_path(),
            serde_json::to_vec(&AppSettings::default()).unwrap(),
        )
        .unwrap();
        assert!(store.load().recovered_backup);
    }

    #[test]
    fn save_keeps_previous_file_as_backup() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        let first = AppSettings::default();
        store.save(&first).unwrap();
        let mut second = first.clone();
        second.volume_step = 0.2;
        store.save(&second).unwrap();
        assert_eq!(read_settings(&store.backup_path()).unwrap().0, first);
        assert_eq!(read_settings(&store.settings_path()).unwrap().0, second);
    }

    #[test]
    fn save_does_not_replace_good_backup_with_corrupt_main() {
        let root = tempdir().unwrap();
        let store = SettingsStore::with_paths(root.path().to_path_buf(), PathBuf::new());
        let backup = AppSettings::default();
        let mut previous = backup.clone();
        previous.volume_step = 0.1;
        store.save(&backup).unwrap();
        store.save(&previous).unwrap();
        fs::write(store.settings_path(), b"corrupt").unwrap();

        let mut current = backup.clone();
        current.volume_step = 0.2;
        store.save(&current).unwrap();

        assert_eq!(read_settings(&store.backup_path()).unwrap().0, backup);
        assert_eq!(read_settings(&store.settings_path()).unwrap().0, current);
    }
}
