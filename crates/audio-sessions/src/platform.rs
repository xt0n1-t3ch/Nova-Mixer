use super::*;
use app_icons::{metadata_for_process, IconCache};
use std::{
    path::Path,
    ptr, thread,
    time::{Duration, Instant},
};
use tracing::{debug, error, warn};
use windows::{
    core::{Interface, GUID, PWSTR},
    Win32::{
        Foundation::S_OK,
        Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation},
        Media::Audio::{
            eConsole, eRender, AudioSessionState, AudioSessionStateActive,
            AudioSessionStateInactive, IAudioSessionControl, IAudioSessionControl2,
            IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume,
            MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IGlobalInterfaceTable,
            CLSCTX_ALL, COINIT_MULTITHREADED,
        },
    },
};

const CLSID_STD_GLOBAL_INTERFACE_TABLE: GUID =
    GUID::from_u128(0x00000323_0000_0000_c000_000000000046);

pub fn spawn(groups: Vec<Group>, callback: EventCallback) -> Result<AudioService> {
    let (sender, receiver) = crossbeam_channel::unbounded();
    thread::Builder::new()
        .name("windows-audio".into())
        .spawn(move || unsafe {
            if let Err(err) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
                error!(error = %err, "cannot initialize COM");
                return;
            }
            match Worker::new(groups, callback) {
                Ok(mut worker) => worker.run(receiver),
                Err(err) => error!(error = %err, "cannot start audio worker"),
            }
            CoUninitialize();
        })
        .map_err(|err| AudioError::Unavailable(err.to_string()))?;
    Ok(AudioService { sender })
}

struct Worker {
    enumerator: IMMDeviceEnumerator,
    endpoint: IMMDevice,
    manager: IAudioSessionManager2,
    endpoint_volume: IAudioEndpointVolume,
    endpoint_meter: IAudioMeterInformation,
    endpoint_id: String,
    sessions: HashMap<String, LiveSession>,
    groups: Vec<Group>,
    icons: IconCache,
    callback: EventCallback,
    metering_active: bool,
    started: Instant,
}
struct LiveSession {
    simple: ISimpleAudioVolume,
    meter: Option<IAudioMeterInformation>,
    data: AudioSession,
}

impl Worker {
    unsafe fn new(groups: Vec<Group>, callback: EventCallback) -> Result<Self> {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(winerr)?;
        let endpoint = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(winerr)?;
        let manager = endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        let endpoint_volume = endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        let endpoint_meter = endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        let endpoint_id = take_pwstr(endpoint.GetId().map_err(winerr)?);
        let mut worker = Self {
            enumerator,
            endpoint,
            manager,
            endpoint_volume,
            endpoint_meter,
            endpoint_id,
            sessions: HashMap::new(),
            groups,
            icons: IconCache::default(),
            callback,
            metering_active: false,
            started: Instant::now(),
        };
        worker.enumerate(false)?;
        Ok(worker)
    }

    unsafe fn enumerate(&mut self, emit: bool) -> Result<()> {
        let enumerator = self.manager.GetSessionEnumerator().map_err(winerr)?;
        // This mandatory call enables future session-created notifications according to Microsoft.
        // https://learn.microsoft.com/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessionmanager2-registersessionnotification
        let count = enumerator.GetCount().map_err(winerr)?;
        let mut seen = HashSet::new();
        for index in 0..count {
            match enumerator
                .GetSession(index)
                .and_then(|control| self.read_session(control))
            {
                Ok(mut session) => {
                    let id = session.data.live_id.clone();
                    seen.insert(id.clone());
                    if let Some(existing) = self.sessions.get_mut(&id) {
                        if existing.data != session.data {
                            existing.data = session.data.clone();
                            if emit {
                                (self.callback)(AudioEvent::SessionUpdated(session.data));
                            }
                        }
                    } else {
                        if emit {
                            self.apply_policy(&mut session);
                            (self.callback)(AudioEvent::SessionAdded(session.data.clone()));
                        }
                        self.sessions.insert(id, session);
                    }
                }
                Err(err) => warn!(index, error = %err, "cannot read audio session"),
            }
        }
        let removed: Vec<_> = self
            .sessions
            .keys()
            .filter(|id| !seen.contains(*id))
            .cloned()
            .collect();
        for live_id in removed {
            self.sessions.remove(&live_id);
            if emit {
                (self.callback)(AudioEvent::SessionRemoved { live_id });
            }
        }
        Ok(())
    }

    unsafe fn read_session(
        &self,
        control: IAudioSessionControl,
    ) -> windows::core::Result<LiveSession> {
        let control2: IAudioSessionControl2 = control.cast()?;
        let instance = take_pwstr(control2.GetSessionInstanceIdentifier()?);
        let live_id = format!("{}::{instance}", self.endpoint_id);
        let state = map_state(control.GetState()?);
        let is_system_sounds = control2.IsSystemSoundsSession() == S_OK;
        // AUDCLNT_S_NO_SINGLE_PROCESS is a success HRESULT. windows-rs returns its output when available.
        let process_id = if is_system_sounds {
            None
        } else {
            control2.GetProcessId().ok()
        };
        let metadata = process_id.map(metadata_for_process);
        let executable_path = metadata
            .as_ref()
            .and_then(|item| item.executable_path.clone());
        let executable_name = metadata
            .as_ref()
            .and_then(|item| item.executable_name.clone());
        let app_key = audio_policy::resolve_app_key(
            metadata.as_ref().and_then(|item| item.aumid.as_deref()),
            executable_path.as_deref(),
            executable_name.as_deref(),
        );
        let display = take_pwstr(control.GetDisplayName()?).trim().to_owned();
        let display_name = if display.is_empty() {
            executable_name
                .as_deref()
                .and_then(|name| Path::new(name).file_stem())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    if is_system_sounds {
                        "System Sounds".into()
                    } else {
                        app_key.clone()
                    }
                })
        } else {
            display
        };
        let simple: ISimpleAudioVolume = control.cast()?;
        let volume = simple.GetMasterVolume()?;
        let muted = simple.GetMute()?.as_bool();
        let meter = control.cast().ok();
        let group_id =
            audio_policy::group_for(&app_key, &self.groups).map(|group| group.id.clone());
        let icon = executable_path
            .as_ref()
            .and_then(|path| self.icons.icon_for_path(Path::new(path)));
        Ok(LiveSession {
            simple,
            meter,
            data: AudioSession {
                live_id,
                app_key,
                display_name,
                executable_name,
                executable_path,
                process_id,
                icon,
                volume,
                muted,
                state,
                is_system_sounds,
                controllable: true,
                group_id,
            },
        })
    }

    unsafe fn apply_policy(&self, session: &mut LiveSession) {
        let policy = audio_policy::policy_for_new_session(audio_policy::group_for(
            &session.data.app_key,
            &self.groups,
        ));
        if let Some(muted) = policy.muted {
            if session.simple.SetMute(muted, ptr::null()).is_ok() {
                session.data.muted = muted;
            }
        }
        if let Some(volume) = policy.volume {
            if session.simple.SetMasterVolume(volume, ptr::null()).is_ok() {
                session.data.volume = volume;
            }
        }
    }

    unsafe fn run(&mut self, receiver: crossbeam_channel::Receiver<Command>) {
        let mut next_reconcile = Instant::now();
        let mut next_meter = Instant::now();
        loop {
            let deadline = next_reconcile.min(next_meter);
            match receiver.recv_deadline(deadline) {
                Ok(Command::Snapshot(reply)) => {
                    let _ = self.enumerate(true);
                    let _ = reply.send(self.snapshot());
                }
                Ok(Command::SetSessionVolume {
                    live_id,
                    volume,
                    reply,
                }) => {
                    let result = self.set_volume(&live_id, volume);
                    let _ = reply.send(result);
                }
                Ok(Command::SetSessionMute {
                    live_id,
                    muted,
                    reply,
                }) => {
                    let result = self.set_mute(&live_id, muted);
                    let _ = reply.send(result);
                }
                Ok(Command::SetMasterVolume { volume, reply }) => {
                    let _ = reply.send(
                        self.endpoint_volume
                            .SetMasterVolumeLevelScalar(
                                audio_policy::clamp_scalar(volume),
                                ptr::null(),
                            )
                            .map_err(winerr),
                    );
                }
                Ok(Command::SetMasterMute { muted, reply }) => {
                    let _ = reply.send(
                        self.endpoint_volume
                            .SetMute(muted, ptr::null())
                            .map_err(winerr),
                    );
                }
                Ok(Command::SetMeteringActive { active, reply }) => {
                    self.metering_active = active;
                    let _ = reply.send(Ok(()));
                }
                Ok(Command::UpdateGroups { groups, reply }) => {
                    self.groups = groups;
                    let _ = reply.send(Ok(()));
                }
                Ok(Command::AdoptGit(cookie)) => self.adopt_git(cookie),
                Ok(Command::Reenumerate) => {
                    let _ = self.enumerate(true);
                }
                Ok(Command::RebuildEndpoint) => {
                    let _ = self.rebuild();
                }
                Ok(Command::Shutdown) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    break
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            }
            let now = Instant::now();
            if now >= next_meter {
                self.emit_peaks();
                next_meter = now
                    + if self.metering_active {
                        Duration::from_millis(50)
                    } else {
                        Duration::from_millis(250)
                    };
            }
            if now >= next_reconcile {
                let before = self.sessions.len();
                let _ = self.enumerate(true);
                if before != self.sessions.len() {
                    debug!(
                        before,
                        after = self.sessions.len(),
                        "reconciliation found session drift"
                    );
                }
                next_reconcile = now + Duration::from_secs(10);
            }
        }
    }

    unsafe fn snapshot(&self) -> Result<MixerSnapshot> {
        Ok(MixerSnapshot {
            master: MasterState {
                endpoint_id: self.endpoint_id.clone(),
                endpoint_name: self.endpoint_id.clone(),
                volume: self
                    .endpoint_volume
                    .GetMasterVolumeLevelScalar()
                    .map_err(winerr)?,
                muted: self.endpoint_volume.GetMute().map_err(winerr)?.as_bool(),
            },
            sessions: self.sessions.values().map(|s| s.data.clone()).collect(),
        })
    }
    unsafe fn set_volume(&mut self, id: &str, volume: f32) -> Result<()> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(AudioError::InvalidVolume);
        }
        let session = self.sessions.get_mut(id).ok_or(AudioError::SessionGone)?;
        if session.simple.SetMasterVolume(volume, ptr::null()).is_err() {
            session.data.controllable = false;
            (self.callback)(AudioEvent::SessionUpdated(session.data.clone()));
            return Err(AudioError::NotControllable);
        }
        session.data.volume = volume;
        (self.callback)(AudioEvent::SessionUpdated(session.data.clone()));
        Ok(())
    }
    unsafe fn set_mute(&mut self, id: &str, muted: bool) -> Result<()> {
        let session = self.sessions.get_mut(id).ok_or(AudioError::SessionGone)?;
        if session.simple.SetMute(muted, ptr::null()).is_err() {
            session.data.controllable = false;
            (self.callback)(AudioEvent::SessionUpdated(session.data.clone()));
            return Err(AudioError::NotControllable);
        }
        session.data.muted = muted;
        (self.callback)(AudioEvent::SessionUpdated(session.data.clone()));
        Ok(())
    }
    unsafe fn emit_peaks(&self) {
        (self.callback)(AudioEvent::Peaks(PeakBatch {
            timestamp_ms: self.started.elapsed().as_millis() as u64,
            master_peak: self.endpoint_meter.GetPeakValue().unwrap_or(0.0),
            sessions: self
                .sessions
                .values()
                .filter_map(|s| {
                    s.meter
                        .as_ref()
                        .and_then(|m| m.GetPeakValue().ok())
                        .map(|peak| SessionPeak {
                            live_id: s.data.live_id.clone(),
                            peak,
                        })
                })
                .collect(),
        }));
    }
    unsafe fn adopt_git(&mut self, cookie: u32) {
        let Ok(git): windows::core::Result<IGlobalInterfaceTable> =
            CoCreateInstance(&CLSID_STD_GLOBAL_INTERFACE_TABLE, None, CLSCTX_ALL)
        else {
            return;
        };
        let mut raw = ptr::null_mut();
        if git
            .GetInterfaceFromGlobal(cookie, &IAudioSessionControl::IID, &mut raw)
            .is_ok()
        {
            let control = IAudioSessionControl::from_raw(raw);
            if let Ok(mut session) = self.read_session(control) {
                self.apply_policy(&mut session);
                (self.callback)(AudioEvent::SessionAdded(session.data.clone()));
                self.sessions.insert(session.data.live_id.clone(), session);
            }
        }
        let _ = git.RevokeInterfaceFromGlobal(cookie);
    }
    unsafe fn rebuild(&mut self) -> Result<()> {
        self.sessions.clear();
        self.endpoint = self
            .enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(winerr)?;
        self.manager = self.endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        self.endpoint_volume = self.endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        self.endpoint_meter = self.endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        self.endpoint_id = take_pwstr(self.endpoint.GetId().map_err(winerr)?);
        self.enumerate(false)?;
        (self.callback)(AudioEvent::EndpointChanged(self.snapshot()?));
        Ok(())
    }
}

fn map_state(state: AudioSessionState) -> SessionState {
    if state == AudioSessionStateActive {
        SessionState::Active
    } else if state == AudioSessionStateInactive {
        SessionState::Inactive
    } else {
        SessionState::Expired
    }
}
fn winerr(err: windows::core::Error) -> AudioError {
    AudioError::Unavailable(err.to_string())
}
unsafe fn take_pwstr(value: PWSTR) -> String {
    if value.is_null() {
        return String::new();
    }
    let text = value.to_string().unwrap_or_default();
    CoTaskMemFree(Some(value.as_ptr().cast()));
    text
}
