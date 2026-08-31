use novamixer_contracts::{
    AppPeak, Application, AudioDevice, AudioSession, Group, IdentityKind, MasterState,
    MixerSnapshot, PeakBatch, Scene, SessionPeak, SessionState,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
    time::Instant,
};
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Debug, Error, Clone)]
pub enum AudioError {
    #[error("audio service is unavailable: {0}")]
    Unavailable(String),
    #[error("audio session no longer exists")]
    SessionGone,
    #[error("application is not managed")]
    AppUnknown,
    #[error("audio session cannot be controlled")]
    NotControllable,
    #[error("invalid volume scalar")]
    InvalidVolume,
    #[error("operation is unsupported: {0}")]
    Unsupported(String),
}
pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug, Clone)]
pub enum AudioEvent {
    ApplicationAdded(Application),
    ApplicationUpdated(Application),
    ApplicationRemoved { app_key: String },
    MasterUpdated(MasterState),
    EndpointChanged(MixerSnapshot),
    Peaks(PeakBatch),
}

pub trait SessionHandle {
    fn snapshot(&self) -> AudioSession;
    fn application(&self) -> Application {
        let session = self.snapshot();
        let mut app = Application::offline(
            session.app_key.clone(),
            IdentityKind::Filename,
            session.display_name.clone(),
        );
        app.executable_name = Some(session.app_key.clone());
        app
    }
    fn set_volume(&mut self, volume: f32) -> Result<()>;
    fn set_mute(&mut self, muted: bool) -> Result<()>;
    fn peak(&self) -> Option<f32>;
}
pub trait SessionSource {
    type Handle: SessionHandle;
    fn master(&self) -> Result<MasterState>;
    fn enumerate(&mut self) -> Result<Vec<Self::Handle>>;
    fn set_master_volume(&mut self, volume: f32) -> Result<()>;
    fn set_master_mute(&mut self, muted: bool) -> Result<()>;
    fn master_peak(&self) -> f32;
    fn list_output_devices(&self) -> Result<Vec<AudioDevice>> {
        Err(AudioError::Unsupported(
            "output device enumeration is unavailable".into(),
        ))
    }
    fn set_default_output(&mut self, _device_id: &str) -> Result<()> {
        Err(AudioError::Unsupported(
            "changing the default output is unavailable".into(),
        ))
    }
}

pub struct Registry<S: SessionSource> {
    source: S,
    sessions: HashMap<String, S::Handle>,
    applications: HashMap<String, Application>,
    groups: Vec<Group>,
    policy_applied: HashSet<String>,
}
impl<S: SessionSource> Registry<S> {
    pub fn start(
        mut source: S,
        applications: Vec<Application>,
        mut groups: Vec<Group>,
    ) -> Result<Self> {
        let handles = source.enumerate()?;
        let applications = deduplicate_applications(applications, &mut groups);
        let mut value = Self {
            source,
            sessions: HashMap::new(),
            applications,
            groups,
            policy_applied: HashSet::new(),
        };
        for h in handles {
            value.adopt(h, false)?;
        }
        Ok(value)
    }
    pub fn snapshot(&self) -> Result<MixerSnapshot> {
        let mut applications: Vec<_> = self
            .applications
            .keys()
            .filter_map(|k| self.aggregate(k))
            .collect();
        applications.sort_by_key(|a| (!a.pinned, a.sort_order));
        Ok(MixerSnapshot {
            master: self.source.master()?,
            applications,
        })
    }
    fn aggregate(&self, key: &str) -> Option<Application> {
        let mut app = self.applications.get(key)?.clone();
        let mut sessions: Vec<_> = self
            .sessions
            .values()
            .map(SessionHandle::snapshot)
            .filter(|s| s.app_key == key && s.state != SessionState::Expired)
            .collect();
        sessions.sort_by(|a, b| a.live_id.cmp(&b.live_id));
        app.running = !sessions.is_empty();
        app.controllable = sessions.iter().any(|s| s.controllable);
        app.peak = sessions.iter().map(|s| s.peak).fold(0.0, f32::max);
        if !sessions.is_empty() {
            let first_v = sessions[0].volume;
            let first_m = sessions[0].muted;
            app.mixed = !self.policy_applied.contains(key)
                && sessions
                    .iter()
                    .any(|s| (s.volume - first_v).abs() > 0.0001 || s.muted != first_m);
            if app.mixed {
                app.volume = first_v;
                app.muted = first_m;
            } else if self.policy_applied.contains(key) {
            } else {
                app.volume = first_v;
                app.muted = first_m;
            }
        }
        app.sessions = sessions;
        Some(app)
    }
    pub fn adopt(
        &mut self,
        mut handle: S::Handle,
        apply_policy: bool,
    ) -> Result<Option<Application>> {
        let s = handle.snapshot();
        if s.state == SessionState::Expired || self.sessions.contains_key(&s.live_id) {
            return Ok(None);
        }
        let discovered = handle.application();
        let key = discovered.app_key.clone();
        let upgraded_from = self.merge_key_for(&discovered);
        let existed = self.applications.contains_key(&key) || upgraded_from.is_some();
        if let Some(old_key) = upgraded_from {
            let persisted = self
                .applications
                .remove(&old_key)
                .expect("merge key exists");
            let merged = merge_discovered_application(persisted, &discovered);
            replace_group_key(&mut self.groups, &old_key, &key);
            self.applications.insert(key.clone(), merged);
        } else if !existed {
            let mut app = discovered;
            app.sort_order = self.applications.len() as u32;
            self.applications.insert(key.clone(), app);
        }
        if apply_policy {
            if let Some(app) = self.applications.get(&key).filter(|a| a.remembered) {
                handle.set_mute(app.muted)?;
                handle.set_volume(app.volume)?;
                self.policy_applied.insert(key.clone());
            }
        }
        self.sessions.insert(s.live_id, handle);
        Ok(self.aggregate(&key).filter(|_| !existed || apply_policy))
    }
    fn merge_key_for(&self, discovered: &Application) -> Option<String> {
        let discovered_path = discovered.executable_path.as_deref()?;
        let discovered_name = Path::new(discovered_path).file_name()?.to_str()?;
        let path_collision = self.applications.values().any(|app| {
            app.identity_kind == IdentityKind::Path
                && app.executable_path.as_deref().is_some_and(|path| {
                    !path.eq_ignore_ascii_case(discovered_path)
                        && Path::new(path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.eq_ignore_ascii_case(discovered_name))
                })
        });
        if path_collision {
            return None;
        }
        self.applications
            .iter()
            .find(|(_, app)| {
                app.identity_kind == IdentityKind::Filename
                    && app
                        .executable_name
                        .as_deref()
                        .unwrap_or(&app.app_key)
                        .eq_ignore_ascii_case(discovered_name)
            })
            .map(|(key, _)| key.clone())
    }

    pub fn set_app_volume(&mut self, key: &str, volume: f32) -> Result<Application> {
        validate_volume(volume)?;
        if !self.applications.contains_key(key) {
            return Err(AudioError::AppUnknown);
        }
        self.policy_applied.insert(key.into());
        self.applications.get_mut(key).unwrap().volume = volume;
        let ids: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, h)| h.snapshot().app_key == key)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(h) = self.sessions.get_mut(&id) {
                if h.snapshot().controllable {
                    let _ = h.set_volume(volume);
                }
            }
        }
        Ok(self.aggregate(key).unwrap())
    }
    pub fn set_app_mute(&mut self, key: &str, muted: bool) -> Result<Application> {
        if !self.applications.contains_key(key) {
            return Err(AudioError::AppUnknown);
        }
        self.policy_applied.insert(key.into());
        self.applications.get_mut(key).unwrap().muted = muted;
        for h in self
            .sessions
            .values_mut()
            .filter(|h| h.snapshot().app_key == key)
        {
            if h.snapshot().controllable {
                let _ = h.set_mute(muted);
            }
        }
        Ok(self.aggregate(key).unwrap())
    }
    pub fn set_session_volume(&mut self, id: &str, v: f32) -> Result<AudioSession> {
        validate_volume(v)?;
        let h = self.sessions.get_mut(id).ok_or(AudioError::SessionGone)?;
        h.set_volume(v)?;
        Ok(h.snapshot())
    }
    pub fn set_session_mute(&mut self, id: &str, m: bool) -> Result<AudioSession> {
        let h = self.sessions.get_mut(id).ok_or(AudioError::SessionGone)?;
        h.set_mute(m)?;
        Ok(h.snapshot())
    }
    pub fn remove(&mut self, id: &str) -> Option<Application> {
        let key = self.sessions.remove(id)?.snapshot().app_key;
        self.aggregate(&key)
    }
    pub fn reconcile(&mut self) -> Result<Vec<AudioEvent>> {
        let handles = self.source.enumerate()?;
        let seen: HashSet<_> = handles.iter().map(|h| h.snapshot().live_id).collect();
        let old: Vec<_> = self
            .sessions
            .keys()
            .filter(|id| !seen.contains(*id))
            .cloned()
            .collect();
        let mut events = vec![];
        for id in old {
            if let Some(app) = self.remove(&id) {
                events.push(AudioEvent::ApplicationUpdated(app));
            }
        }
        for h in handles {
            if let Some(app) = self.adopt(h, true)? {
                events.push(if app.sessions.len() == 1 {
                    AudioEvent::ApplicationAdded(app)
                } else {
                    AudioEvent::ApplicationUpdated(app)
                });
            }
        }
        Ok(events)
    }
    pub fn peaks(&self, started: Instant) -> PeakBatch {
        let applications = self
            .applications
            .keys()
            .filter_map(|key| {
                let sessions: Vec<_> = self
                    .sessions
                    .values()
                    .filter(|h| h.snapshot().app_key == *key)
                    .filter_map(|h| {
                        h.peak().map(|peak| SessionPeak {
                            live_id: h.snapshot().live_id,
                            peak,
                        })
                    })
                    .collect();
                (!sessions.is_empty()).then(|| AppPeak {
                    app_key: key.clone(),
                    peak: sessions.iter().map(|s| s.peak).fold(0.0, f32::max),
                    sessions,
                })
            })
            .collect();
        PeakBatch {
            timestamp_ms: started.elapsed().as_millis() as u64,
            master_peak: self.source.master_peak(),
            applications,
        }
    }
    pub fn update_settings(&mut self, applications: Vec<Application>, mut groups: Vec<Group>) {
        self.applications = deduplicate_applications(applications, &mut groups);
        self.groups = groups;
    }
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }
}
fn deduplicate_applications(
    applications: Vec<Application>,
    groups: &mut [Group],
) -> HashMap<String, Application> {
    let mut deduplicated = HashMap::<String, Application>::new();
    for app in applications {
        let app_key = app.app_key.clone();
        if deduplicated.contains_key(&app_key) {
            continue;
        }
        let filename = application_filename(&app);
        let filename_match = filename.as_deref().and_then(|name| {
            deduplicated
                .iter()
                .find(|(_, existing)| {
                    application_filename(existing)
                        .as_deref()
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
                        && (existing.identity_kind == IdentityKind::Filename
                            || app.identity_kind == IdentityKind::Filename)
                })
                .map(|(key, _)| key.clone())
        });
        if let Some(old_key) = filename_match {
            let old = deduplicated
                .remove(&old_key)
                .expect("deduplication key exists");
            let (survivor, removed_key) = if app.identity_kind == IdentityKind::Filename {
                (old, app_key)
            } else {
                (merge_discovered_application(old, &app), old_key)
            };
            let survivor_key = survivor.app_key.clone();
            replace_group_key(groups, &removed_key, &survivor_key);
            deduplicated.insert(survivor_key, survivor);
        } else {
            deduplicated.insert(app_key, app);
        }
    }
    deduplicated
}

fn application_filename(app: &Application) -> Option<String> {
    app.executable_name.clone().or_else(|| {
        app.executable_path
            .as_deref()
            .and_then(|path| Path::new(path).file_name())
            .map(|name| name.to_string_lossy().into_owned())
    })
}

fn merge_discovered_application(
    mut persisted: Application,
    discovered: &Application,
) -> Application {
    persisted.app_key = discovered.app_key.clone();
    persisted.identity_kind = discovered.identity_kind.clone();
    persisted.executable_name = discovered.executable_name.clone();
    persisted.executable_path = discovered.executable_path.clone();
    persisted.icon = discovered.icon.clone().or(persisted.icon);
    if persisted.custom_name.is_none() {
        persisted.display_name = discovered.display_name.clone();
    }
    persisted
}

fn replace_group_key(groups: &mut [Group], old_key: &str, new_key: &str) {
    for group in groups {
        for app_key in &mut group.app_keys {
            if app_key.eq_ignore_ascii_case(old_key) {
                *app_key = new_key.to_owned();
            }
        }
        let mut seen = HashSet::new();
        group
            .app_keys
            .retain(|app_key| seen.insert(app_key.to_lowercase()));
    }
}

fn validate_volume(v: f32) -> Result<()> {
    if v.is_finite() && (0.0..=1.0).contains(&v) {
        Ok(())
    } else {
        Err(AudioError::InvalidVolume)
    }
}

type EventCallback = Arc<dyn Fn(AudioEvent) + Send + Sync + 'static>;
#[derive(Clone)]
pub struct AudioService {
    sender: crossbeam_channel::Sender<Command>,
}
impl AudioService {
    pub fn start(
        applications: Vec<Application>,
        groups: Vec<Group>,
        callback: impl Fn(AudioEvent) + Send + Sync + 'static,
    ) -> Result<Self> {
        platform::spawn(applications, groups, Arc::new(callback))
    }
    async fn request<T>(
        &self,
        make: impl FnOnce(oneshot::Sender<Result<T>>) -> Command,
    ) -> Result<T> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(make(tx))
            .map_err(|_| AudioError::Unavailable("worker stopped".into()))?;
        rx.await
            .map_err(|_| AudioError::Unavailable("worker dropped reply".into()))?
    }
    pub async fn list_applications(&self) -> Result<MixerSnapshot> {
        self.request(Command::Snapshot).await
    }
    pub async fn set_app_volume(&self, app_key: String, volume: f32) -> Result<()> {
        self.request(|reply| Command::SetAppVolume {
            app_key,
            volume,
            reply,
        })
        .await
    }
    pub async fn set_app_mute(&self, app_key: String, muted: bool) -> Result<()> {
        self.request(|reply| Command::SetAppMute {
            app_key,
            muted,
            reply,
        })
        .await
    }
    pub async fn set_session_volume(&self, live_id: String, volume: f32) -> Result<()> {
        self.request(|reply| Command::SetSessionVolume {
            live_id,
            volume,
            reply,
        })
        .await
    }
    pub async fn set_session_mute(&self, live_id: String, muted: bool) -> Result<()> {
        self.request(|reply| Command::SetSessionMute {
            live_id,
            muted,
            reply,
        })
        .await
    }
    pub async fn set_master_volume(&self, volume: f32) -> Result<()> {
        self.request(|reply| Command::SetMasterVolume { volume, reply })
            .await
    }
    pub async fn set_master_mute(&self, muted: bool) -> Result<()> {
        self.request(|reply| Command::SetMasterMute { muted, reply })
            .await
    }
    pub async fn set_metering_active(&self, active: bool) -> Result<()> {
        self.request(|reply| Command::SetMeteringActive { active, reply })
            .await
    }
    pub async fn update_settings(
        &self,
        applications: Vec<Application>,
        groups: Vec<Group>,
    ) -> Result<()> {
        self.request(|reply| Command::UpdateSettings {
            applications,
            groups,
            reply,
        })
        .await
    }
    pub async fn list_output_devices(&self) -> Result<Vec<AudioDevice>> {
        self.request(Command::ListOutputDevices).await
    }
    pub async fn set_default_output(&self, device_id: String) -> Result<()> {
        self.request(|reply| Command::SetDefaultOutput { device_id, reply })
            .await
    }
    pub async fn apply_scene(&self, scene: Scene) -> Result<()> {
        self.request(|reply| Command::ApplyScene { scene, reply })
            .await
    }
    pub async fn emit_application_event(&self, event: AudioEvent) -> Result<()> {
        self.request(|reply| Command::EmitEvent { event, reply })
            .await
    }
}
#[allow(dead_code)]
pub(crate) enum Command {
    Snapshot(oneshot::Sender<Result<MixerSnapshot>>),
    SetAppVolume {
        app_key: String,
        volume: f32,
        reply: oneshot::Sender<Result<()>>,
    },
    SetAppMute {
        app_key: String,
        muted: bool,
        reply: oneshot::Sender<Result<()>>,
    },
    SetSessionVolume {
        live_id: String,
        volume: f32,
        reply: oneshot::Sender<Result<()>>,
    },
    SetSessionMute {
        live_id: String,
        muted: bool,
        reply: oneshot::Sender<Result<()>>,
    },
    SetMasterVolume {
        volume: f32,
        reply: oneshot::Sender<Result<()>>,
    },
    SetMasterMute {
        muted: bool,
        reply: oneshot::Sender<Result<()>>,
    },
    SetMeteringActive {
        active: bool,
        reply: oneshot::Sender<Result<()>>,
    },
    UpdateSettings {
        applications: Vec<Application>,
        groups: Vec<Group>,
        reply: oneshot::Sender<Result<()>>,
    },
    ListOutputDevices(oneshot::Sender<Result<Vec<AudioDevice>>>),
    SetDefaultOutput {
        device_id: String,
        reply: oneshot::Sender<Result<()>>,
    },
    ApplyScene {
        scene: Scene,
        reply: oneshot::Sender<Result<()>>,
    },
    EmitEvent {
        event: AudioEvent,
        reply: oneshot::Sender<Result<()>>,
    },
    #[cfg(windows)]
    AdoptGit(u32),
    #[cfg(windows)]
    Reenumerate,
    #[cfg(windows)]
    SessionChanged(String),
    #[cfg(windows)]
    SessionRemoved(String),
    #[cfg(windows)]
    RebuildEndpoint,
    Shutdown,
}
#[cfg(windows)]
mod platform;
#[cfg(not(windows))]
mod platform {
    use super::*;
    pub fn spawn(_a: Vec<Application>, _g: Vec<Group>, _c: EventCallback) -> Result<AudioService> {
        Err(AudioError::Unavailable(
            "Core Audio requires Windows".into(),
        ))
    }
}
