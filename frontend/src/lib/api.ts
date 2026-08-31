/**
 * Typed frontend view of the IPC surface defined in `contracts/ipc.md`.
 *
 * Every call goes through the generated transport so command names stay owned by
 * the Rust `generate_handler!` registry. Nothing in this file may import `invoke`
 * directly — `cargo xtask check-architecture` fails the build if it does.
 */
import { invokeCommand as transport, COMMANDS } from "../generated/bindings";

/* ── Session domain ─────────────────────────────────────────────────────── */

export type SessionState = "active" | "inactive" | "expired";

export interface AudioSession {
  /** `"{endpoint_id}::{session_instance_id}"`. Stable while the session lives, never persisted. */
  live_id: string;
  /** Durable identity: AUMID, else canonical exe path, else exe file name. */
  app_key: string;
  display_name: string;
  executable_name: string | null;
  executable_path: string | null;
  process_id: number | null;
  /** 32x32 PNG data URL, or null when no icon could be extracted. */
  icon: string | null;
  /** Linear amplitude scalar, 0.0–1.0. */
  volume: number;
  muted: boolean;
  state: SessionState;
  is_system_sounds: boolean;
  /** False when the OS rejects volume calls for this session. The slider is disabled. */
  controllable: boolean;
  group_id: string | null;
}

export interface MasterState {
  endpoint_id: string;
  endpoint_name: string;
  volume: number;
  muted: boolean;
}

export interface MixerSnapshot {
  master: MasterState;
  sessions: AudioSession[];
}

export interface SessionPeak {
  live_id: string;
  peak: number;
}

export interface PeakBatch {
  timestamp_ms: number;
  master_peak: number;
  sessions: SessionPeak[];
}

/* ── Groups ─────────────────────────────────────────────────────────────── */

export interface AppBinding {
  app_key: string;
  display_name: string;
  executable_name: string | null;
  executable_path: string | null;
}

export interface Group {
  id: string;
  name: string;
  is_default: boolean;
  volume: number;
  apps: AppBinding[];
  /** Applied when a bound app's session is created. Null disables it. */
  startup_volume: number | null;
  /** Mutes a bound app on session creation. Takes precedence over `startup_volume`. */
  auto_mute_on_launch: boolean;
  hotkeys_enabled: boolean;
}

/* ── Settings ───────────────────────────────────────────────────────────── */

export type HotkeyAction = "volume_up" | "volume_down" | "mute_toggle";

export interface HotkeyBinding {
  action: HotkeyAction;
  /** Tauri accelerator, e.g. `"MediaVolumeUp"`. Null unbinds the action. */
  accelerator: string | null;
}

export type Theme = "dark" | "light";
export type Density = "compact" | "comfy";

export interface UiPrefs {
  theme: Theme;
  language: string;
  sidebar_collapsed: boolean;
  density: Density;
  show_inactive: boolean;
  show_system_sounds: boolean;
}

export interface AppSettings {
  schema_version: number;
  ui_prefs: UiPrefs;
  groups: Group[];
  active_group_id: string | null;
  hotkeys: HotkeyBinding[];
  /** Hotkey step as a scalar, e.g. 0.05 for five percent. */
  volume_step: number;
  smart_volume: boolean;
  launch_on_startup: boolean;
  start_minimized: boolean;
  minimize_to_tray: boolean;
  auto_save: boolean;
}

/* ── Errors ─────────────────────────────────────────────────────────────── */

export type ApiErrorKind =
  | "audio_unavailable"
  | "session_gone"
  | "not_controllable"
  | "validation"
  | "io"
  | "other";

export interface ApiError {
  kind: ApiErrorKind;
  message: string;
}

/** Narrows an unknown rejection to the backend's error envelope. */
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

/* ── Event names ────────────────────────────────────────────────────────── */

export const EVENTS = {
  sessionAdded: "session-added",
  sessionUpdated: "session-updated",
  sessionRemoved: "session-removed",
  masterUpdated: "master-updated",
  endpointChanged: "endpoint-changed",
  peaks: "peaks",
  settingsUpdated: "settings-updated",
  hotkeyFired: "hotkey-fired",
} as const;

export interface SessionRemovedPayload {
  live_id: string;
}

export interface HotkeyFiredPayload {
  action: HotkeyAction;
}

/* ── Commands ───────────────────────────────────────────────────────────── */

export async function listSessions(): Promise<MixerSnapshot> {
  return transport(COMMANDS.list_sessions);
}

export async function setSessionVolume(liveId: string, volume: number): Promise<void> {
  return transport(COMMANDS.set_session_volume, { liveId, volume });
}

export async function setSessionMute(liveId: string, muted: boolean): Promise<void> {
  return transport(COMMANDS.set_session_mute, { liveId, muted });
}

export async function setMasterVolume(volume: number): Promise<void> {
  return transport(COMMANDS.set_master_volume, { volume });
}

export async function setMasterMute(muted: boolean): Promise<void> {
  return transport(COMMANDS.set_master_mute, { muted });
}

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

export async function listRunningApps(): Promise<AppBinding[]> {
  return transport(COMMANDS.list_running_apps);
}

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
