use audio_sessions::{AudioEvent, AudioService};
use novamixer_contracts::{AppBinding, AppSettings, Group, MixerSnapshot};
use parking_lot::RwLock;
use settings_store::{LoadReport, SettingsStore};
use std::{io, sync::Arc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Audio(#[from] audio_sessions::AudioError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("group not found")]
    GroupNotFound,
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
        let audio = AudioService::start(report.settings.groups.clone(), callback)?;
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
    pub fn data_root(&self) -> &std::path::Path {
        self.store.data_root()
    }
    pub fn migration_ran(&self) -> bool {
        self.load_report.migration_ran
    }
    pub async fn snapshot(&self) -> Result<MixerSnapshot> {
        Ok(self.audio.list_sessions().await?)
    }
    pub async fn save_settings(&self, mut settings: AppSettings) -> Result<()> {
        settings.validate();
        self.store.save(&settings)?;
        self.audio.update_groups(settings.groups.clone()).await?;
        *self.settings.write() = settings;
        Ok(())
    }
    pub async fn set_group_volume(&self, group_id: &str, volume: f32) -> Result<()> {
        let sessions = self.audio.list_sessions().await?;
        let groups = self.settings.read().groups.clone();
        let group = groups
            .iter()
            .find(|group| group.id == group_id)
            .ok_or(ApplicationError::GroupNotFound)?;
        for session in sessions.sessions.iter().filter(|session| {
            audio_policy::group_for(&session.app_key, std::slice::from_ref(group)).is_some()
        }) {
            let _ = self
                .audio
                .set_session_volume(session.live_id.clone(), audio_policy::clamp_scalar(volume))
                .await;
        }
        self.settings
            .write()
            .groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .expect("group checked")
            .volume = audio_policy::clamp_scalar(volume);
        self.store.save(&self.settings.read())?;
        Ok(())
    }
    pub async fn upsert_group(&self, group: Group) -> Result<Group> {
        let mut settings = self.settings();
        if let Some(existing) = settings
            .groups
            .iter_mut()
            .find(|value| value.id == group.id)
        {
            *existing = group.clone();
        } else {
            settings.groups.push(group.clone());
        }
        self.save_settings(settings).await?;
        Ok(group)
    }
    pub async fn delete_group(&self, group_id: &str) -> Result<()> {
        let mut settings = self.settings();
        if settings.groups.len() == 1 {
            return Err(ApplicationError::LastGroup);
        }
        settings.groups.retain(|group| group.id != group_id);
        if settings.active_group_id.as_deref() == Some(group_id) {
            settings.active_group_id = settings.groups.first().map(|group| group.id.clone());
        }
        self.save_settings(settings).await
    }
    pub async fn set_active_group(&self, group_id: Option<String>) -> Result<()> {
        let mut settings = self.settings();
        if let Some(id) = group_id.as_deref() {
            if !settings.groups.iter().any(|group| group.id == id) {
                return Err(ApplicationError::GroupNotFound);
            }
        }
        settings.active_group_id = group_id;
        self.save_settings(settings).await
    }
    pub async fn running_apps(&self) -> Result<Vec<AppBinding>> {
        let mut apps = Vec::new();
        for session in self.snapshot().await?.sessions {
            if !apps
                .iter()
                .any(|app: &AppBinding| app.app_key == session.app_key)
            {
                apps.push(AppBinding {
                    app_key: session.app_key,
                    display_name: session.display_name,
                    executable_name: session.executable_name,
                    executable_path: session.executable_path,
                });
            }
        }
        Ok(apps)
    }
    pub fn restore_backup(&self) -> Result<AppSettings> {
        let value = self.store.restore_backup()?;
        *self.settings.write() = value.clone();
        Ok(value)
    }
}
