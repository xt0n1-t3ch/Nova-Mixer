use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Application {
    pub app_key: String,
    pub identity_kind: IdentityKind,
    pub display_name: String,
    pub custom_name: Option<String>,
    pub executable_name: Option<String>,
    pub executable_path: Option<String>,
    pub icon: Option<String>,
    pub volume: f32,
    pub muted: bool,
    pub mixed: bool,
    pub remembered: bool,
    pub pinned: bool,
    pub hidden: bool,
    pub sort_order: u32,
    pub running: bool,
    pub controllable: bool,
    pub is_system_sounds: bool,
    pub group_id: Option<String>,
    pub sessions: Vec<AudioSession>,
    pub peak: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum IdentityKind {
    Aumid,
    Path,
    Filename,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AudioSession {
    pub live_id: String,
    pub app_key: String,
    pub display_name: String,
    pub process_id: Option<u32>,
    pub volume: f32,
    pub muted: bool,
    pub state: SessionState,
    pub controllable: bool,
    pub peak: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    Inactive,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MasterState {
    pub endpoint_id: String,
    pub endpoint_name: String,
    pub volume: f32,
    pub muted: bool,
    pub peak: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MixerSnapshot {
    pub master: MasterState,
    pub applications: Vec<Application>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AppCandidate {
    pub app_key: String,
    pub display_name: String,
    pub executable_name: Option<String>,
    pub executable_path: Option<String>,
    pub icon: Option<String>,
    pub running: bool,
    pub already_managed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct PeakBatch {
    pub timestamp_ms: u64,
    pub master_peak: f32,
    pub applications: Vec<AppPeak>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AppPeak {
    pub app_key: String,
    pub peak: f32,
    pub sessions: Vec<SessionPeak>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SessionPeak {
    pub live_id: String,
    pub peak: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub master_volume: Option<f32>,
    pub entries: Vec<SceneEntry>,
    pub fade_ms: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SceneEntry {
    pub app_key: String,
    pub volume: Option<f32>,
    pub muted: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub volume: f32,
    pub app_keys: Vec<String>,
    pub startup_volume: Option<f32>,
    pub auto_mute_on_launch: bool,
    pub hotkeys_enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct HotkeyBinding {
    pub action: HotkeyAction,
    pub accelerator: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyAction {
    VolumeUp,
    VolumeDown,
    MuteToggle,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    Comfy,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct UiPrefs {
    pub theme: Theme,
    pub language: String,
    pub sidebar_collapsed: bool,
    pub density: Density,
    pub show_offline: bool,
    pub show_hidden: bool,
    pub show_system_sounds: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
pub struct ApplicationPatch {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable::deserialize"
    )]
    pub custom_name: Option<Option<String>>,
    pub remembered: Option<bool>,
    pub pinned: Option<bool>,
    pub hidden: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable::deserialize"
    )]
    pub group_id: Option<Option<String>>,
}
mod nullable {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct EfficiencyStatus {
    pub supported: bool,
    pub enabled: bool,
    pub detail: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AppSettings {
    pub schema_version: u32,
    pub ui_prefs: UiPrefs,
    pub applications: Vec<Application>,
    pub groups: Vec<Group>,
    pub scenes: Vec<Scene>,
    pub active_group_id: Option<String>,
    pub hotkeys: Vec<HotkeyBinding>,
    pub volume_step: f32,
    pub smart_volume: bool,
    pub launch_on_startup: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub auto_save: bool,
    pub efficiency_mode: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ApiError {
    pub kind: String,
    pub message: String,
}

impl Application {
    pub fn offline(app_key: String, identity_kind: IdentityKind, display_name: String) -> Self {
        Self {
            app_key,
            identity_kind,
            display_name,
            custom_name: None,
            executable_name: None,
            executable_path: None,
            icon: None,
            volume: 1.0,
            muted: false,
            mixed: false,
            remembered: true,
            pinned: false,
            hidden: false,
            sort_order: 0,
            running: false,
            controllable: false,
            is_system_sounds: false,
            group_id: None,
            sessions: vec![],
            peak: 0.0,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            schema_version: 2,
            ui_prefs: UiPrefs {
                theme: Theme::Dark,
                language: "en".into(),
                sidebar_collapsed: false,
                density: Density::Comfy,
                show_offline: true,
                show_hidden: false,
                show_system_sounds: true,
            },
            applications: vec![],
            groups: vec![Group {
                id: id.clone(),
                name: "Main".into(),
                is_default: true,
                volume: 1.0,
                app_keys: vec![],
                startup_volume: None,
                auto_mute_on_launch: false,
                hotkeys_enabled: false,
            }],
            scenes: vec![],
            active_group_id: Some(id),
            hotkeys: vec![
                binding(HotkeyAction::VolumeUp, "AudioVolumeUp"),
                binding(HotkeyAction::VolumeDown, "AudioVolumeDown"),
                binding(HotkeyAction::MuteToggle, "AudioVolumeMute"),
            ],
            volume_step: 0.05,
            smart_volume: true,
            launch_on_startup: false,
            start_minimized: false,
            minimize_to_tray: true,
            auto_save: true,
            efficiency_mode: false,
        }
    }
}
fn binding(action: HotkeyAction, key: &str) -> HotkeyBinding {
    HotkeyBinding {
        action,
        accelerator: Some(key.into()),
    }
}
fn scalar(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
impl AppSettings {
    pub fn validate(&mut self) {
        self.schema_version = 2;
        self.volume_step = scalar(self.volume_step);
        let mut seen = HashSet::new();
        self.applications
            .retain(|app| seen.insert(app.app_key.to_lowercase()));
        for (index, app) in self.applications.iter_mut().enumerate() {
            app.app_key = app.app_key.to_lowercase();
            app.volume = scalar(app.volume);
            app.peak = scalar(app.peak);
            app.sort_order = index as u32;
            app.sessions.clear();
            app.running = false;
            app.controllable = false;
            app.mixed = false;
            app.display_name = app
                .custom_name
                .clone()
                .unwrap_or_else(|| app.display_name.clone());
        }
        for group in &mut self.groups {
            group.volume = scalar(group.volume);
            group.startup_volume = group.startup_volume.map(scalar);
            let mut keys = HashSet::new();
            group.app_keys.retain(|k| keys.insert(k.to_lowercase()));
        }
        if self.groups.is_empty() {
            self.groups = Self::default().groups;
        }
        let chosen = self.groups.iter().position(|g| g.is_default).unwrap_or(0);
        for (i, group) in self.groups.iter_mut().enumerate() {
            group.is_default = i == chosen;
        }
        if !self
            .active_group_id
            .as_ref()
            .is_some_and(|id| self.groups.iter().any(|g| &g.id == id))
        {
            self.active_group_id = Some(self.groups[chosen].id.clone());
        }
        for scene in &mut self.scenes {
            scene.master_volume = scene.master_volume.map(scalar);
            for entry in &mut scene.entries {
                entry.volume = entry.volume.map(scalar);
            }
        }
        for item in &mut self.hotkeys {
            if let Some(value) = &item.accelerator {
                item.accelerator = Some(value.replace("MediaVolume", "AudioVolume"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enum_wire_names_are_snake_case() {
        assert_eq!(
            serde_json::to_string(&IdentityKind::Filename).unwrap(),
            "\"filename\""
        );
        assert_eq!(
            serde_json::to_string(&SessionState::Inactive).unwrap(),
            "\"inactive\""
        );
        assert_eq!(
            serde_json::to_string(&HotkeyAction::VolumeDown).unwrap(),
            "\"volume_down\""
        );
    }
    #[test]
    fn defaults_match_v2() {
        let s = AppSettings::default();
        assert_eq!(s.schema_version, 2);
        assert!(s.applications.is_empty() && s.scenes.is_empty() && s.ui_prefs.show_offline);
        assert!(!s.efficiency_mode);
    }
    #[test]
    fn all_dtos_round_trip() {
        let s = AppSettings::default();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<AppSettings>(&json).unwrap(), s);
        let patch = ApplicationPatch {
            custom_name: Some(None),
            ..Default::default()
        };
        assert_eq!(
            serde_json::from_str::<ApplicationPatch>(&serde_json::to_string(&patch).unwrap())
                .unwrap(),
            patch
        );
        let status = EfficiencyStatus {
            supported: true,
            enabled: true,
            detail: None,
        };
        assert_eq!(
            serde_json::from_str::<EfficiencyStatus>(&serde_json::to_string(&status).unwrap())
                .unwrap(),
            status
        );
    }
    #[test]
    fn validate_repairs_every_invariant() {
        let mut s = AppSettings::default();
        let mut a = Application::offline("APP.EXE".into(), IdentityKind::Filename, "App".into());
        a.volume = 2.0;
        a.peak = f32::NAN;
        a.sort_order = 99;
        a.running = true;
        a.mixed = true;
        a.sessions.push(AudioSession {
            live_id: "x".into(),
            app_key: "x".into(),
            display_name: "x".into(),
            process_id: None,
            volume: 1.0,
            muted: false,
            state: SessionState::Active,
            controllable: true,
            peak: 0.0,
        });
        s.applications = vec![a.clone(), a];
        s.volume_step = -2.0;
        s.groups[0].volume = 3.0;
        s.groups[0].startup_volume = Some(-1.0);
        s.hotkeys[0].accelerator = Some("MediaVolumeUp".into());
        s.schema_version = 1;
        s.validate();
        assert_eq!(s.schema_version, 2);
        assert_eq!(s.applications.len(), 1);
        assert_eq!(s.applications[0].app_key, "app.exe");
        assert_eq!(s.applications[0].sort_order, 0);
        assert!(
            !s.applications[0].running
                && !s.applications[0].mixed
                && s.applications[0].sessions.is_empty()
        );
        assert_eq!(s.volume_step, 0.0);
        assert_eq!(s.groups[0].volume, 1.0);
        assert_eq!(s.groups[0].startup_volume, Some(0.0));
        assert_eq!(s.hotkeys[0].accelerator.as_deref(), Some("AudioVolumeUp"));
    }
}
