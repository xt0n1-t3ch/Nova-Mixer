use audio_sessions::{AudioEvent, AudioService};
use novamixer_contracts::{
    AppCandidate, AppSettings, Application, ApplicationPatch, Group, IdentityKind, MixerSnapshot,
    Scene, SceneEntry,
};
use parking_lot::RwLock;
use settings_store::{LoadReport, SettingsStore};
use std::{
    io,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

/// How long after the last level change the levels are written to disk.
const PERSIST_DEBOUNCE: Duration = Duration::from_millis(400);
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Audio(#[from] audio_sessions::AudioError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("group not found")]
    GroupNotFound,
    #[error("scene not found")]
    SceneNotFound,
    #[error("application not found")]
    AppNotFound,
    #[error("cannot delete the only group")]
    LastGroup,
}
pub type Result<T> = std::result::Result<T, ApplicationError>;
#[derive(Clone)]
pub struct NovaMixerApplication {
    pub audio: AudioService,
    settings: Arc<RwLock<AppSettings>>,
    store: SettingsStore,
    load_report: LoadReport,
    /// Bumped on every level change; only the newest scheduled save runs.
    persist_generation: Arc<AtomicU64>,
}
impl NovaMixerApplication {
    pub fn start(callback: impl Fn(AudioEvent) + Send + Sync + 'static) -> Result<Self> {
        let store = SettingsStore::discover()?;
        let mut report = store.load();
        if enrich_applications(&mut report.settings.applications, store.data_root()) {
            store.save(&report.settings)?;
        }
        let audio = AudioService::start(
            report.settings.applications.clone(),
            report.settings.groups.clone(),
            callback,
        )?;
        Ok(Self {
            audio,
            settings: Arc::new(RwLock::new(report.settings.clone())),
            store,
            load_report: report,
            persist_generation: Arc::new(AtomicU64::new(0)),
        })
    }
    pub fn settings(&self) -> AppSettings {
        self.settings.read().clone()
    }
    pub fn data_root(&self) -> &Path {
        self.store.data_root()
    }
    pub fn migration_ran(&self) -> bool {
        self.load_report.migration_ran
    }
    pub async fn snapshot(&self) -> Result<MixerSnapshot> {
        Ok(self.audio.list_applications().await?)
    }
    /// Writes what the running sessions report about their executables back to
    /// the saved applications (see `refresh_live_identities`), and saves when
    /// anything changed. Returns the settings that were saved, if any.
    ///
    /// Run once after startup: the audio worker adopts the sessions that were
    /// already playing, but nothing else saves until the user changes something,
    /// so without this the stored record keeps naming an install folder that no
    /// longer exists.
    pub async fn sync_live_identities(&self) -> Result<Option<AppSettings>> {
        let snapshot = self.snapshot().await?;
        let mut settings = self.settings();
        if !refresh_live_identities(&mut settings, &snapshot) {
            return Ok(None);
        }
        self.persist_settings(&mut settings)?;
        *self.settings.write() = settings.clone();
        Ok(Some(settings))
    }
    pub async fn save_settings(&self, mut settings: AppSettings) -> Result<()> {
        self.persist_settings(&mut settings)?;
        self.audio
            .update_settings(settings.applications.clone(), settings.groups.clone())
            .await?;
        *self.settings.write() = settings;
        Ok(())
    }
    fn persist_settings(&self, settings: &mut AppSettings) -> Result<()> {
        settings.validate();
        enrich_applications(&mut settings.applications, self.store.data_root());
        self.store.save(settings)?;
        Ok(())
    }
    async fn persist_snapshot(&self) -> Result<()> {
        let snapshot = self.snapshot().await?;
        let mut settings = self.settings();
        settings.applications = snapshot.applications;
        // The snapshot already came from the live registry. Sending it back during every fader
        // write creates a needless rebuild boundary and used to erase sessions absent from disk.
        self.persist_settings(&mut settings)?;
        *self.settings.write() = settings;
        Ok(())
    }
    /// Saves the live levels shortly after the last change instead of on every change.
    ///
    /// A fader drag sends a level every frame. Writing the whole settings file and its
    /// backup for each one put a disk write on the path of every volume change, so the
    /// level lagged behind the fader whenever the disk was slow. The Windows call now
    /// returns at once; one save follows the burst, from a background thread.
    fn schedule_persist(&self) {
        let generation = self.persist_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let app = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(PERSIST_DEBOUNCE);
            if app.persist_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            if let Err(error) = futures_lite::future::block_on(app.persist_snapshot()) {
                tracing::warn!(%error, "saving levels failed");
            }
        });
    }
    pub async fn set_app_volume(&self, key: &str, volume: f32) -> Result<()> {
        self.audio.set_app_volume(key.into(), volume).await?;
        self.schedule_persist();
        Ok(())
    }
    pub async fn set_app_mute(&self, key: &str, muted: bool) -> Result<()> {
        self.audio.set_app_mute(key.into(), muted).await?;
        self.schedule_persist();
        Ok(())
    }
    /// Sets every member of a group to one level, then saves shortly after.
    ///
    /// A hotkey held down fires this many times a second. It used to save the
    /// whole settings file and its backup on every repeat, so presses queued
    /// behind disk writes and concurrent writes failed with "Access is denied",
    /// which made hotkeys look dead. The level now lands at once and the group
    /// value is saved by the same debounced writer as the faders.
    pub async fn set_group_volume(&self, id: &str, volume: f32) -> Result<()> {
        let volume = audio_policy::clamp_scalar(volume);
        let group = {
            let mut settings = self.settings.write();
            let group = settings
                .groups
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or(ApplicationError::GroupNotFound)?;
            group.volume = volume;
            group.clone()
        };
        for key in &group.app_keys {
            // A member that is not running has no session to set; its saved level
            // is still applied when it starts.
            let _ = self.audio.set_app_volume(key.clone(), volume).await;
        }
        self.schedule_persist();
        Ok(())
    }
    pub async fn upsert_group(&self, group: Group) -> Result<Group> {
        let mut s = self.settings();
        if let Some(x) = s.groups.iter_mut().find(|x| x.id == group.id) {
            *x = group.clone()
        } else {
            s.groups.push(group.clone())
        }
        self.save_settings(s).await?;
        Ok(group)
    }
    pub async fn delete_group(&self, id: &str) -> Result<()> {
        let mut s = self.settings();
        if s.groups.len() == 1 {
            return Err(ApplicationError::LastGroup);
        }
        s.groups.retain(|g| g.id != id);
        if s.active_group_id.as_deref() == Some(id) {
            s.active_group_id = s.groups.first().map(|g| g.id.clone())
        }
        self.save_settings(s).await
    }
    pub async fn set_active_group(&self, id: Option<String>) -> Result<()> {
        let mut s = self.settings();
        if id
            .as_ref()
            .is_some_and(|id| !s.groups.iter().any(|g| &g.id == id))
        {
            return Err(ApplicationError::GroupNotFound);
        }
        s.active_group_id = id;
        self.save_settings(s).await
    }
    pub async fn list_candidates(&self) -> Result<Vec<AppCandidate>> {
        let snapshot = self.snapshot().await?;
        let settings = self.settings();
        Ok(snapshot
            .applications
            .into_iter()
            .map(|a| {
                let already_managed = settings
                    .applications
                    .iter()
                    .any(|saved| saved.app_key.eq_ignore_ascii_case(&a.app_key));
                AppCandidate {
                    app_key: a.app_key,
                    display_name: a.display_name,
                    executable_name: a.executable_name,
                    executable_path: a.executable_path,
                    icon: a.icon,
                    running: a.running,
                    already_managed,
                }
            })
            .collect())
    }
    pub async fn add_application(&self, path: &str) -> Result<Application> {
        let metadata = app_icons::metadata_for_path(Path::new(path));
        let key = audio_policy::resolve_app_key(
            None,
            metadata.executable_path.as_deref(),
            metadata.executable_name.as_deref(),
        );
        let name = metadata.version_name.clone().unwrap_or_else(|| {
            app_icons::fallback_name(metadata.executable_name.as_deref().unwrap_or(&key))
        });
        let mut app = Application::offline(key, IdentityKind::Path, name);
        app.executable_name = metadata.executable_name;
        app.executable_path = metadata.executable_path.clone();
        app.icon = metadata
            .executable_path
            .as_ref()
            .and_then(|path| app_icons::IconCache::default().icon_for_path(Path::new(path)));
        let mut s = self.settings();
        if let Some(old) = s.applications.iter().find(|existing| {
            existing.app_key.eq_ignore_ascii_case(&app.app_key)
                || (existing.identity_kind == IdentityKind::Filename
                    && existing
                        .executable_name
                        .as_deref()
                        .zip(app.executable_name.as_deref())
                        .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right)))
        }) {
            return Ok(old.clone());
        }
        app.sort_order = s.applications.len() as u32;
        s.applications.push(app.clone());
        self.save_settings(s).await?;
        self.audio
            .emit_application_event(AudioEvent::ApplicationAdded(app.clone()))
            .await?;
        Ok(app)
    }
    pub async fn remove_application(&self, key: &str) -> Result<()> {
        let mut s = self.settings();
        let before = s.applications.len();
        s.applications.retain(|a| a.app_key != key);
        if before == s.applications.len() {
            return Err(ApplicationError::AppNotFound);
        }
        for g in &mut s.groups {
            g.app_keys.retain(|k| k != key)
        }
        self.save_settings(s).await?;
        self.audio
            .emit_application_event(AudioEvent::ApplicationRemoved {
                app_key: key.to_owned(),
            })
            .await?;
        Ok(())
    }
    pub async fn update_application(
        &self,
        key: &str,
        patch: ApplicationPatch,
    ) -> Result<Application> {
        let mut s = self.settings();
        let app = s
            .applications
            .iter_mut()
            .find(|a| a.app_key == key)
            .ok_or(ApplicationError::AppNotFound)?;
        if let Some(v) = patch.custom_name {
            app.custom_name = v;
            app.display_name = app.custom_name.clone().unwrap_or_else(|| {
                app.executable_path
                    .as_deref()
                    .and_then(|path| app_icons::metadata_for_path(Path::new(path)).version_name)
                    .unwrap_or_else(|| {
                        app_icons::fallback_name(
                            app.executable_name.as_deref().unwrap_or(&app.app_key),
                        )
                    })
            })
        }
        if let Some(v) = patch.remembered {
            app.remembered = v
        }
        if let Some(v) = patch.pinned {
            app.pinned = v
        }
        if let Some(v) = patch.hidden {
            app.hidden = v
        }
        if let Some(v) = patch.group_id {
            app.group_id = v
        }
        let result = app.clone();
        self.save_settings(s).await?;
        self.audio
            .emit_application_event(AudioEvent::ApplicationUpdated(result.clone()))
            .await?;
        Ok(result)
    }
    pub async fn reorder_applications(&self, keys: Vec<String>) -> Result<()> {
        let mut s = self.settings();
        for (i, key) in keys.iter().enumerate() {
            if let Some(a) = s.applications.iter_mut().find(|a| &a.app_key == key) {
                a.sort_order = i as u32
            }
        }
        s.applications.sort_by_key(|a| a.sort_order);
        let updated = s.applications.clone();
        self.save_settings(s).await?;
        for application in updated {
            self.audio
                .emit_application_event(AudioEvent::ApplicationUpdated(application))
                .await?;
        }
        Ok(())
    }
    pub async fn upsert_scene(&self, scene: Scene) -> Result<Scene> {
        let mut s = self.settings();
        if let Some(x) = s.scenes.iter_mut().find(|x| x.id == scene.id) {
            *x = scene.clone()
        } else {
            s.scenes.push(scene.clone())
        }
        self.save_settings(s).await?;
        Ok(scene)
    }
    pub async fn delete_scene(&self, id: &str) -> Result<()> {
        let mut s = self.settings();
        s.scenes.retain(|x| x.id != id);
        self.save_settings(s).await
    }
    pub async fn apply_scene(&self, id: &str) -> Result<()> {
        let scene = self
            .settings()
            .scenes
            .into_iter()
            .find(|s| s.id == id)
            .ok_or(ApplicationError::SceneNotFound)?;
        self.audio.apply_scene(scene).await?;
        self.persist_snapshot().await
    }
    pub async fn capture_scene(&self, name: String) -> Result<Scene> {
        let snap = self.snapshot().await?;
        let scene = Scene {
            id: Uuid::new_v4().to_string(),
            name,
            icon: None,
            master_volume: Some(snap.master.volume),
            entries: snap
                .applications
                .into_iter()
                .map(|a| SceneEntry {
                    app_key: a.app_key,
                    volume: Some(a.volume),
                    muted: Some(a.muted),
                })
                .collect(),
            fade_ms: 0,
        };
        self.upsert_scene(scene).await
    }
    pub fn restore_backup(&self) -> Result<AppSettings> {
        let mut v = self.store.restore_backup()?;
        if enrich_applications(&mut v.applications, self.store.data_root()) {
            self.store.save(&v)?;
        }
        *self.settings.write() = v.clone();
        Ok(v)
    }
}

/// Updates each saved application from the live application with the same
/// `app_key` in `snapshot`: executable path, executable name, and, when the
/// path changed, the icon and discovered name read from that executable.
/// User choices (custom name, level, pin, group) are untouched. Returns `true`
/// when anything changed.
///
/// One key can cover several install folders (Squirrel.Windows
/// `app-<version>`), so this is what moves a saved record from a removed
/// version folder to the one that runs now.
pub fn refresh_live_identities(settings: &mut AppSettings, snapshot: &MixerSnapshot) -> bool {
    let mut changed = false;
    for live in snapshot.applications.iter().filter(|app| app.running) {
        if let Some(saved) = settings
            .applications
            .iter_mut()
            .find(|saved| saved.app_key.eq_ignore_ascii_case(&live.app_key))
        {
            changed |= audio_sessions::refresh_from_live(saved, live);
        }
    }
    changed
}

fn enrich_applications(applications: &mut [Application], _data_root: &Path) -> bool {
    // Persisting the data URL in settings keeps the cache tied to its application record and
    // avoids a second index/cleanup contract for small (at most 128 px, roughly 10–30 KB)
    // derived PNGs.
    let icons = app_icons::IconCache::default();
    let mut changed = false;
    for app in applications {
        let Some(path) = app.executable_path.as_deref() else {
            continue;
        };
        let metadata = app_icons::metadata_for_path(Path::new(path));
        if app.executable_name.is_none() && metadata.executable_name.is_some() {
            app.executable_name = metadata.executable_name.clone();
            changed = true;
        }
        let display_name = app.custom_name.clone().unwrap_or_else(|| {
            metadata.version_name.unwrap_or_else(|| {
                app_icons::fallback_name(app.executable_name.as_deref().unwrap_or_else(|| {
                    Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(path)
                }))
            })
        });
        if app.display_name != display_name {
            app.display_name = display_name;
            changed = true;
        }
        if app.icon.is_none() {
            if let Some(icon) = icons.icon_for_path(Path::new(path)) {
                app.icon = Some(icon);
                changed = true;
            }
        }
    }
    changed
}
