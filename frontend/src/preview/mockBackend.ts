/**
 * Design-review harness.
 *
 * Serves the IPC surface from in-memory data so the real application code path
 * runs unchanged in a plain browser: `App.svelte` still calls `get_settings`,
 * `list_sessions` and the rest, and gets plausible answers back. Stubbing the
 * transport rather than writing to the stores directly is what makes the
 * screenshots evidence of the real UI instead of a staged copy of it.
 *
 * Loaded only by `preview.html`; nothing here reaches the shipped bundle.
 */
import type { AppSettings, AudioSession, MasterState, MixerSnapshot } from "../lib/api";
import { applyPeaks } from "../lib/stores";

function session(overrides: Partial<AudioSession>): AudioSession {
  return {
    live_id: "endpoint-1::0",
    app_key: "app.exe",
    display_name: "Application",
    executable_name: "app.exe",
    executable_path: "C:\\Program Files\\App\\app.exe",
    process_id: 1000,
    icon: null,
    volume: 0.6,
    muted: false,
    state: "active",
    is_system_sounds: false,
    controllable: true,
    group_id: null,
    ...overrides,
  };
}

/** Covers every row state the design has to hold: audible, grouped, muted,
 *  idle, locked, and the system sounds session. */
const SESSIONS: AudioSession[] = [
  session({
    live_id: "endpoint-1::spotify",
    app_key: "spotify.exe",
    display_name: "Spotify",
    executable_name: "Spotify.exe",
    volume: 0.45,
    group_id: "group-main",
  }),
  session({
    live_id: "endpoint-1::edge",
    app_key: "msedge.exe",
    display_name: "Microsoft Edge",
    executable_name: "msedge.exe",
    volume: 0.8,
    group_id: "group-main",
  }),
  session({
    live_id: "endpoint-1::discord",
    app_key: "discord.exe",
    display_name: "Discord",
    executable_name: "Discord.exe",
    volume: 1,
  }),
  session({
    live_id: "endpoint-1::thorium",
    app_key: "thorium.exe",
    display_name: "Thorium",
    executable_name: "thorium.exe",
    volume: 0.12,
    muted: true,
    group_id: "group-main",
  }),
  session({
    live_id: "endpoint-1::steam",
    app_key: "steam.exe",
    display_name: "Steam",
    executable_name: "steam.exe",
    volume: 0.7,
    state: "inactive",
  }),
  session({
    live_id: "endpoint-1::protected",
    app_key: "securehost.exe",
    display_name: "Secure Host",
    executable_name: "SecureHost.exe",
    volume: 1,
    controllable: false,
  }),
  session({
    live_id: "endpoint-1::system",
    app_key: "system-sounds",
    display_name: "",
    executable_name: null,
    executable_path: null,
    process_id: null,
    volume: 0.6,
    state: "inactive",
    is_system_sounds: true,
  }),
];

const MASTER: MasterState = {
  endpoint_id: "endpoint-1",
  endpoint_name: "Speakers (Realtek High Definition Audio)",
  volume: 0.72,
  muted: false,
};

function baseSettings(): AppSettings {
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
    groups: [
      {
        id: "group-main",
        name: "Main",
        is_default: true,
        volume: 0.625,
        apps: [
          {
            app_key: "msedge.exe",
            display_name: "Microsoft Edge",
            executable_name: "msedge.exe",
            executable_path: "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
          },
          {
            app_key: "thorium.exe",
            display_name: "Thorium",
            executable_name: "thorium.exe",
            executable_path: "C:\\Users\\xt0n1\\AppData\\Local\\Thorium\\Application\\thorium.exe",
          },
          {
            app_key: "spotify.exe",
            display_name: "Spotify",
            executable_name: "Spotify.exe",
            executable_path: "C:\\Users\\xt0n1\\AppData\\Roaming\\Spotify\\Spotify.exe",
          },
        ],
        startup_volume: 0.5,
        auto_mute_on_launch: false,
        hotkeys_enabled: true,
      },
      {
        id: "group-games",
        name: "Games",
        is_default: false,
        volume: 0.85,
        apps: [
          {
            app_key: "steam.exe",
            display_name: "Steam",
            executable_name: "steam.exe",
            executable_path: "C:\\Program Files (x86)\\Steam\\steam.exe",
          },
        ],
        startup_volume: null,
        auto_mute_on_launch: true,
        hotkeys_enabled: false,
      },
    ],
    active_group_id: "group-main",
    hotkeys: [
      { action: "volume_up", accelerator: "AudioVolumeUp" },
      { action: "volume_down", accelerator: "AudioVolumeDown" },
      { action: "mute_toggle", accelerator: "Control+Alt+M" },
    ],
    volume_step: 0.05,
    smart_volume: true,
    launch_on_startup: true,
    start_minimized: false,
    minimize_to_tray: true,
    auto_save: true,
  };
}

export interface PreviewOptions {
  theme: "dark" | "light";
  language: "en" | "es";
}

/**
 * Installs the transport stub. Must run before `App.svelte` is imported so the
 * Tauri modules see the internals during their own initialization.
 */
export function installPreviewBackend(options: PreviewOptions): void {
  const settings = baseSettings();
  settings.ui_prefs.theme = options.theme;
  settings.ui_prefs.language = options.language;

  const snapshot: MixerSnapshot = { master: MASTER, sessions: SESSIONS };

  const respond = async (command: string): Promise<unknown> => {
    switch (command) {
      case "list_sessions":
        return snapshot;
      case "get_settings":
      case "restore_backup":
        return settings;
      case "list_running_apps":
        return SESSIONS.filter((item) => !item.is_system_sounds).map((item) => ({
          app_key: item.app_key,
          display_name: item.display_name,
          executable_name: item.executable_name,
          executable_path: item.executable_path,
        }));
      // The app plugin routes `getVersion` through the same transport.
      case "plugin:app|version":
        return "2.0.0";
      case "plugin:app|name":
        return "NovaMixer";
      case "plugin:os|os_type":
        return "windows";
      case "plugin:os|version":
        return "11";
      default:
        // Mutating commands succeed silently; the UI already applied them
        // optimistically, which is exactly the behaviour under review.
        return null;
    }
  };

  const internals: Record<string, unknown> = {
    invoke: (command: string) => respond(command),
    transformCallback: (callback: unknown) => {
      const id = Math.floor(Math.random() * 1e9);
      (window as unknown as Record<string, unknown>)[`_${id}`] = callback;
      return id;
    },
    convertFileSrc: (source: string) => source,
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    },
    plugins: {},
  };

  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
  (globalThis as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
  (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: () => undefined,
  };
}

/** Starts the metering loop so the VU bars carry believable, varied levels. */
export function startPreviewMetering(): void {
  // Each app gets its own rate and depth, so a screenshot shows genuinely
  // different levels rather than a row of identical bars.
  const shapes = [
    { liveId: "endpoint-1::spotify", rate: 0.9, base: 0.55, depth: 0.35 },
    { liveId: "endpoint-1::edge", rate: 1.7, base: 0.35, depth: 0.25 },
    { liveId: "endpoint-1::discord", rate: 0.4, base: 0.22, depth: 0.18 },
    { liveId: "endpoint-1::protected", rate: 1.2, base: 0.75, depth: 0.22 },
  ];

  let tick = 0;
  const advance = (): void => {
    tick += 1;
    const t = tick / 20;
    applyPeaks({
      timestamp_ms: tick * 50,
      master_peak: 0.55 + Math.sin(t * 1.1) * 0.3,
      sessions: shapes.map((shape) => ({
        live_id: shape.liveId,
        peak: Math.max(0, shape.base + Math.sin(t * shape.rate) * shape.depth),
      })),
    });
  };

  advance();
  window.setInterval(advance, 50);
}
