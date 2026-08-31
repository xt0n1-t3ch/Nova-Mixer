use audio_sessions::{AudioError, Registry, Result, SessionHandle, SessionSource};
use novamixer_contracts::{AudioSession, MasterState, SessionState};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct FakeHandle {
    session: AudioSession,
    observed: Arc<Mutex<Vec<f32>>>,
}
impl SessionHandle for FakeHandle {
    fn snapshot(&self) -> AudioSession {
        self.session.clone()
    }
    fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.session.volume = volume;
        self.observed.lock().unwrap().push(volume);
        Ok(())
    }
    fn set_mute(&mut self, muted: bool) -> Result<()> {
        self.session.muted = muted;
        Ok(())
    }
    fn peak(&self) -> Option<f32> {
        Some(0.0)
    }
}
struct FakeSource {
    sessions: Vec<FakeHandle>,
}
impl SessionSource for FakeSource {
    type Handle = FakeHandle;
    fn master(&self) -> Result<MasterState> {
        Ok(MasterState {
            endpoint_id: "endpoint".into(),
            endpoint_name: "Fake".into(),
            volume: 1.0,
            muted: false,
        })
    }
    fn enumerate(&mut self) -> Result<Vec<Self::Handle>> {
        Ok(self.sessions.clone())
    }
    fn set_master_volume(&mut self, _volume: f32) -> Result<()> {
        Ok(())
    }
    fn set_master_mute(&mut self, _muted: bool) -> Result<()> {
        Ok(())
    }
    fn master_peak(&self) -> f32 {
        0.0
    }
}

#[test]
fn session_created_after_startup_is_controllable() -> std::result::Result<(), AudioError> {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let source = FakeSource { sessions: vec![] };
    let mut registry = Registry::start(source, vec![])?;
    assert!(registry.snapshot()?.sessions.is_empty());
    let session = AudioSession {
        live_id: "endpoint::late-session".into(),
        app_key: "late.exe".into(),
        display_name: "Late app".into(),
        executable_name: Some("late.exe".into()),
        executable_path: None,
        process_id: Some(42),
        icon: None,
        volume: 1.0,
        muted: false,
        state: SessionState::Active,
        is_system_sounds: false,
        controllable: true,
        group_id: None,
    };
    registry.adopt(
        FakeHandle {
            session,
            observed: observed.clone(),
        },
        true,
    )?;
    let updated = registry.set_session_volume("endpoint::late-session", 0.42)?;
    assert_eq!(updated.volume, 0.42);
    assert_eq!(*observed.lock().unwrap(), vec![0.42]);
    Ok(())
}
