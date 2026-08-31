use audio_policy::percent_to_scalar;
use novamixer_contracts::{
    AppBinding, AppSettings, Density, Group, HotkeyAction, HotkeyBinding, Theme, UiPrefs,
};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
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
        if exe_dir.join("portable.txt").is_file() {
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
            Ok(settings) => LoadReport {
                settings,
                migration_ran: false,
                recovered_backup: false,
            },
            Err(main_err) => {
                if main.exists() {
                    warn!(error = %main_err, path = %main.display(), "settings file is unreadable; trying backup");
                }
                match read_settings(&self.backup_path()) {
                    Ok(settings) => {
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

    pub fn save(&self, settings: &AppSettings) -> io::Result<()> {
        fs::create_dir_all(&self.data_root)?;
        let path = self.settings_path();
        if path.is_file() {
            fs::copy(&path, self.backup_path())?;
        }
        atomic_write_json(&path, settings)
    }

    pub fn restore_backup(&self) -> io::Result<AppSettings> {
        let settings = read_settings(&self.backup_path())?;
        self.save(&settings)?;
        Ok(settings)
    }

    fn migrate_legacy(&self) -> io::Result<AppSettings> {
        let bytes = fs::read(&self.legacy_path)?;
        let legacy: LegacyConfig = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let mut settings = legacy.into_settings();
        settings.validate();
        fs::create_dir_all(&self.data_root)?;
        fs::copy(
            &self.legacy_path,
            self.data_root.join("settings.legacy.json"),
        )?;
        atomic_write_json(&self.settings_path(), &settings)?;
        info!(source = %self.legacy_path.display(), destination = %self.settings_path().display(), "migrated legacy settings");
        Ok(settings)
    }
}

fn read_settings(path: &Path) -> io::Result<AppSettings> {
    let bytes = fs::read(path)?;
    let mut settings: AppSettings = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    settings.validate();
    Ok(settings)
}

fn atomic_write_json(path: &Path, settings: &AppSettings) -> io::Result<()> {
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    let mut file = File::create(&temp)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;
    drop(file);
    if path.exists() {
        #[cfg(windows)]
        fs::remove_file(path)?;
    }
    fs::rename(temp, path)
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
            schema_version: 1,
            ui_prefs: UiPrefs {
                theme: Theme::Dark,
                language: if self.language == 1 { "es" } else { "en" }.into(),
                sidebar_collapsed: self.sidebar_collapsed,
                density: Density::Comfy,
                show_inactive: true,
                show_system_sounds: true,
            },
            groups: self
                .groups
                .into_iter()
                .map(|group| Group {
                    is_default: group.is_default || group.id.eq_ignore_ascii_case(&default_id),
                    id: group.id,
                    name: group.name,
                    volume: percent_to_scalar(group.volume_percent),
                    apps: group
                        .processes
                        .into_iter()
                        .map(|process| AppBinding {
                            app_key: process.process_name.to_lowercase(),
                            display_name: process
                                .display_name
                                .unwrap_or_else(|| process.process_name.clone()),
                            executable_name: Some(process.process_name),
                            executable_path: process.exe_path,
                        })
                        .collect(),
                    startup_volume: group.startup_volume_percent.map(percent_to_scalar),
                    auto_mute_on_launch: group.auto_mute_on_launch,
                    hotkeys_enabled: group.enable_hotkeys,
                })
                .collect(),
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
        }
    }
}

fn nonempty_string(value: String) -> Option<String> {
    (!value.trim().is_empty() && value != "00000000-0000-0000-0000-000000000000").then_some(value)
}

fn hotkey(action: HotkeyAction, vk: i32) -> HotkeyBinding {
    let accelerator = match vk {
        175 => Some("MediaVolumeUp".into()),
        174 => Some("MediaVolumeDown".into()),
        173 => Some("MediaVolumeMute".into()),
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
                .apps
                .iter()
                .map(|app| app.app_key.as_str())
                .collect::<Vec<_>>(),
            vec!["msedge.exe", "thorium.exe", "spotify.exe"]
        );
        assert_eq!(
            report
                .settings
                .hotkeys
                .iter()
                .map(|item| item.accelerator.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("MediaVolumeUp"),
                Some("MediaVolumeDown"),
                Some("MediaVolumeMute")
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
        assert_eq!(read_settings(&store.backup_path()).unwrap(), first);
        assert_eq!(read_settings(&store.settings_path()).unwrap(), second);
    }
}
