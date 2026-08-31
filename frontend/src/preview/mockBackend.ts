/**
 * Design-review harness.
 *
 * Serves the IPC surface from in-memory data so the real application code path
 * runs unchanged in a plain browser: `App.svelte` still calls `get_settings`,
 * `list_applications` and the rest, and gets plausible answers back. Stubbing
 * the transport rather than writing to the stores directly is what makes the
 * screenshots evidence of the real interface instead of a staged copy of it.
 *
 * Loaded only by `preview.html`; nothing here reaches the shipped bundle.
 */
import type {
  AppCandidate,
  Application,
  AppSettings,
  AudioSession,
  MasterState,
  MixerSnapshot,
} from "../lib/api";
import { applyPeaks } from "../lib/stores";

function session(overrides: Partial<AudioSession>): AudioSession {
  return {
    live_id: "endpoint-1::0",
    app_key: "app.exe",
    display_name: "Stream",
    process_id: 1000,
    volume: 0.6,
    muted: false,
    state: "active",
    controllable: true,
    peak: 0,
    ...overrides,
  };
}

function app(overrides: Partial<Application>): Application {
  return {
    app_key: "app.exe",
    identity_kind: "path",
    display_name: "Application",
    custom_name: null,
    executable_name: "app.exe",
    executable_path: "C:\\Program Files\\App\\app.exe",
    icon: null,
    volume: 0.6,
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

/**
 * Covers every state the design has to hold: pinned, remembered, several
 * sessions collapsed into one row, muted, closed-but-configured, locked, and
 * system sounds.
 */
const APPLICATIONS: Application[] = [
  app({
    app_key: "spotify.exe",
    display_name: "Spotify",
    executable_name: "Spotify.exe",
    executable_path: "C:\\Users\\xt0n1\\AppData\\Roaming\\Spotify\\Spotify.exe",
    volume: 0.45,
    remembered: true,
    pinned: true,
    sort_order: 0,
    group_id: "group-main",
    sessions: [session({ live_id: "endpoint-1::spotify", app_key: "spotify.exe", volume: 0.45 })],
  }),
  app({
    app_key: "discord.exe",
    display_name: "Discord",
    executable_name: "Discord.exe",
    executable_path: "C:\\Users\\xt0n1\\AppData\\Local\\Discord\\Discord.exe",
    volume: 0.8,
    remembered: true,
    sort_order: 1,
    sessions: [
      session({
        live_id: "endpoint-1::discord-voice",
        app_key: "discord.exe",
        display_name: "Voice",
        process_id: 9452,
        volume: 0.8,
      }),
      session({
        live_id: "endpoint-1::discord-app",
        app_key: "discord.exe",
        display_name: "Notifications",
        process_id: 9453,
        volume: 0.8,
        state: "inactive",
      }),
    ],
  }),
  app({
    app_key: "msedge.exe",
    display_name: "Microsoft Edge",
    executable_name: "msedge.exe",
    executable_path: "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
    volume: 0.62,
    sort_order: 2,
    group_id: "group-main",
    sessions: [session({ live_id: "endpoint-1::edge", app_key: "msedge.exe", volume: 0.62 })],
  }),
  app({
    app_key: "thorium.exe",
    display_name: "Thorium",
    executable_name: "thorium.exe",
    volume: 0.12,
    muted: true,
    sort_order: 3,
    group_id: "group-main",
    sessions: [
      session({
        live_id: "endpoint-1::thorium",
        app_key: "thorium.exe",
        volume: 0.12,
        muted: true,
      }),
    ],
  }),
  app({
    app_key: "securehost.exe",
    display_name: "Secure Host",
    executable_name: "SecureHost.exe",
    volume: 1,
    controllable: false,
    sort_order: 4,
    sessions: [
      session({ live_id: "endpoint-1::secure", app_key: "securehost.exe", controllable: false }),
    ],
  }),
  app({
    app_key: "steam.exe",
    display_name: "Steam",
    executable_name: "steam.exe",
    executable_path: "C:\\Program Files (x86)\\Steam\\steam.exe",
    volume: 0.7,
    remembered: true,
    running: false,
    sort_order: 5,
    sessions: [],
  }),
  app({
    app_key: "system-sounds",
    display_name: "System Sounds",
    executable_name: null,
    executable_path: null,
    identity_kind: "filename",
    volume: 0.6,
    is_system_sounds: true,
    sort_order: 6,
    sessions: [
      session({
        live_id: "endpoint-1::system",
        app_key: "system-sounds",
        display_name: "System Sounds",
        process_id: null,
        state: "inactive",
      }),
    ],
  }),
];

const MASTER: MasterState = {
  endpoint_id: "endpoint-1",
  endpoint_name: "Headphones (MSI IMMERSE GH50 WIRELESS)",
  volume: 0.72,
  muted: false,
  peak: 0,
};

const CANDIDATES: AppCandidate[] = APPLICATIONS.filter((item) => !item.is_system_sounds).map(
  (item) => ({
    app_key: item.app_key,
    display_name: item.display_name,
    executable_name: item.executable_name,
    executable_path: item.executable_path,
    icon: item.icon,
    running: item.running,
    already_managed: true,
  }),
);

function baseSettings(): AppSettings {
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
    applications: APPLICATIONS,
    groups: [
      {
        id: "group-main",
        name: "Main",
        is_default: true,
        volume: 0.625,
        app_keys: ["msedge.exe", "thorium.exe", "spotify.exe"],
        startup_volume: 0.5,
        auto_mute_on_launch: false,
        hotkeys_enabled: true,
      },
      {
        id: "group-games",
        name: "Games",
        is_default: false,
        volume: 0.85,
        app_keys: ["steam.exe"],
        startup_volume: null,
        auto_mute_on_launch: true,
        hotkeys_enabled: false,
      },
    ],
    scenes: [
      {
        id: "scene-gaming",
        name: "Gaming",
        icon: null,
        master_volume: 0.8,
        entries: [],
        fade_ms: 250,
      },
      {
        id: "scene-focus",
        name: "Focus",
        icon: null,
        master_volume: 0.5,
        entries: [],
        fade_ms: 400,
      },
      {
        id: "scene-call",
        name: "Call",
        icon: null,
        master_volume: 0.6,
        entries: [],
        fade_ms: 150,
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
    efficiency_mode: true,
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

  const snapshot: MixerSnapshot = { master: MASTER, applications: APPLICATIONS };

  const respond = async (command: string): Promise<unknown> => {
    switch (command) {
      case "list_applications":
        return snapshot;
      case "get_settings":
      case "restore_backup":
        return settings;
      case "list_app_candidates":
        return CANDIDATES;
      case "list_output_devices":
        return [
          { id: "endpoint-1", name: MASTER.endpoint_name, is_default: true },
          { id: "endpoint-2", name: "Speakers (Realtek High Definition Audio)", is_default: false },
        ];
      case "get_efficiency_status":
      case "set_efficiency_mode":
        return { supported: true, enabled: true, detail: null };
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
        // Mutating commands succeed silently; the interface already applied them
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

/** Starts the metering loop so the level meters carry believable, varied levels. */
export function startPreviewMetering(): void {
  // Each application gets its own rate and depth, so a screenshot shows
  // genuinely different levels rather than a row of identical bars.
  const shapes = [
    { key: "spotify.exe", live: "endpoint-1::spotify", rate: 0.9, base: 0.55, depth: 0.35 },
    { key: "discord.exe", live: "endpoint-1::discord-voice", rate: 1.7, base: 0.4, depth: 0.28 },
    { key: "msedge.exe", live: "endpoint-1::edge", rate: 0.4, base: 0.25, depth: 0.2 },
    { key: "securehost.exe", live: "endpoint-1::secure", rate: 1.2, base: 0.72, depth: 0.22 },
  ];

  let tick = 0;
  const advance = (): void => {
    tick += 1;
    const t = tick / 20;
    applyPeaks({
      timestamp_ms: tick * 50,
      master_peak: 0.55 + Math.sin(t * 1.1) * 0.3,
      applications: shapes.map((shape) => {
        const peak = Math.max(0, shape.base + Math.sin(t * shape.rate) * shape.depth);
        return {
          app_key: shape.key,
          peak,
          sessions: [{ live_id: shape.live, peak }],
        };
      }),
    });
  };

  advance();
  window.setInterval(advance, 50);
}
