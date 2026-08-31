use super::*;
use app_icons::{metadata_for_process, IconCache};
use std::{
    path::Path,
    ptr, thread,
    time::{Duration, Instant},
};
use tracing::{debug, error, trace, warn};
use windows::{
    core::{implement, Interface, Ref, BOOL, GUID, PCWSTR, PWSTR},
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Foundation::{PROPERTYKEY, S_OK},
        Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation},
        Media::Audio::{
            eConsole, eRender, AudioSessionDisconnectReason, AudioSessionState,
            AudioSessionStateActive, AudioSessionStateExpired, AudioSessionStateInactive,
            EDataFlow, ERole, IAudioSessionControl, IAudioSessionControl2, IAudioSessionEvents,
            IAudioSessionEvents_Impl, IAudioSessionManager2, IAudioSessionNotification,
            IAudioSessionNotification_Impl, IMMDevice, IMMDeviceEnumerator, IMMNotificationClient,
            IMMNotificationClient_Impl, ISimpleAudioVolume, MMDeviceEnumerator,
            AUDCLNT_E_DEVICE_INVALIDATED, DEVICE_STATE,
        },
        System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc},
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IGlobalInterfaceTable,
            CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
        },
    },
};

const CLSID_STD_GLOBAL_INTERFACE_TABLE: GUID =
    GUID::from_u128(0x00000323_0000_0000_c000_000000000046);

pub fn spawn(
    applications: Vec<Application>,
    groups: Vec<Group>,
    callback: EventCallback,
) -> Result<AudioService> {
    let (sender, receiver) = crossbeam_channel::unbounded();
    let worker_sender = sender.clone();
    thread::Builder::new()
        .name("windows-audio".into())
        .spawn(move || unsafe {
            exempt_audio_thread_from_throttling();
            if let Err(err) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
                error!(error = %err, "cannot initialize COM");
                return;
            }
            match Worker::new(worker_sender, applications, groups, callback) {
                Ok(mut worker) => {
                    worker.run(receiver);
                    worker.shutdown();
                }
                Err(err) => error!(error = %err, "cannot start audio worker"),
            }
            CoUninitialize();
        })
        .map_err(|err| AudioError::Unavailable(err.to_string()))?;
    Ok(AudioService { sender })
}

struct Worker {
    sender: crossbeam_channel::Sender<Command>,
    enumerator: IMMDeviceEnumerator,
    endpoint: IMMDevice,
    manager: IAudioSessionManager2,
    endpoint_volume: IAudioEndpointVolume,
    endpoint_meter: IAudioMeterInformation,
    endpoint_id: String,
    endpoint_name: String,
    endpoint_sink: IMMNotificationClient,
    manager_sink: IAudioSessionNotification,
    sessions: HashMap<String, LiveSession>,
    applications: HashMap<String, Application>,
    policy_applied: HashSet<String>,
    groups: Vec<Group>,
    icons: IconCache,
    callback: EventCallback,
    metering_active: bool,
    started: Instant,
    scene_ramp: Option<SceneRamp>,
}

struct SceneRamp {
    started: Instant,
    next_tick: Instant,
    duration: Duration,
    master_target: Option<(f32, f32)>,
    targets: HashMap<String, (f32, f32)>,
    mutes: Vec<(String, bool)>,
    reply: oneshot::Sender<Result<()>>,
}

struct LiveSession {
    control: IAudioSessionControl,
    simple: ISimpleAudioVolume,
    meter: Option<IAudioMeterInformation>,
    sink: IAudioSessionEvents,
    data: AudioSession,
    application: Application,
}

impl Worker {
    unsafe fn new(
        sender: crossbeam_channel::Sender<Command>,
        applications: Vec<Application>,
        mut groups: Vec<Group>,
        callback: EventCallback,
    ) -> Result<Self> {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(winerr)?;
        let endpoint_sink: IMMNotificationClient = EndpointSink {
            sender: sender.clone(),
        }
        .into();
        enumerator
            .RegisterEndpointNotificationCallback(&endpoint_sink)
            .map_err(winerr)?;

        let endpoint = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(winerr)?;
        let manager: IAudioSessionManager2 = endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        let endpoint_volume = endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        let endpoint_meter = endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        let endpoint_id = take_pwstr(endpoint.GetId().map_err(winerr)?);
        let endpoint_name =
            endpoint_friendly_name(&endpoint).unwrap_or_else(|_| endpoint_id.clone());
        let manager_sink: IAudioSessionNotification = ManagerSink {
            sender: sender.clone(),
        }
        .into();
        manager
            .RegisterSessionNotification(&manager_sink)
            .map_err(winerr)?;

        let mut worker = Self {
            sender,
            enumerator,
            endpoint,
            manager,
            endpoint_volume,
            endpoint_meter,
            endpoint_id,
            endpoint_name,
            endpoint_sink,
            manager_sink,
            sessions: HashMap::new(),
            applications: deduplicate_applications(applications, &mut groups),
            policy_applied: HashSet::new(),
            groups,
            icons: IconCache::default(),
            callback,
            metering_active: false,
            started: Instant::now(),
            scene_ramp: None,
        };
        // Priming runs after the worker owns both sinks. If it fails, `shutdown`
        // can unregister them; returning early instead would leave Windows
        // holding callbacks into interfaces that are about to drop.
        if let Err(error) = worker.prime_and_enumerate(false) {
            worker.shutdown();
            return Err(error);
        }
        Ok(worker)
    }

    unsafe fn prime_and_enumerate(&mut self, emit: bool) -> Result<()> {
        let enumerator = self.manager.GetSessionEnumerator().map_err(winerr)?;
        // GetCount is mandatory after RegisterSessionNotification. Until a client retrieves the
        // existing list, Windows discards create-session notifications.
        // https://learn.microsoft.com/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessionmanager2-registersessionnotification
        let count = enumerator.GetCount().map_err(winerr)?;
        self.adopt_enumerator(&enumerator, count, emit)
    }

    unsafe fn reconcile(&mut self) -> Result<()> {
        let enumerator = self.manager.GetSessionEnumerator().map_err(winerr)?;
        let count = enumerator.GetCount().map_err(winerr)?;
        let before: HashSet<_> = self.sessions.keys().cloned().collect();
        self.adopt_enumerator(&enumerator, count, true)?;
        let after: HashSet<_> = self.sessions.keys().cloned().collect();
        if before != after {
            debug!(
                before = before.len(),
                after = after.len(),
                "reconciliation found session drift"
            );
        }
        Ok(())
    }

    unsafe fn adopt_enumerator(
        &mut self,
        enumerator: &windows::Win32::Media::Audio::IAudioSessionEnumerator,
        count: i32,
        emit: bool,
    ) -> Result<()> {
        let mut seen = HashSet::new();
        for index in 0..count {
            match enumerator.GetSession(index) {
                Ok(control) => match self.make_session(control) {
                    Ok(session) => {
                        seen.insert(session.data.live_id.clone());
                        self.adopt(session, emit);
                    }
                    Err(err) => warn!(index, error = %err, "cannot read audio session"),
                },
                Err(err) => warn!(index, error = %err, "cannot enumerate audio session"),
            }
        }
        let removed: Vec<_> = self
            .sessions
            .keys()
            .filter(|id| !seen.contains(*id))
            .cloned()
            .collect();
        for live_id in removed {
            self.remove_session(&live_id, emit);
        }
        Ok(())
    }

    unsafe fn make_session(
        &self,
        control: IAudioSessionControl,
    ) -> windows::core::Result<LiveSession> {
        let control2: IAudioSessionControl2 = control.cast()?;
        let instance = take_pwstr(control2.GetSessionInstanceIdentifier()?);
        let live_id = format!("{}::{instance}", self.endpoint_id);
        let state = map_state(control.GetState()?);
        let is_system_sounds = control2.IsSystemSoundsSession() == S_OK;
        // AUDCLNT_S_NO_SINGLE_PROCESS is successful but does not identify one owner process.
        let process_id = if is_system_sounds {
            None
        } else {
            control2.GetProcessId().ok().filter(|pid| *pid != 0)
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
        let display = resolve_indirect_name(&display).unwrap_or(display);
        let display_name = if display.is_empty() || (is_system_sounds && display.starts_with('@')) {
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
        let meter = control.cast().ok();
        let group_id =
            audio_policy::group_for(&app_key, &self.groups).map(|group| group.id.clone());
        let icon = executable_path
            .as_ref()
            .and_then(|path| self.icons.icon_for_path(Path::new(path)));
        let identity_kind = if metadata
            .as_ref()
            .and_then(|item| item.aumid.as_ref())
            .is_some()
        {
            IdentityKind::Aumid
        } else if executable_path.is_some() {
            IdentityKind::Path
        } else {
            IdentityKind::Filename
        };
        let mut application =
            Application::offline(app_key.clone(), identity_kind, display_name.clone());
        application.executable_name = executable_name;
        application.executable_path = executable_path;
        application.icon = icon;
        application.group_id = group_id;
        application.is_system_sounds = is_system_sounds;
        // Every fallible read happens before the sink is registered. Registering
        // first would leak the sink into the session control whenever one of
        // these calls failed, because the early return has nothing to unregister
        // it with.
        let volume = simple.GetMasterVolume()?;
        let muted = simple.GetMute()?.as_bool();

        let sink: IAudioSessionEvents = SessionSink {
            live_id: live_id.clone(),
            sender: self.sender.clone(),
        }
        .into();
        control.RegisterAudioSessionNotification(&sink)?;

        Ok(LiveSession {
            data: AudioSession {
                live_id,
                app_key,
                display_name,
                process_id,
                volume,
                muted,
                state,
                controllable: true,
                peak: 0.0,
            },
            application,
            control,
            simple,
            meter,
            sink,
        })
    }

    unsafe fn adopt(&mut self, mut session: LiveSession, emit: bool) {
        let live_id = session.data.live_id.clone();
        if session.data.state == SessionState::Expired || self.sessions.contains_key(&live_id) {
            let _ = session
                .control
                .UnregisterAudioSessionNotification(&session.sink);
            return;
        }
        let key = session.data.app_key.clone();
        let upgraded_from = self.merge_key_for(&session.application);
        let existed = self.applications.contains_key(&key) || upgraded_from.is_some();
        if let Some(old_key) = upgraded_from.as_ref() {
            let persisted = self.applications.remove(old_key).expect("merge key exists");
            let merged = merge_discovered_application(persisted, &session.application);
            replace_group_key(&mut self.groups, old_key, &key);
            self.applications.insert(key.clone(), merged);
        } else {
            self.applications
                .entry(key.clone())
                .or_insert_with(|| session.application.clone());
        }
        if emit {
            self.apply_policy(&mut session);
        }
        self.sessions.insert(live_id, session);
        if emit {
            if let Some(old_key) = upgraded_from {
                (self.callback)(AudioEvent::ApplicationRemoved { app_key: old_key });
            }
            if let Some(app) = self.aggregate(&key) {
                (self.callback)(if existed {
                    AudioEvent::ApplicationUpdated(app)
                } else {
                    AudioEvent::ApplicationAdded(app)
                });
            }
        }
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

    unsafe fn apply_policy(&mut self, session: &mut LiveSession) {
        let key = session.data.app_key.clone();
        if let Some(app) = self.applications.get(&key).filter(|app| app.remembered) {
            if session.simple.SetMute(app.muted, ptr::null()).is_ok() {
                session.data.muted = app.muted;
            }
            if session
                .simple
                .SetMasterVolume(app.volume, ptr::null())
                .is_ok()
            {
                session.data.volume = app.volume;
            }
            self.policy_applied.insert(key);
            return;
        }
        let policy =
            audio_policy::policy_for_new_session(audio_policy::group_for(&key, &self.groups));
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

    fn aggregate(&self, key: &str) -> Option<Application> {
        let mut app = self.applications.get(key)?.clone();
        app.sessions = self
            .sessions
            .values()
            .filter(|s| s.data.app_key == key)
            .map(|s| s.data.clone())
            .collect();
        app.running = !app.sessions.is_empty();
        app.controllable = app.sessions.iter().any(|s| s.controllable);
        app.peak = app.sessions.iter().map(|s| s.peak).fold(0.0, f32::max);
        if let Some(first) = app.sessions.first() {
            let agrees = app
                .sessions
                .iter()
                .all(|s| (s.volume - first.volume).abs() <= 0.0001 && s.muted == first.muted);

            if agrees {
                // The live sessions are the truth whenever they agree, even
                // after a policy has been applied. Reporting the stored policy
                // instead would leave the row showing a stale number after
                // anything else moved that application: the Windows mixer, a
                // game's own volume control, or our CLI.
                app.volume = first.volume;
                app.muted = first.muted;
                app.mixed = false;
            } else {
                // They disagree. Before a policy exists there is no single true
                // level, so the row says so rather than inventing an average.
                // Once one exists it is what the user asked for, and the next
                // fan-out resynchronizes the stragglers.
                app.mixed = !self.policy_applied.contains(key);
            }
        }
        Some(app)
    }

    unsafe fn run(&mut self, receiver: crossbeam_channel::Receiver<Command>) {
        let mut next_reconcile = Instant::now() + Duration::from_secs(10);
        let mut next_meter = Instant::now();
        loop {
            let next_ramp = self
                .scene_ramp
                .as_ref()
                .map_or(next_reconcile, |ramp| ramp.next_tick);
            let deadline = next_reconcile.min(next_meter).min(next_ramp);
            match receiver.recv_deadline(deadline) {
                Ok(Command::Snapshot(reply)) => {
                    let _ = reply.send(self.snapshot());
                }
                Ok(Command::SetAppVolume {
                    app_key,
                    volume,
                    reply,
                }) => {
                    let result = self.set_app_volume(&app_key, volume);
                    let _ = reply.send(result);
                }
                Ok(Command::SetAppMute {
                    app_key,
                    muted,
                    reply,
                }) => {
                    let result = self.set_app_mute(&app_key, muted);
                    let _ = reply.send(result);
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
                    let result = self
                        .endpoint_volume
                        .SetMasterVolumeLevelScalar(audio_policy::clamp_scalar(volume), ptr::null())
                        .map_err(|err| self.map_endpoint_error(err));
                    if result.is_ok() {
                        if let Ok(master) = self.master() {
                            (self.callback)(AudioEvent::MasterUpdated(master));
                        }
                    }
                    let _ = reply.send(result);
                }
                Ok(Command::SetMasterMute { muted, reply }) => {
                    let result = self
                        .endpoint_volume
                        .SetMute(muted, ptr::null())
                        .map_err(|err| self.map_endpoint_error(err));
                    if result.is_ok() {
                        if let Ok(master) = self.master() {
                            (self.callback)(AudioEvent::MasterUpdated(master));
                        }
                    }
                    let _ = reply.send(result);
                }
                Ok(Command::SetMeteringActive { active, reply }) => {
                    self.metering_active = active;
                    let _ = reply.send(Ok(()));
                }
                Ok(Command::UpdateSettings {
                    applications,
                    groups,
                    reply,
                }) => {
                    let mut groups = groups;
                    self.applications = deduplicate_applications(applications, &mut groups);
                    self.groups = groups;
                    let _ = reply.send(Ok(()));
                }
                Ok(Command::ListOutputDevices(reply)) => {
                    let result = self.list_output_devices();
                    let _ = reply.send(result);
                }
                Ok(Command::SetDefaultOutput { device_id, reply }) => {
                    let result = self.set_default_output(&device_id);
                    let _ = reply.send(result);
                }
                Ok(Command::ApplyScene { scene, reply }) => {
                    self.start_scene(scene, reply);
                }
                Ok(Command::EmitEvent { event, reply }) => {
                    (self.callback)(event);
                    let _ = reply.send(Ok(()));
                }
                Ok(Command::AdoptGit(cookie)) => self.adopt_git(cookie),
                Ok(Command::Reenumerate) => {
                    if let Err(err) = self.reconcile() {
                        warn!(error = %err, "callback fallback enumeration failed");
                    }
                }
                Ok(Command::SessionChanged(live_id)) => self.refresh_session(&live_id),
                Ok(Command::SessionRemoved(live_id)) => self.remove_session(&live_id, true),
                Ok(Command::RebuildEndpoint) => {
                    if let Err(err) = self.rebuild() {
                        error!(error = %err, "audio endpoint rebuild failed");
                    }
                }
                Ok(Command::Shutdown) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    break;
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            }
            let now = Instant::now();
            self.tick_scene(now);
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
                if let Err(err) = self.reconcile() {
                    warn!(error = %err, "audio reconciliation failed");
                }
                next_reconcile = now + Duration::from_secs(10);
            }
        }
    }

    unsafe fn refresh_session(&mut self, live_id: &str) {
        let Some(session) = self.sessions.get_mut(live_id) else {
            return;
        };
        match session.control.GetState() {
            Ok(state) if state == AudioSessionStateExpired => {
                self.remove_session(live_id, true);
                return;
            }
            Ok(state) => session.data.state = map_state(state),
            Err(err) if err.code() == AUDCLNT_E_DEVICE_INVALIDATED => {
                let _ = self.sender.send(Command::RebuildEndpoint);
                return;
            }
            Err(err) => {
                warn!(live_id, error = %err, "cannot refresh session state");
                return;
            }
        }
        if let Ok(volume) = session.simple.GetMasterVolume() {
            session.data.volume = volume;
        }
        if let Ok(muted) = session.simple.GetMute() {
            session.data.muted = muted.as_bool();
        }
        if let Ok(name) = session.control.GetDisplayName() {
            let name = take_pwstr(name);
            if !name.trim().is_empty() {
                session.data.display_name = name;
            }
        }
        let key = session.data.app_key.clone();
        if let Some(app) = self.aggregate(&key) {
            (self.callback)(AudioEvent::ApplicationUpdated(app));
        }
    }

    unsafe fn remove_session(&mut self, live_id: &str, emit: bool) {
        if let Some(session) = self.sessions.remove(live_id) {
            let _ = session
                .control
                .UnregisterAudioSessionNotification(&session.sink);
            if emit {
                if let Some(app) = self.aggregate(&session.data.app_key) {
                    (self.callback)(AudioEvent::ApplicationUpdated(app));
                }
            }
        }
    }

    unsafe fn snapshot(&self) -> Result<MixerSnapshot> {
        let mut applications: Vec<_> = self
            .applications
            .keys()
            .filter_map(|key| self.aggregate(key))
            .collect();
        applications.sort_by_key(|app| (!app.pinned, app.sort_order));
        Ok(MixerSnapshot {
            master: self.master()?,
            applications,
        })
    }

    unsafe fn master(&self) -> Result<MasterState> {
        Ok(MasterState {
            endpoint_id: self.endpoint_id.clone(),
            endpoint_name: self.endpoint_name.clone(),
            volume: self
                .endpoint_volume
                .GetMasterVolumeLevelScalar()
                .map_err(|err| self.map_endpoint_error(err))?,
            muted: self
                .endpoint_volume
                .GetMute()
                .map_err(|err| self.map_endpoint_error(err))?
                .as_bool(),
            peak: self.endpoint_meter.GetPeakValue().unwrap_or(0.0),
        })
    }

    /// Classifies a failed session control call.
    ///
    /// A lost or replaced endpoint is not an uncontrollable application: marking
    /// the session permanently uncontrollable would disable its slider until the
    /// user restarted NovaMixer. Device invalidation queues a rebuild instead,
    /// and only a genuine control rejection sets `controllable = false`.
    unsafe fn map_session_error(&mut self, id: &str, error: windows_core::Error) -> AudioError {
        if error.code() == AUDCLNT_E_DEVICE_INVALIDATED {
            let _ = self.sender.send(Command::RebuildEndpoint);
            return AudioError::Unavailable("audio endpoint was invalidated".into());
        }
        if let Some(session) = self.sessions.get_mut(id) {
            session.data.controllable = false;
            let key = session.data.app_key.clone();
            if let Some(app) = self.aggregate(&key) {
                (self.callback)(AudioEvent::ApplicationUpdated(app));
            }
        }
        AudioError::NotControllable
    }

    unsafe fn start_scene(&mut self, scene: Scene, reply: oneshot::Sender<Result<()>>) {
        if let Some(previous) = self.scene_ramp.take() {
            let _ = previous.reply.send(Ok(()));
        }
        let now = Instant::now();
        let master_target = scene.master_volume.and_then(|target| {
            self.endpoint_volume
                .GetMasterVolumeLevelScalar()
                .ok()
                .map(|start| (start, audio_policy::clamp_scalar(target)))
        });
        let targets = scene
            .entries
            .iter()
            .filter_map(|entry| {
                entry.volume.and_then(|target| {
                    self.aggregate(&entry.app_key).map(|app| {
                        (
                            entry.app_key.clone(),
                            (app.volume, audio_policy::clamp_scalar(target)),
                        )
                    })
                })
            })
            .collect();
        let mutes = scene
            .entries
            .into_iter()
            .filter_map(|entry| entry.muted.map(|muted| (entry.app_key, muted)))
            .collect();
        self.scene_ramp = Some(SceneRamp {
            started: now,
            next_tick: now,
            duration: Duration::from_millis(scene.fade_ms as u64),
            master_target,
            targets,
            mutes,
            reply,
        });
        self.tick_scene(now);
    }

    unsafe fn tick_scene(&mut self, now: Instant) {
        let Some(ramp) = self.scene_ramp.as_ref() else {
            return;
        };
        if now < ramp.next_tick {
            return;
        }
        let progress = if ramp.duration.is_zero() {
            1.0
        } else {
            (now.duration_since(ramp.started).as_secs_f32() / ramp.duration.as_secs_f32()).min(1.0)
        };
        let master_value = ramp
            .master_target
            .map(|(start, target)| start + (target - start) * progress);
        let values: Vec<_> = ramp
            .targets
            .iter()
            .map(|(key, (start, target))| (key.clone(), start + (target - start) * progress))
            .collect();
        if let Some(value) = master_value {
            let _ = self
                .endpoint_volume
                .SetMasterVolumeLevelScalar(value, ptr::null());
        }
        for (key, value) in values {
            let _ = self.set_app_volume_internal(&key, value);
        }
        if progress >= 1.0 {
            let ramp = self.scene_ramp.take().expect("ramp exists");
            for (key, muted) in ramp.mutes {
                let _ = self.set_app_mute_internal(&key, muted);
            }
            let _ = ramp.reply.send(Ok(()));
        } else if let Some(ramp) = self.scene_ramp.as_mut() {
            ramp.next_tick = now + Duration::from_millis(16);
        }
    }

    unsafe fn set_app_volume(&mut self, key: &str, volume: f32) -> Result<()> {
        if let Some(ramp) = self.scene_ramp.as_mut() {
            ramp.targets.remove(key);
        }
        self.set_app_volume_internal(key, volume)
    }

    unsafe fn set_app_volume_internal(&mut self, key: &str, volume: f32) -> Result<()> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(AudioError::InvalidVolume);
        }
        if !self.applications.contains_key(key) {
            return Err(AudioError::AppUnknown);
        }
        self.policy_applied.insert(key.to_owned());
        self.applications.get_mut(key).unwrap().volume = volume;
        let ids: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.data.app_key == key)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(session) = self.sessions.get_mut(&id) {
                if session.simple.SetMasterVolume(volume, ptr::null()).is_ok() {
                    session.data.volume = volume;
                }
            }
        }
        if let Some(app) = self.aggregate(key) {
            (self.callback)(AudioEvent::ApplicationUpdated(app));
        }
        Ok(())
    }
    unsafe fn set_app_mute(&mut self, key: &str, muted: bool) -> Result<()> {
        if let Some(ramp) = self.scene_ramp.as_mut() {
            ramp.targets.remove(key);
            ramp.mutes.retain(|(app_key, _)| app_key != key);
        }
        self.set_app_mute_internal(key, muted)
    }

    unsafe fn set_app_mute_internal(&mut self, key: &str, muted: bool) -> Result<()> {
        if !self.applications.contains_key(key) {
            return Err(AudioError::AppUnknown);
        }
        self.policy_applied.insert(key.to_owned());
        self.applications.get_mut(key).unwrap().muted = muted;
        let ids: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, session)| session.data.app_key == key)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            let result = self.sessions.get_mut(&id).map(|session| {
                session
                    .simple
                    .SetMute(muted, ptr::null())
                    .map(|()| session.data.muted = muted)
            });
            if let Some(Err(error)) = result {
                let _ = self.map_session_error(&id, error);
            }
        }
        if let Some(app) = self.aggregate(key) {
            (self.callback)(AudioEvent::ApplicationUpdated(app));
        }
        Ok(())
    }

    unsafe fn set_volume(&mut self, id: &str, volume: f32) -> Result<()> {
        if let Some(key) = self
            .sessions
            .get(id)
            .map(|session| session.data.app_key.clone())
        {
            if let Some(ramp) = self.scene_ramp.as_mut() {
                ramp.targets.remove(&key);
            }
        }
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(AudioError::InvalidVolume);
        }
        let session = self.sessions.get_mut(id).ok_or(AudioError::SessionGone)?;
        if let Err(error) = session.simple.SetMasterVolume(volume, ptr::null()) {
            return Err(self.map_session_error(id, error));
        }
        session.data.volume = volume;
        let key = session.data.app_key.clone();
        if let Some(app) = self.aggregate(&key) {
            (self.callback)(AudioEvent::ApplicationUpdated(app));
        }
        Ok(())
    }

    unsafe fn set_mute(&mut self, id: &str, muted: bool) -> Result<()> {
        if let Some(key) = self
            .sessions
            .get(id)
            .map(|session| session.data.app_key.clone())
        {
            if let Some(ramp) = self.scene_ramp.as_mut() {
                ramp.targets.remove(&key);
                ramp.mutes.retain(|(app_key, _)| app_key != &key);
            }
        }
        let session = self.sessions.get_mut(id).ok_or(AudioError::SessionGone)?;
        if let Err(error) = session.simple.SetMute(muted, ptr::null()) {
            return Err(self.map_session_error(id, error));
        }
        session.data.muted = muted;
        let key = session.data.app_key.clone();
        if let Some(app) = self.aggregate(&key) {
            (self.callback)(AudioEvent::ApplicationUpdated(app));
        }
        Ok(())
    }

    unsafe fn emit_peaks(&self) {
        let master_peak = self.endpoint_meter.GetPeakValue().unwrap_or(0.0);
        let applications = self
            .applications
            .keys()
            .filter_map(|key| {
                let sessions: Vec<_> = self
                    .sessions
                    .values()
                    .filter(|s| s.data.app_key == *key)
                    .filter_map(|s| {
                        s.meter
                            .as_ref()
                            .and_then(|m| m.GetPeakValue().ok())
                            .map(|peak| SessionPeak {
                                live_id: s.data.live_id.clone(),
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
        (self.callback)(AudioEvent::Peaks(PeakBatch {
            timestamp_ms: self.started.elapsed().as_millis() as u64,
            master_peak,
            applications,
        }));
    }

    unsafe fn adopt_git(&mut self, cookie: u32) {
        let git: windows::core::Result<IGlobalInterfaceTable> =
            CoCreateInstance(&CLSID_STD_GLOBAL_INTERFACE_TABLE, None, CLSCTX_ALL);
        let Ok(git) = git else {
            let _ = self.sender.send(Command::Reenumerate);
            return;
        };
        let mut raw = ptr::null_mut();
        let resolved = git.GetInterfaceFromGlobal(cookie, &IAudioSessionControl::IID, &mut raw);
        let _ = git.RevokeInterfaceFromGlobal(cookie);
        match resolved {
            Ok(()) => {
                let control = IAudioSessionControl::from_raw(raw);
                match self.make_session(control) {
                    Ok(session) => {
                        trace!(live_id = %session.data.live_id, "OnSessionCreated reached audio worker");
                        self.adopt(session, true);
                    }
                    Err(err) => warn!(error = %err, "cannot adopt callback-created session"),
                }
            }
            Err(err) => {
                warn!(error = %err, "cannot resolve session GIT cookie");
                let _ = self.sender.send(Command::Reenumerate);
            }
        }
    }

    unsafe fn rebuild(&mut self) -> Result<()> {
        self.unregister_sessions();
        self.manager
            .UnregisterSessionNotification(&self.manager_sink)
            .map_err(winerr)?;

        self.endpoint = self
            .enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(winerr)?;
        self.manager = self.endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        self.endpoint_volume = self.endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        self.endpoint_meter = self.endpoint.Activate(CLSCTX_ALL, None).map_err(winerr)?;
        self.endpoint_id = take_pwstr(self.endpoint.GetId().map_err(winerr)?);
        self.endpoint_name =
            endpoint_friendly_name(&self.endpoint).unwrap_or_else(|_| self.endpoint_id.clone());
        self.manager
            .RegisterSessionNotification(&self.manager_sink)
            .map_err(winerr)?;
        self.prime_and_enumerate(false)?;
        (self.callback)(AudioEvent::EndpointChanged(self.snapshot()?));
        Ok(())
    }

    unsafe fn unregister_sessions(&mut self) {
        for (_, session) in self.sessions.drain() {
            let _ = session
                .control
                .UnregisterAudioSessionNotification(&session.sink);
        }
    }

    unsafe fn shutdown(&mut self) {
        self.metering_active = false;
        self.unregister_sessions();
        let _ = self
            .manager
            .UnregisterSessionNotification(&self.manager_sink);
        let _ = self
            .enumerator
            .UnregisterEndpointNotificationCallback(&self.endpoint_sink);
    }

    unsafe fn list_output_devices(&self) -> Result<Vec<AudioDevice>> {
        let collection = self
            .enumerator
            .EnumAudioEndpoints(eRender, windows::Win32::Media::Audio::DEVICE_STATE_ACTIVE)
            .map_err(winerr)?;
        let count = collection.GetCount().map_err(winerr)?;
        let mut devices = Vec::new();
        for index in 0..count {
            let device = collection.Item(index).map_err(winerr)?;
            let id = take_pwstr(device.GetId().map_err(winerr)?);
            devices.push(AudioDevice {
                name: endpoint_friendly_name(&device).unwrap_or_else(|_| id.clone()),
                is_default: id == self.endpoint_id,
                id,
            });
        }
        Ok(devices)
    }
    unsafe fn set_default_output(&mut self, _device_id: &str) -> Result<()> {
        Err(AudioError::Unsupported(
            "Windows policy configuration interface is unavailable".into(),
        ))
    }

    fn map_endpoint_error(&self, err: windows::core::Error) -> AudioError {
        if err.code() == AUDCLNT_E_DEVICE_INVALIDATED {
            let _ = self.sender.send(Command::RebuildEndpoint);
        }
        winerr(err)
    }
}

#[implement(IAudioSessionNotification)]
struct ManagerSink {
    sender: crossbeam_channel::Sender<Command>,
}

impl IAudioSessionNotification_Impl for ManagerSink_Impl {
    fn OnSessionCreated(
        &self,
        new_session: Ref<IAudioSessionControl>,
    ) -> windows::core::Result<()> {
        unsafe {
            let Some(new_session) = new_session.as_ref() else {
                return Ok(());
            };
            let git: windows::core::Result<IGlobalInterfaceTable> =
                CoCreateInstance(&CLSID_STD_GLOBAL_INTERFACE_TABLE, None, CLSCTX_ALL);
            match git.and_then(|git| {
                git.RegisterInterfaceInGlobal(new_session, &IAudioSessionControl::IID)
            }) {
                Ok(cookie) => {
                    debug!(cookie, "OnSessionCreated queued GIT cookie");
                    // Only the worker revokes a cookie. If it is gone, this
                    // callback must revoke it here, or the globally registered
                    // interface is never released.
                    if self.sender.send(Command::AdoptGit(cookie)).is_err() {
                        if let Ok(git) = CoCreateInstance::<_, IGlobalInterfaceTable>(
                            &CLSID_STD_GLOBAL_INTERFACE_TABLE,
                            None,
                            CLSCTX_ALL,
                        ) {
                            let _ = git.RevokeInterfaceFromGlobal(cookie);
                        }
                    }
                }
                Err(err) => {
                    warn!(error = %err, "GIT registration failed; requesting enumeration fallback");
                    let _ = self.sender.send(Command::Reenumerate);
                }
            }
        }
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct EndpointSink {
    sender: crossbeam_channel::Sender<Command>,
}

impl IMMNotificationClient_Impl for EndpointSink_Impl {
    fn OnDeviceStateChanged(
        &self,
        _device_id: &PCWSTR,
        _new_state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceAdded(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceRemoved(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.sender.send(Command::RebuildEndpoint);
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        flow: EDataFlow,
        role: ERole,
        _device_id: &PCWSTR,
    ) -> windows::core::Result<()> {
        if flow == eRender && role == eConsole {
            let _ = self.sender.send(Command::RebuildEndpoint);
        }
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _device_id: &PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

#[implement(IAudioSessionEvents)]
struct SessionSink {
    live_id: String,
    sender: crossbeam_channel::Sender<Command>,
}

impl IAudioSessionEvents_Impl for SessionSink_Impl {
    fn OnDisplayNameChanged(
        &self,
        _new_name: &PCWSTR,
        _event_context: *const GUID,
    ) -> windows::core::Result<()> {
        let _ = self
            .sender
            .send(Command::SessionChanged(self.live_id.clone()));
        Ok(())
    }

    fn OnIconPathChanged(
        &self,
        _new_path: &PCWSTR,
        _event_context: *const GUID,
    ) -> windows::core::Result<()> {
        let _ = self
            .sender
            .send(Command::SessionChanged(self.live_id.clone()));
        Ok(())
    }

    fn OnSimpleVolumeChanged(
        &self,
        _new_volume: f32,
        _new_mute: BOOL,
        _event_context: *const GUID,
    ) -> windows::core::Result<()> {
        let _ = self
            .sender
            .send(Command::SessionChanged(self.live_id.clone()));
        Ok(())
    }

    fn OnChannelVolumeChanged(
        &self,
        _channel_count: u32,
        _new_volumes: *const f32,
        _changed_channel: u32,
        _event_context: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnGroupingParamChanged(
        &self,
        _new_group: *const GUID,
        _event_context: *const GUID,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnStateChanged(&self, new_state: AudioSessionState) -> windows::core::Result<()> {
        let command = if new_state == AudioSessionStateExpired {
            Command::SessionRemoved(self.live_id.clone())
        } else {
            Command::SessionChanged(self.live_id.clone())
        };
        let _ = self.sender.send(command);
        Ok(())
    }

    fn OnSessionDisconnected(
        &self,
        _reason: AudioSessionDisconnectReason,
    ) -> windows::core::Result<()> {
        let _ = self
            .sender
            .send(Command::SessionRemoved(self.live_id.clone()));
        Ok(())
    }
}

unsafe fn exempt_audio_thread_from_throttling() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadInformation, ThreadPowerThrottling,
        THREAD_POWER_THROTTLING_CURRENT_VERSION, THREAD_POWER_THROTTLING_EXECUTION_SPEED,
        THREAD_POWER_THROTTLING_STATE,
    };
    let state = THREAD_POWER_THROTTLING_STATE {
        Version: THREAD_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: 0,
    };
    if let Err(error) = SetThreadInformation(
        GetCurrentThread(),
        ThreadPowerThrottling,
        &state as *const _ as *const _,
        std::mem::size_of_val(&state) as u32,
    ) {
        warn!(%error, "cannot exempt audio worker from power throttling");
    }
}

fn resolve_indirect_name(value: &str) -> Option<String> {
    if !value.starts_with('@') {
        return None;
    }
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::SHLoadIndirectString;
    let wide: Vec<u16> = std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut output = vec![0u16; 1024];
    unsafe {
        SHLoadIndirectString(PCWSTR(wide.as_ptr()), &mut output, None).ok()?;
    }
    let end = output
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(output.len());
    let text = String::from_utf16_lossy(&output[..end]);
    (!text.trim().is_empty()).then_some(text)
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

unsafe fn endpoint_friendly_name(endpoint: &IMMDevice) -> windows::core::Result<String> {
    let store = endpoint.OpenPropertyStore(STGM_READ)?;
    let mut value = store.GetValue(&PKEY_Device_FriendlyName)?;
    let text = PropVariantToStringAlloc(&value).map(|value| take_pwstr(value));
    let _ = PropVariantClear(&mut value);
    text
}
