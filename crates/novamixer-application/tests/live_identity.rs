//! The saved record of a Squirrel.Windows application must follow the
//! executable that is running, in the order the real app starts: settings load
//! (with the record still naming the old version folder), the audio registry
//! adopts the session that is already playing, and the result is persisted.
use audio_sessions::{Registry, Result, SessionHandle, SessionSource};
use novamixer_application::refresh_live_identities;
use novamixer_contracts::{
    AppSettings, Application, AudioSession, IdentityKind, MasterState, SessionState,
};

const OLD: &str = r"C:\Users\xt0n1\AppData\Local\Discord\app-1.0.9256\Discord.exe";
const LIVE: &str = r"C:\Users\xt0n1\AppData\Local\Discord\app-1.0.9258\Discord.exe";
const LIVE_ICON: &str = "data:image/png;base64,bGl2ZQ==";

#[derive(Clone)]
struct Handle {
    session: AudioSession,
    application: Application,
}
impl SessionHandle for Handle {
    fn snapshot(&self) -> AudioSession {
        self.session.clone()
    }
    fn application(&self) -> Application {
        self.application.clone()
    }
    fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.session.volume = volume;
        Ok(())
    }
    fn set_mute(&mut self, muted: bool) -> Result<()> {
        self.session.muted = muted;
        Ok(())
    }
    fn peak(&self) -> Option<f32> {
        None
    }
}
struct Source(Vec<Handle>);
impl SessionSource for Source {
    type Handle = Handle;
    fn master(&self) -> Result<MasterState> {
        Ok(MasterState {
            endpoint_id: "endpoint".into(),
            endpoint_name: "Speakers".into(),
            volume: 1.0,
            muted: false,
            peak: 0.0,
        })
    }
    fn enumerate(&mut self) -> Result<Vec<Handle>> {
        Ok(self.0.clone())
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

/// The session exactly as the Windows worker builds it: key from the live
/// image path, icon and version name read from that executable.
fn live_discord_session() -> Handle {
    let key = audio_policy::resolve_app_key(None, Some(LIVE), Some("Discord.exe"));
    let mut application = Application::offline(key.clone(), IdentityKind::Path, "Discord".into());
    application.executable_name = Some("Discord.exe".into());
    application.executable_path = Some(LIVE.into());
    application.icon = Some(LIVE_ICON.into());
    Handle {
        session: AudioSession {
            live_id: "endpoint::discord".into(),
            app_key: key,
            display_name: "Discord".into(),
            process_id: Some(26600),
            volume: 1.0,
            muted: false,
            state: SessionState::Active,
            controllable: true,
            peak: 0.0,
        },
        application,
    }
}

/// The record as the settings store hands it over after the Squirrel
/// migration: merged key, but still the path of the removed 9256 folder.
fn persisted_discord() -> Application {
    let key = audio_policy::resolve_app_key(None, Some(OLD), Some("Discord.exe"));
    let mut app = Application::offline(key, IdentityKind::Path, "Discord".into());
    app.executable_name = Some("Discord.exe".into());
    app.executable_path = Some(OLD.into());
    app.icon = Some("data:image/png;base64,b2xk".into());
    app.volume = 0.25;
    app.custom_name = Some("Chat".into());
    app.display_name = "Chat".into();
    app
}

#[test]
fn startup_moves_saved_squirrel_path_to_the_running_version() -> Result<()> {
    let mut settings = AppSettings {
        applications: vec![persisted_discord()],
        ..AppSettings::default()
    };

    let registry = Registry::start(
        Source(vec![live_discord_session()]),
        settings.applications.clone(),
        settings.groups.clone(),
    )?;
    let snapshot = registry.snapshot()?;

    assert_eq!(snapshot.applications.len(), 1);
    let live = &snapshot.applications[0];
    assert!(live.running);
    assert_eq!(live.volume, 0.25, "remembered level applied");
    assert_eq!(live.executable_path.as_deref(), Some(LIVE), "snapshot");

    assert!(refresh_live_identities(&mut settings, &snapshot));
    let saved = &settings.applications[0];
    assert_eq!(saved.executable_path.as_deref(), Some(LIVE), "persisted");
    assert_eq!(
        saved.icon.as_deref(),
        Some(LIVE_ICON),
        "icon follows the live path"
    );
    assert_eq!(saved.custom_name.as_deref(), Some("Chat"));
    assert_eq!(saved.display_name, "Chat", "a rename survives");
    assert_eq!(saved.volume, 0.25);
    assert!(
        !refresh_live_identities(&mut settings, &snapshot),
        "a second pass is a no-op, so startup saves at most once"
    );
    Ok(())
}
