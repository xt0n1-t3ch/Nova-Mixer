use audio_sessions::{AudioError, Registry, Result, SessionHandle, SessionSource};
use novamixer_contracts::{Application, AudioSession, IdentityKind, MasterState, SessionState};
use std::sync::{Arc, Mutex};
#[derive(Clone)]
struct FakeHandle {
    session: AudioSession,
    application: Application,
    observed: Arc<Mutex<Vec<f32>>>,
}
impl SessionHandle for FakeHandle {
    fn snapshot(&self) -> AudioSession {
        self.session.clone()
    }
    fn application(&self) -> Application {
        self.application.clone()
    }
    fn set_volume(&mut self, v: f32) -> Result<()> {
        self.session.volume = v;
        self.observed.lock().unwrap().push(v);
        Ok(())
    }
    fn set_mute(&mut self, m: bool) -> Result<()> {
        self.session.muted = m;
        Ok(())
    }
    fn peak(&self) -> Option<f32> {
        Some(self.session.peak)
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
            peak: 0.0,
        })
    }
    fn enumerate(&mut self) -> Result<Vec<Self::Handle>> {
        Ok(self.sessions.clone())
    }
    fn set_master_volume(&mut self, _: f32) -> Result<()> {
        Ok(())
    }
    fn set_master_mute(&mut self, _: bool) -> Result<()> {
        Ok(())
    }
    fn master_peak(&self) -> f32 {
        0.0
    }
}
fn session(id: &str, volume: f32, observed: Arc<Mutex<Vec<f32>>>) -> FakeHandle {
    path_session(id, volume, "C:\\Apps\\Discord.exe", observed)
}
fn path_session(id: &str, volume: f32, path: &str, observed: Arc<Mutex<Vec<f32>>>) -> FakeHandle {
    let app_key = path.to_lowercase();
    let mut application =
        Application::offline(app_key.clone(), IdentityKind::Path, "Discord".into());
    application.executable_name = Some("Discord.exe".into());
    application.executable_path = Some(path.into());
    FakeHandle {
        session: AudioSession {
            live_id: id.into(),
            app_key,
            display_name: "Discord".into(),
            process_id: Some(42),
            volume,
            muted: false,
            state: SessionState::Active,
            controllable: true,
            peak: 0.2,
        },
        application,
        observed,
    }
}
#[test]
fn application_aggregation_and_late_inheritance() -> std::result::Result<(), AudioError> {
    let seen = Arc::new(Mutex::new(vec![]));
    let source = FakeSource {
        sessions: vec![
            session("one", 0.2, seen.clone()),
            session("two", 0.7, seen.clone()),
        ],
    };
    let mut registry = Registry::start(source, vec![], vec![])?;
    let snap = registry.snapshot()?;
    assert_eq!(snap.applications.len(), 1);
    assert_eq!(snap.applications[0].sessions.len(), 2);
    assert!(snap.applications[0].mixed);
    registry.set_app_volume("c:\\apps\\discord.exe", 0.4)?;
    assert_eq!(*seen.lock().unwrap(), vec![0.4, 0.4]);
    assert!(!registry.snapshot()?.applications[0].mixed);
    registry.adopt(session("three", 1.0, seen.clone()), true)?;
    assert_eq!(seen.lock().unwrap().last(), Some(&0.4));
    assert_eq!(registry.snapshot()?.applications[0].sessions.len(), 3);
    registry.remove("one");
    registry.remove("two");
    registry.remove("three");
    let app = &registry.snapshot()?.applications[0];
    assert!(!app.running);
    assert!(app.sessions.is_empty());
    Ok(())
}
#[test]
fn filename_identity_upgrades_to_path_and_keeps_policy() -> std::result::Result<(), AudioError> {
    let seen = Arc::new(Mutex::new(vec![]));
    let mut persisted = Application::offline(
        "discord.exe".into(),
        IdentityKind::Filename,
        "Discord".into(),
    );
    persisted.executable_name = Some("Discord.exe".into());
    persisted.volume = 0.27;
    persisted.muted = true;
    persisted.pinned = true;
    persisted.hidden = true;
    persisted.custom_name = Some("Chat".into());
    persisted.display_name = "Chat".into();
    persisted.group_id = Some("group".into());
    persisted.sort_order = 4;
    let mut registry = Registry::start(FakeSource { sessions: vec![] }, vec![persisted], vec![])?;
    registry.adopt(session("live", 1.0, seen.clone()), true)?;
    let snapshot = registry.snapshot()?;
    assert_eq!(snapshot.applications.len(), 1);
    let app = &snapshot.applications[0];
    assert_eq!(app.app_key, "c:\\apps\\discord.exe");
    assert_eq!(app.identity_kind, IdentityKind::Path);
    assert_eq!(app.volume, 0.27);
    assert!(app.muted && app.pinned && app.hidden);
    assert_eq!(app.custom_name.as_deref(), Some("Chat"));
    assert_eq!(app.group_id.as_deref(), Some("group"));
    assert_eq!(app.sort_order, 4);
    Ok(())
}

#[test]
fn startup_deduplicates_filename_and_path_records() -> std::result::Result<(), AudioError> {
    let mut filename = Application::offline(
        "discord.exe".into(),
        IdentityKind::Filename,
        "Discord".into(),
    );
    filename.executable_name = Some("Discord.exe".into());
    filename.volume = 0.31;
    filename.pinned = true;
    let mut path = Application::offline(
        "c:\\apps\\discord.exe".into(),
        IdentityKind::Path,
        "Discord".into(),
    );
    path.executable_name = Some("Discord.exe".into());
    path.executable_path = Some("C:\\Apps\\Discord.exe".into());
    let registry = Registry::start(
        FakeSource { sessions: vec![] },
        vec![filename, path],
        vec![],
    )?;
    let snapshot = registry.snapshot()?;
    assert_eq!(snapshot.applications.len(), 1);
    assert_eq!(snapshot.applications[0].app_key, "c:\\apps\\discord.exe");
    assert_eq!(snapshot.applications[0].volume, 0.31);
    assert!(snapshot.applications[0].pinned);
    Ok(())
}

#[test]
fn same_filename_in_different_directories_stays_distinct() -> std::result::Result<(), AudioError> {
    let seen = Arc::new(Mutex::new(vec![]));
    let source = FakeSource {
        sessions: vec![
            path_session("one", 1.0, "C:\\One\\Discord.exe", seen.clone()),
            path_session("two", 1.0, "D:\\Two\\Discord.exe", seen),
        ],
    };
    let registry = Registry::start(source, vec![], vec![])?;
    assert_eq!(registry.snapshot()?.applications.len(), 2);
    Ok(())
}

#[test]
fn session_created_after_startup_is_controllable() -> std::result::Result<(), AudioError> {
    let seen = Arc::new(Mutex::new(vec![]));
    let mut registry = Registry::start(FakeSource { sessions: vec![] }, vec![], vec![])?;
    registry.adopt(session("late", 1.0, seen.clone()), true)?;
    registry.set_session_volume("late", 0.42)?;
    assert_eq!(seen.lock().unwrap().last(), Some(&0.42));
    Ok(())
}
