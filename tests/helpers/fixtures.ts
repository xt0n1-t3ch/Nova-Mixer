import type {
  AppSettings,
  AudioSession,
  Group,
  MasterState,
} from "@/lib/api";

/** A plausible live session; override only the fields a test cares about. */
export function makeSession(overrides: Partial<AudioSession> = {}): AudioSession {
  return {
    live_id: "endpoint-1::session-1",
    app_key: "spotify.exe",
    display_name: "Spotify",
    executable_name: "Spotify.exe",
    executable_path: "C:\\Users\\test\\AppData\\Roaming\\Spotify\\Spotify.exe",
    process_id: 4242,
    icon: null,
    volume: 0.5,
    muted: false,
    state: "active",
    is_system_sounds: false,
    controllable: true,
    group_id: null,
    ...overrides,
  };
}

export function masterState(overrides: Partial<MasterState> = {}): MasterState {
  return {
    endpoint_id: "endpoint-1",
    endpoint_name: "Speakers (Realtek Audio)",
    volume: 0.72,
    muted: false,
    ...overrides,
  };
}

export function makeGroup(overrides: Partial<Group> = {}): Group {
  return {
    id: "group-1",
    name: "Main",
    is_default: true,
    volume: 1,
    apps: [],
    startup_volume: null,
    auto_mute_on_launch: false,
    hotkeys_enabled: true,
    ...overrides,
  };
}

/** Mirrors `AppSettings::default()` in `novamixer-contracts`. */
export function defaultSettings(overrides: Partial<AppSettings> = {}): AppSettings {
  return {
    schema_version: 1,
    ui_prefs: {
      theme: "dark",
      language: "en",
      sidebar_collapsed: false,
      density: "comfy",
      show_inactive: true,
      show_system_sounds: true,
    },
    groups: [makeGroup()],
    active_group_id: "group-1",
    hotkeys: [
      { action: "volume_up", accelerator: "AudioVolumeUp" },
      { action: "volume_down", accelerator: "AudioVolumeDown" },
      { action: "mute_toggle", accelerator: "AudioVolumeMute" },
    ],
    volume_step: 0.05,
    smart_volume: true,
    launch_on_startup: false,
    start_minimized: false,
    minimize_to_tray: true,
    auto_save: true,
    ...overrides,
  };
}
