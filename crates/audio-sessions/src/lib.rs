use novamixer_contracts::{
    AudioSession, Group, MasterState, MixerSnapshot, PeakBatch, SessionPeak, SessionState,
};
use std::{
    collections::{HashMap, HashSet},
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
    #[error("audio session cannot be controlled")]
    NotControllable,
    #[error("invalid volume scalar")]
    InvalidVolume,
}
pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug, Clone)]
pub enum AudioEvent {
    SessionAdded(AudioSession),
    SessionUpdated(AudioSession),
    SessionRemoved { live_id: String },
    MasterUpdated(MasterState),
    EndpointChanged(MixerSnapshot),
    Peaks(PeakBatch),
}

pub trait SessionHandle {
    fn snapshot(&self) -> AudioSession;
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
}

pub struct Registry<S: SessionSource> {
    source: S,
    sessions: HashMap<String, S::Handle>,
    groups: Vec<Group>,
}
impl<S: SessionSource> Registry<S> {
    pub fn start(mut source: S, groups: Vec<Group>) -> Result<Self> {
        let handles = source.enumerate()?;
        let mut value = Self {
            source,
            sessions: HashMap::new(),
            groups,
        };
        for handle in handles {
            value.adopt(handle, false)?;
        }
        Ok(value)
    }
    pub fn snapshot(&self) -> Result<MixerSnapshot> {
        Ok(MixerSnapshot {
            master: self.source.master()?,
            sessions: self
                .sessions
                .values()
                .map(SessionHandle::snapshot)
                .collect(),
        })
    }
    pub fn adopt(
        &mut self,
        mut handle: S::Handle,
        apply_policy: bool,
    ) -> Result<Option<AudioSession>> {
        let initial = handle.snapshot();
        if initial.state == SessionState::Expired || self.sessions.contains_key(&initial.live_id) {
            return Ok(None);
        }
        if apply_policy {
            let policy = audio_policy::policy_for_new_session(audio_policy::group_for(
                &initial.app_key,
                &self.groups,
            ));
            if let Some(muted) = policy.muted {
                handle.set_mute(muted)?;
            }
            if let Some(volume) = policy.volume {
                handle.set_volume(volume)?;
            }
        }
        let snapshot = handle.snapshot();
        self.sessions.insert(snapshot.live_id.clone(), handle);
        Ok(Some(snapshot))
    }
    pub fn set_session_volume(&mut self, live_id: &str, volume: f32) -> Result<AudioSession> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(AudioError::InvalidVolume);
        }
        let handle = self
            .sessions
            .get_mut(live_id)
            .ok_or(AudioError::SessionGone)?;
        handle.set_volume(volume)?;
        Ok(handle.snapshot())
    }
    pub fn set_session_mute(&mut self, live_id: &str, muted: bool) -> Result<AudioSession> {
        let handle = self
            .sessions
            .get_mut(live_id)
            .ok_or(AudioError::SessionGone)?;
        handle.set_mute(muted)?;
        Ok(handle.snapshot())
    }
    pub fn remove(&mut self, live_id: &str) -> bool {
        self.sessions.remove(live_id).is_some()
    }
    pub fn reconcile(&mut self) -> Result<Vec<AudioEvent>> {
        let handles = self.source.enumerate()?;
        let seen: HashSet<String> = handles
            .iter()
            .map(|handle| handle.snapshot().live_id)
            .collect();
        let mut events = Vec::new();
        let removed: Vec<String> = self
            .sessions
            .keys()
            .filter(|id| !seen.contains(*id))
            .cloned()
            .collect();
        for live_id in removed {
            self.sessions.remove(&live_id);
            events.push(AudioEvent::SessionRemoved { live_id });
        }
        for handle in handles {
            if let Some(session) = self.adopt(handle, true)? {
                events.push(AudioEvent::SessionAdded(session));
            }
        }
        Ok(events)
    }
    pub fn peaks(&self, started: Instant) -> PeakBatch {
        PeakBatch {
            timestamp_ms: started.elapsed().as_millis() as u64,
            master_peak: self.source.master_peak(),
            sessions: self
                .sessions
                .iter()
                .filter_map(|(id, handle)| {
                    handle.peak().map(|peak| SessionPeak {
                        live_id: id.clone(),
                        peak,
                    })
                })
                .collect(),
        }
    }
    pub fn update_groups(&mut self, groups: Vec<Group>) {
        self.groups = groups;
    }
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }
}

type EventCallback = Arc<dyn Fn(AudioEvent) + Send + Sync + 'static>;
pub struct AudioService {
    sender: crossbeam_channel::Sender<Command>,
}
impl Clone for AudioService {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}
impl AudioService {
    pub fn start(
        groups: Vec<Group>,
        callback: impl Fn(AudioEvent) + Send + Sync + 'static,
    ) -> Result<Self> {
        platform::spawn(groups, Arc::new(callback))
    }
    async fn request<T>(
        &self,
        make: impl FnOnce(oneshot::Sender<Result<T>>) -> Command,
    ) -> Result<T> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(make(reply))
            .map_err(|_| AudioError::Unavailable("worker stopped".into()))?;
        receive
            .await
            .map_err(|_| AudioError::Unavailable("worker dropped reply".into()))?
    }
    pub async fn list_sessions(&self) -> Result<MixerSnapshot> {
        self.request(Command::Snapshot).await
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
    pub async fn update_groups(&self, groups: Vec<Group>) -> Result<()> {
        self.request(|reply| Command::UpdateGroups { groups, reply })
            .await
    }
}

#[allow(dead_code)]
pub(crate) enum Command {
    Snapshot(oneshot::Sender<Result<MixerSnapshot>>),
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
    UpdateGroups {
        groups: Vec<Group>,
        reply: oneshot::Sender<Result<()>>,
    },
    #[cfg(windows)]
    AdoptGit(u32),
    #[cfg(windows)]
    Reenumerate,
    #[cfg(windows)]
    RebuildEndpoint,
    #[allow(dead_code)]
    Shutdown,
}

#[cfg(windows)]
mod platform;
#[cfg(not(windows))]
mod platform {
    use super::*;
    pub fn spawn(_groups: Vec<Group>, _callback: EventCallback) -> Result<AudioService> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        std::thread::Builder::new()
            .name("windows-audio".into())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        Command::Snapshot(reply) => {
                            let _ = reply.send(Err(AudioError::Unavailable(
                                "Core Audio requires Windows".into(),
                            )));
                        }
                        Command::SetSessionVolume { reply, .. }
                        | Command::SetSessionMute { reply, .. }
                        | Command::SetMasterVolume { reply, .. }
                        | Command::SetMasterMute { reply, .. }
                        | Command::SetMeteringActive { reply, .. }
                        | Command::UpdateGroups { reply, .. } => {
                            let _ = reply.send(Err(AudioError::Unavailable(
                                "Core Audio requires Windows".into(),
                            )));
                        }
                        Command::Shutdown => break,
                    }
                }
            })
            .map_err(|err| AudioError::Unavailable(err.to_string()))?;
        Ok(AudioService { sender })
    }
}
