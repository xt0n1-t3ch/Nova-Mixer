/**
 * Typed frontend view of the IPC surface defined in `contracts/ipc.md` (v2).
 *
 * Every call goes through the generated transport so command names stay owned by
 * the Rust `generate_handler!` registry. Nothing in this file may import `invoke`
 * directly — `cargo xtask check-architecture` fails the build if it does.
 */
import { invokeCommand as transport, COMMANDS } from "../generated/bindings";

/* ── Applications ───────────────────────────────────────────────────────── */

export type IdentityKind = "aumid" | "path" | "filename";

/**
 * The persistent, user-facing object: one row in the Applications view.
 *
 * Windows hands one application several audio sessions whenever it likes, which
 * is why the previous version rendered Discord twice and lost a row the moment
 * its session expired. An `Application` outlives its sessions and owns them.
 */
export interface Application {
  /** Durable identity, stable across restarts. */
  app_key: string;
  identity_kind: IdentityKind;
  /** `custom_name` when set, else the discovered name. */
  display_name: string;
  custom_name: string | null;
  executable_name: string | null;
  executable_path: string | null;
  /** 32x32 PNG data URL, or null when no icon could be extracted. */
  icon: string | null;
  /** Linear amplitude scalar, 0.0–1.0, applied to every live session. */
  volume: number;
  muted: boolean;
  /** Live sessions disagree and no policy has been applied; show indeterminate. */
  mixed: boolean;
  /** NovaMixer reapplies volume and mute to sessions created later. */
  remembered: boolean;
  pinned: boolean;
  hidden: boolean;
  sort_order: number;
  /** Owns at least one live session. */
  running: boolean;
  /** False when every live session refuses control. */
  controllable: boolean;
  is_system_sounds: boolean;
  group_id: string | null;
  sessions: AudioSession[];
  /** Highest child peak. Never a sum — that would read as clipping. */
  peak: number;
}

export type SessionState = "active" | "inactive" | "expired";

/** One live Windows session. A child of an application, never a top-level row. */
export interface AudioSession {
  /** `"{endpoint_id}::{session_instance_id}"`. Never persisted. */
  live_id: string;
  app_key: string;
  display_name: string;
  process_id: number | null;
  volume: number;
  muted: boolean;
  state: SessionState;
  controllable: boolean;
  peak: number;
}

export interface MasterState {
  endpoint_id: string;
  endpoint_name: string;
  volume: number;
  muted: boolean;
  peak: number;
}

export interface MixerSnapshot {
  master: MasterState;
  applications: Application[];
}

export interface AudioDevice {
  id: string;
  name: string;
  is_default: boolean;
}

/** An application the user can add, shown in the picker. */
export interface AppCandidate {
  app_key: string;
  display_name: string;
  executable_name: string | null;
  executable_path: string | null;
  icon: string | null;
  /** Currently owns a session. Running candidates sort first. */
  running: boolean;
  /** Already added; shown but not selectable. */
  already_managed: boolean;
}

/** Every field optional; an omitted field is unchanged. */
export interface ApplicationPatch {
  custom_name?: string | null;
  remembered?: boolean;
  pinned?: boolean;
  hidden?: boolean;
  group_id?: string | null;
}

/* ── Metering ───────────────────────────────────────────────────────────── */

export interface SessionPeak {
  live_id: string;
  peak: number;
}

export interface AppPeak {
  app_key: string;
  peak: number;
  sessions: SessionPeak[];
}

export interface PeakBatch {
  timestamp_ms: number;
  master_peak: number;
  applications: AppPeak[];
}

/* ── Scenes and groups ──────────────────────────────────────────────────── */

export interface SceneEntry {
  app_key: string;
  /** Null leaves the level alone. */
  volume: number | null;
  /** Null leaves mute alone. */
  muted: boolean | null;
}

export interface Scene {
  id: string;
  name: string;
  /** Lucide icon key. */
  icon: string | null;
  master_volume: number | null;
  entries: SceneEntry[];
  /** Ramp duration when applying. 0 is instant. */
  fade_ms: number;
}

export interface Group {
  id: string;
  name: string;
  is_default: boolean;
  volume: number;
  app_keys: string[];
  startup_volume: number | null;
  auto_mute_on_launch: boolean;
  hotkeys_enabled: boolean;
}

/* ── Settings ───────────────────────────────────────────────────────────── */

export type HotkeyAction = "volume_up" | "volume_down" | "mute_toggle";

export interface HotkeyBinding {
  action: HotkeyAction;
  /** Tauri accelerator, e.g. `"AudioVolumeUp"`. Null unbinds the action. */
  accelerator: string | null;
}

export type Theme = "dark" | "light";
export type Density = "compact" | "comfy";

export interface UiPrefs {
  theme: Theme;
  language: string;
  sidebar_collapsed: boolean;
  density: Density;
  /** Show applications that are not currently running. */
  show_offline: boolean;
  show_hidden: boolean;
  show_system_sounds: boolean;
}

export interface AppSettings {
  schema_version: number;
  ui_prefs: UiPrefs;
  applications: Application[];
  groups: Group[];
  scenes: Scene[];
  active_group_id: string | null;
  hotkeys: HotkeyBinding[];
  /** Hotkey step as a scalar, e.g. 0.05 for five percent. */
  volume_step: number;
  smart_volume: boolean;
  launch_on_startup: boolean;
  start_minimized: boolean;
  minimize_to_tray: boolean;
  auto_save: boolean;
  efficiency_mode: boolean;
}

export interface EfficiencyStatus {
  /** False on a Windows build without EcoQoS. */
  supported: boolean;
  enabled: boolean;
  /** Why it is unsupported, or why applying failed. */
  detail: string | null;
}

/* ── Errors ─────────────────────────────────────────────────────────────── */

export type ApiErrorKind =
  | "audio_unavailable"
  | "session_gone"
  | "app_unknown"
  | "not_controllable"
  | "unsupported"
  | "validation"
  | "io"
  | "other";

export interface ApiError {
  kind: ApiErrorKind;
  message: string;
}

export function isApiError(value: unknown): value is ApiError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as ApiError).kind === "string" &&
    typeof (value as ApiError).message === "string"
  );
}

/** A message safe to show a user, whatever the rejection actually was. */
export function errorMessage(value: unknown): string {
  if (isApiError(value)) return value.message;
  if (value instanceof Error) return value.message;
  return String(value);
}

/* ── Events ─────────────────────────────────────────────────────────────── */

export const EVENTS = {
  applicationAdded: "application-added",
  applicationUpdated: "application-updated",
  applicationRemoved: "application-removed",
  masterUpdated: "master-updated",
  endpointChanged: "endpoint-changed",
  peaks: "peaks",
  settingsUpdated: "settings-updated",
  hotkeyFired: "hotkey-fired",
  sceneApplied: "scene-applied",
} as const;

export interface ApplicationRemovedPayload {
  app_key: string;
}

export interface HotkeyFiredPayload {
  action: HotkeyAction;
}

export interface SceneAppliedPayload {
  scene_id: string;
}

/* ── Application commands ───────────────────────────────────────────────── */

export async function listApplications(): Promise<MixerSnapshot> {
  return transport(COMMANDS.list_applications);
}

export async function setAppVolume(appKey: string, volume: number): Promise<void> {
  return transport(COMMANDS.set_app_volume, { appKey, volume });
}

export async function setAppMute(appKey: string, muted: boolean): Promise<void> {
  return transport(COMMANDS.set_app_mute, { appKey, muted });
}

export async function setSessionVolume(liveId: string, volume: number): Promise<void> {
  return transport(COMMANDS.set_session_volume, { liveId, volume });
}

export async function setSessionMute(liveId: string, muted: boolean): Promise<void> {
  return transport(COMMANDS.set_session_mute, { liveId, muted });
}

export async function addApplication(path: string): Promise<Application> {
  return transport(COMMANDS.add_application, { path });
}

/** Forgets NovaMixer's settings for this application. Never closes or uninstalls it. */
export async function removeApplication(appKey: string): Promise<void> {
  return transport(COMMANDS.remove_application, { appKey });
}

export async function updateApplication(
  appKey: string,
  patch: ApplicationPatch,
): Promise<Application> {
  return transport(COMMANDS.update_application, { appKey, patch });
}

export async function reorderApplications(appKeys: string[]): Promise<void> {
  return transport(COMMANDS.reorder_applications, { appKeys });
}

export async function listAppCandidates(): Promise<AppCandidate[]> {
  return transport(COMMANDS.list_app_candidates);
}

/* ── Master and devices ─────────────────────────────────────────────────── */

export async function setMasterVolume(volume: number): Promise<void> {
  return transport(COMMANDS.set_master_volume, { volume });
}

export async function setMasterMute(muted: boolean): Promise<void> {
  return transport(COMMANDS.set_master_mute, { muted });
}

export async function listOutputDevices(): Promise<AudioDevice[]> {
  return transport(COMMANDS.list_output_devices);
}

export async function setDefaultOutput(deviceId: string): Promise<void> {
  return transport(COMMANDS.set_default_output, { deviceId });
}

/* ── Groups and scenes ──────────────────────────────────────────────────── */

export async function setGroupVolume(groupId: string, volume: number): Promise<void> {
  return transport(COMMANDS.set_group_volume, { groupId, volume });
}

export async function setActiveGroup(groupId: string | null): Promise<void> {
  return transport(COMMANDS.set_active_group, { groupId });
}

export async function upsertGroup(group: Group): Promise<Group> {
  return transport(COMMANDS.upsert_group, { group });
}

export async function deleteGroup(groupId: string): Promise<void> {
  return transport(COMMANDS.delete_group, { groupId });
}

export async function upsertScene(scene: Scene): Promise<Scene> {
  return transport(COMMANDS.upsert_scene, { scene });
}

export async function deleteScene(sceneId: string): Promise<void> {
  return transport(COMMANDS.delete_scene, { sceneId });
}

export async function applyScene(sceneId: string): Promise<void> {
  return transport(COMMANDS.apply_scene, { sceneId });
}

/** Snapshots every managed application's current level into a new scene. */
export async function captureScene(name: string): Promise<Scene> {
  return transport(COMMANDS.capture_scene, { name });
}

/* ── Settings and system ────────────────────────────────────────────────── */

export async function getSettings(): Promise<AppSettings> {
  return transport(COMMANDS.get_settings);
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return transport(COMMANDS.save_settings, { settings });
}

export async function restoreBackup(): Promise<AppSettings> {
  return transport(COMMANDS.restore_backup);
}

export async function setHotkeys(hotkeys: HotkeyBinding[]): Promise<void> {
  return transport(COMMANDS.set_hotkeys, { hotkeys });
}

export async function setEfficiencyMode(enabled: boolean): Promise<EfficiencyStatus> {
  return transport(COMMANDS.set_efficiency_mode, { enabled });
}

export async function getEfficiencyStatus(): Promise<EfficiencyStatus> {
  return transport(COMMANDS.get_efficiency_status);
}

export async function openDataFolder(): Promise<void> {
  return transport(COMMANDS.open_data_folder);
}

/** Drops peak polling to the idle rate when the mixer is not visible. */
export async function setMeteringActive(active: boolean): Promise<void> {
  return transport(COMMANDS.set_metering_active, { active });
}

export async function openDevtools(): Promise<void> {
  return transport(COMMANDS.open_devtools);
}
