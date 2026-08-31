import type {
  Application,
  AppSettings,
  AudioSession,
  Group,
  MasterState,
  Scene,
} from "@/lib/api";

/** A live session; a child of an application, never a top-level row. */
export function makeSession(overrides: Partial<AudioSession> = {}): AudioSession {
  return {
    live_id: "endpoint-1::session-1",
    app_key: "spotify.exe",
    display_name: "Spotify",
    process_id: 4242,
    volume: 0.5,
    muted: false,
    state: "active",
    controllable: true,
    peak: 0,
    ...overrides,
  };
}

/** A managed application; one row in the Applications view. */
export function makeApp(overrides: Partial<Application> = {}): Application {
  return {
    app_key: "spotify.exe",
    identity_kind: "path",
    display_name: "Spotify",
    custom_name: null,
    executable_name: "Spotify.exe",
    executable_path: "C:\\Users\\test\\AppData\\Roaming\\Spotify\\Spotify.exe",
    icon: null,
    volume: 0.5,
    muted: false,
    mixed: false,
    remembered: false,
    pinned: false,
    hidden: false,
    sort_order: 0,
    running: true,
    controllable: true,
    is_system_sounds: false,
    group_id: null,
    sessions: [],
    peak: 0,
    ...overrides,
  };
}

export function masterState(overrides: Partial<MasterState> = {}): MasterState {
  return {
    endpoint_id: "endpoint-1",
    endpoint_name: "Speakers (Realtek Audio)",
    volume: 0.72,
    muted: false,
    peak: 0,
    ...overrides,
  };
}

export function makeGroup(overrides: Partial<Group> = {}): Group {
  return {
    id: "group-1",
    name: "Main",
    is_default: true,
    volume: 1,
    app_keys: [],
    startup_volume: null,
    auto_mute_on_launch: false,
    hotkeys_enabled: true,
    ...overrides,
  };
}

export function makeScene(overrides: Partial<Scene> = {}): Scene {
  return {
    id: "scene-1",
    name: "Gaming",
    icon: null,
    master_volume: null,
    entries: [],
    fade_ms: 0,
    ...overrides,
  };
}

/** Mirrors `AppSettings::default()` in `novamixer-contracts`. */
export function defaultSettings(overrides: Partial<AppSettings> = {}): AppSettings {
  return {
    schema_version: 2,
    ui_prefs: {
      theme: "dark",
      language: "en",
      sidebar_collapsed: false,
      density: "comfy",
      show_offline: true,
      show_hidden: false,
      show_system_sounds: true,
    },
    applications: [],
    groups: [makeGroup()],
    scenes: [],
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
    efficiency_mode: false,
    ...overrides,
  };
}
