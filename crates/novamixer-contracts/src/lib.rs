use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AudioSession {
    pub live_id: String,
    pub app_key: String,
    pub display_name: String,
    pub executable_name: Option<String>,
    pub executable_path: Option<String>,
    pub process_id: Option<u32>,
    pub icon: Option<String>,
    pub volume: f32,
    pub muted: bool,
    pub state: SessionState,
    pub is_system_sounds: bool,
    pub controllable: bool,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct MixerSnapshot {
    pub master: MasterState,
    pub sessions: Vec<AudioSession>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct PeakBatch {
    pub timestamp_ms: u64,
    pub master_peak: f32,
    pub sessions: Vec<SessionPeak>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SessionPeak {
    pub live_id: String,
    pub peak: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AppBinding {
    pub app_key: String,
    pub display_name: String,
    pub executable_name: Option<String>,
    pub executable_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub volume: f32,
    pub apps: Vec<AppBinding>,
    pub startup_volume: Option<f32>,
    pub auto_mute_on_launch: bool,
    pub hotkeys_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct HotkeyBinding {
    pub action: HotkeyAction,
    pub accelerator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyAction {
    VolumeUp,
    VolumeDown,
    MuteToggle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
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
    pub show_inactive: bool,
    pub show_system_sounds: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct AppSettings {
    pub schema_version: u32,
    pub ui_prefs: UiPrefs,
    pub groups: Vec<Group>,
    pub active_group_id: Option<String>,
    pub hotkeys: Vec<HotkeyBinding>,
    pub volume_step: f32,
    pub smart_volume: bool,
    pub launch_on_startup: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub auto_save: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ApiError {
    pub kind: String,
    pub message: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            schema_version: 1,
            ui_prefs: UiPrefs {
                theme: Theme::Dark,
                language: "en".into(),
                sidebar_collapsed: false,
                density: Density::Comfy,
                show_inactive: true,
                show_system_sounds: true,
            },
            groups: vec![Group {
                id: id.clone(),
                name: "Main".into(),
                is_default: true,
                volume: 1.0,
                apps: vec![],
                startup_volume: None,
                auto_mute_on_launch: false,
                hotkeys_enabled: false,
            }],
            active_group_id: Some(id),
            hotkeys: vec![
                HotkeyBinding {
                    action: HotkeyAction::VolumeUp,
                    accelerator: Some("AudioVolumeUp".into()),
                },
                HotkeyBinding {
                    action: HotkeyAction::VolumeDown,
                    accelerator: Some("AudioVolumeDown".into()),
                },
                HotkeyBinding {
                    action: HotkeyAction::MuteToggle,
                    accelerator: Some("AudioVolumeMute".into()),
                },
            ],
            volume_step: 0.05,
            smart_volume: true,
            launch_on_startup: false,
            start_minimized: false,
            minimize_to_tray: true,
            auto_save: true,
        }
    }
}

impl AppSettings {
    pub fn validate(&mut self) {
        self.volume_step = self.volume_step.clamp(0.0, 1.0);
        for group in &mut self.groups {
            group.volume = group.volume.clamp(0.0, 1.0);
            group.startup_volume = group.startup_volume.map(|value| value.clamp(0.0, 1.0));
        }
        if self.groups.is_empty() {
            self.groups = Self::default().groups;
        }
        let chosen = self
            .groups
            .iter()
            .position(|group| group.is_default)
            .unwrap_or(0);
        for (index, group) in self.groups.iter_mut().enumerate() {
            group.is_default = index == chosen;
        }
        if !self
            .active_group_id
            .as_ref()
            .is_some_and(|id| self.groups.iter().any(|group| &group.id == id))
        {
            self.active_group_id = Some(self.groups[chosen].id.clone());
        }
        for binding in &mut self.hotkeys {
            if let Some(accelerator) = &binding.accelerator {
                binding.accelerator = Some(repair_accelerator(accelerator));
            }
        }
    }
}

/// Rewrites accelerator names that an earlier build wrote but the platform
/// rejects.
///
/// A settings file outlives the build that wrote it, and a rejected accelerator
/// leaves its action silently unbound, so the repair happens on load rather than
/// being left for the user to discover and fix by hand.
fn repair_accelerator(accelerator: &str) -> String {
    // Tauri names the volume keys `AudioVolume*`; `MediaVolume*` never parsed.
    accelerator.replace("MediaVolume", "AudioVolume")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{de::DeserializeOwned, Serialize};

    fn round_trip<T>(value: T)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<T>(&json).unwrap(), value);
    }

    fn samples() -> (AudioSession, MasterState, AppBinding, Group) {
        let app = AppBinding {
            app_key: "app".into(),
            display_name: "App".into(),
            executable_name: Some("app.exe".into()),
            executable_path: None,
        };
        let group = Group {
            id: "g".into(),
            name: "Group".into(),
            is_default: true,
            volume: 0.5,
            apps: vec![app.clone()],
            startup_volume: Some(0.4),
            auto_mute_on_launch: false,
            hotkeys_enabled: true,
        };
        let audio = AudioSession {
            live_id: "live".into(),
            app_key: "app".into(),
            display_name: "App".into(),
            executable_name: Some("app.exe".into()),
            executable_path: None,
            process_id: Some(42),
            icon: None,
            volume: 0.5,
            muted: false,
            state: SessionState::Active,
            is_system_sounds: false,
            controllable: true,
            group_id: Some("g".into()),
        };
        let master = MasterState {
            endpoint_id: "endpoint".into(),
            endpoint_name: "Speakers".into(),
            volume: 0.8,
            muted: false,
        };
        (audio, master, app, group)
    }

    #[test]
    fn every_dto_round_trips() {
        let (audio, master, app, group) = samples();
        round_trip(audio.clone());
        round_trip(master.clone());
        round_trip(MixerSnapshot {
            master,
            sessions: vec![audio],
        });
        round_trip(PeakBatch {
            timestamp_ms: 7,
            master_peak: 0.2,
            sessions: vec![SessionPeak {
                live_id: "live".into(),
                peak: 0.1,
            }],
        });
        round_trip(SessionPeak {
            live_id: "live".into(),
            peak: 0.1,
        });
        round_trip(app);
        round_trip(group);
        round_trip(HotkeyBinding {
            action: HotkeyAction::MuteToggle,
            accelerator: None,
        });
        round_trip(UiPrefs {
            theme: Theme::Light,
            language: "es".into(),
            sidebar_collapsed: true,
            density: Density::Compact,
            show_inactive: false,
            show_system_sounds: false,
        });
        round_trip(AppSettings::default());
        round_trip(ApiError {
            kind: "other".into(),
            message: "message".into(),
        });
    }

    #[test]
    fn enum_wire_names_are_literal_snake_case() {
        assert_eq!(
            serde_json::to_string(&SessionState::Inactive).unwrap(),
            "\"inactive\""
        );
        assert_eq!(
            serde_json::to_string(&HotkeyAction::VolumeDown).unwrap(),
            "\"volume_down\""
        );
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), "\"dark\"");
        assert_eq!(serde_json::to_string(&Density::Comfy).unwrap(), "\"comfy\"");
    }

    #[test]
    fn defaults_match_contract() {
        let settings = AppSettings::default();
        assert_eq!(settings.schema_version, 1);
        assert_eq!(settings.ui_prefs.theme, Theme::Dark);
        assert_eq!(settings.ui_prefs.language, "en");
        assert_eq!(settings.groups.len(), 1);
        assert!(settings.groups[0].is_default);
        assert_eq!(
            settings.active_group_id.as_deref(),
            Some(settings.groups[0].id.as_str())
        );
        assert_eq!(settings.volume_step, 0.05);
        assert!(settings.smart_volume && settings.minimize_to_tray && settings.auto_save);
        assert!(
            !settings.launch_on_startup
                && !settings.start_minimized
                && !settings.ui_prefs.sidebar_collapsed
        );
    }

    #[test]
    fn validate_clamps_and_repairs() {
        let mut settings = AppSettings {
            volume_step: 2.0,
            ..AppSettings::default()
        };
        settings.groups[0].volume = -1.0;
        settings.groups[0].startup_volume = Some(4.0);
        settings.groups.push(Group {
            id: "other".into(),
            name: "Other".into(),
            is_default: true,
            volume: 2.0,
            apps: vec![],
            startup_volume: Some(-1.0),
            auto_mute_on_launch: false,
            hotkeys_enabled: false,
        });
        settings.active_group_id = Some("missing".into());
        settings.validate();
        assert_eq!(settings.volume_step, 1.0);
        assert_eq!(settings.groups[0].volume, 0.0);
        assert_eq!(settings.groups[0].startup_volume, Some(1.0));
        assert_eq!(settings.groups[1].volume, 1.0);
        assert_eq!(settings.groups[1].startup_volume, Some(0.0));
        assert_eq!(
            settings
                .groups
                .iter()
                .filter(|group| group.is_default)
                .count(),
            1
        );
        assert_eq!(
            settings.active_group_id.as_deref(),
            Some(settings.groups[0].id.as_str())
        );
    }

    #[test]
    fn validate_repairs_empty_groups() {
        let mut settings = AppSettings::default();
        settings.groups.clear();
        settings.active_group_id = None;
        settings.validate();
        assert_eq!(settings.groups.len(), 1);
        assert!(settings.groups[0].is_default);
        assert_eq!(
            settings.active_group_id.as_deref(),
            Some(settings.groups[0].id.as_str())
        );
    }

    #[test]
    fn validate_repairs_rejected_media_key_names() {
        // An earlier build wrote `MediaVolumeUp`, which the platform refuses to
        // parse. Left alone it leaves the action permanently unbound.
        let mut settings = AppSettings {
            hotkeys: vec![
                HotkeyBinding {
                    action: HotkeyAction::VolumeUp,
                    accelerator: Some("MediaVolumeUp".into()),
                },
                HotkeyBinding {
                    action: HotkeyAction::MuteToggle,
                    accelerator: Some("Control+Alt+M".into()),
                },
                HotkeyBinding {
                    action: HotkeyAction::VolumeDown,
                    accelerator: None,
                },
            ],
            ..AppSettings::default()
        };

        settings.validate();

        assert_eq!(
            settings.hotkeys[0].accelerator.as_deref(),
            Some("AudioVolumeUp")
        );
        // An accelerator that already parses is left exactly as it was.
        assert_eq!(
            settings.hotkeys[1].accelerator.as_deref(),
            Some("Control+Alt+M")
        );
        assert_eq!(settings.hotkeys[2].accelerator, None);
    }

    #[test]
    fn defaults_use_accelerators_the_platform_accepts() {
        for binding in AppSettings::default().hotkeys {
            let accelerator = binding.accelerator.expect("default bindings are bound");
            assert!(
                !accelerator.contains("MediaVolume"),
                "{accelerator} is not a valid accelerator name"
            );
        }
    }
}
