use audio_sessions::{AudioEvent, AudioService};
use novamixer_contracts::{
    AppCandidate, AppSettings, Application, ApplicationPatch, Group, IdentityKind, MixerSnapshot,
    Scene, SceneEntry,
};
use parking_lot::RwLock;
use settings_store::{LoadReport, SettingsStore};
use std::{io, path::Path, sync::Arc};
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
}
impl NovaMixerApplication {
    pub fn start(callback: impl Fn(AudioEvent) + Send + Sync + 'static) -> Result<Self> {
        let store = SettingsStore::discover()?;
        let report = store.load();
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
    pub async fn save_settings(&self, mut settings: AppSettings) -> Result<()> {
        settings.validate();
        self.store.save(&settings)?;
        self.audio
            .update_settings(settings.applications.clone(), settings.groups.clone())
            .await?;
        *self.settings.write() = settings;
        Ok(())
    }
    async fn persist_snapshot(&self) -> Result<()> {
        let snapshot = self.snapshot().await?;
        let mut settings = self.settings();
        settings.applications = snapshot.applications;
        self.save_settings(settings).await
    }
    pub async fn set_app_volume(&self, key: &str, volume: f32) -> Result<()> {
        self.audio.set_app_volume(key.into(), volume).await?;
        self.persist_snapshot().await
    }
    pub async fn set_app_mute(&self, key: &str, muted: bool) -> Result<()> {
        self.audio.set_app_mute(key.into(), muted).await?;
        self.persist_snapshot().await
    }
    pub async fn set_group_volume(&self, id: &str, volume: f32) -> Result<()> {
        let group = self
            .settings()
            .groups
            .into_iter()
            .find(|g| g.id == id)
            .ok_or(ApplicationError::GroupNotFound)?;
        for key in &group.app_keys {
            let _ = self
                .audio
                .set_app_volume(key.clone(), audio_policy::clamp_scalar(volume))
                .await;
        }
        let mut s = self.settings();
        s.groups.iter_mut().find(|g| g.id == id).unwrap().volume =
            audio_policy::clamp_scalar(volume);
        self.save_settings(s).await
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
        Ok(snapshot
            .applications
            .into_iter()
            .map(|a| AppCandidate {
                app_key: a.app_key,
                display_name: a.display_name,
                executable_name: a.executable_name,
                executable_path: a.executable_path,
                icon: a.icon,
                running: a.running,
                already_managed: true,
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
            metadata
                .executable_name
                .as_deref()
                .and_then(|n| Path::new(n).file_stem())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| key.clone())
        });
        let mut app = Application::offline(key, IdentityKind::Path, name);
        app.executable_name = metadata.executable_name;
        app.executable_path = metadata.executable_path.clone();
        app.icon = metadata
            .executable_path
            .as_ref()
            .and_then(|p| app_icons::IconCache::default().icon_for_path(Path::new(p)));
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
                app.executable_name
                    .clone()
                    .unwrap_or_else(|| app.app_key.clone())
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
        let v = self.store.restore_backup()?;
        *self.settings.write() = v.clone();
        Ok(v)
    }
}
