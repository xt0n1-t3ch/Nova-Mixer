import { derived, get, writable, type Readable, type Writable } from "svelte/store";
import {
  deleteGroup as ipcDeleteGroup,
  errorMessage,
  getSettings,
  listSessions,
  restoreBackup as ipcRestoreBackup,
  saveSettings,
  setActiveGroup as ipcSetActiveGroup,
  setGroupVolume as ipcSetGroupVolume,
  setHotkeys as ipcSetHotkeys,
  setMasterMute,
  setMasterVolume,
  setSessionMute,
  setSessionVolume,
  upsertGroup as ipcUpsertGroup,
  type AppSettings,
  type AudioSession,
  type Group,
  type HotkeyBinding,
  type MasterState,
  type PeakBatch,
} from "./api";
import { debounce, matchesQuery, type ViewId } from "./ux";

/* ── Navigation and chrome ──────────────────────────────────────────────── */

export const currentView: Writable<ViewId> = writable("mixer");
export const searchQuery: Writable<string> = writable("");
export const selectedGroupId: Writable<string | null> = writable(null);
export const shortcutOverlayOpen: Writable<boolean> = writable(false);

/* ── Audio state ────────────────────────────────────────────────────────── */

export const master: Writable<MasterState | null> = writable(null);
export const sessions: Writable<AudioSession[]> = writable([]);
export const audioAvailable: Writable<boolean> = writable(true);
export const settings: Writable<AppSettings | null> = writable(null);

/**
 * Peaks live outside `sessions` on purpose. They change 20 times a second and
 * would otherwise invalidate every row's props on every tick, re-rendering the
 * whole list instead of only the meters.
 */
export const peaks: Writable<Record<string, number>> = writable({});
export const masterPeak: Writable<number> = writable(0);

/**
 * Volumes the user is dragging right now. The UI reads from here so a slider
 * never jumps back when a `session-updated` event arrives mid-drag carrying the
 * value from two frames ago.
 */
export const pendingVolumes: Writable<Record<string, number>> = writable({});

export const groups: Readable<Group[]> = derived(settings, ($settings) => $settings?.groups ?? []);

export const activeGroupId: Readable<string | null> = derived(
  settings,
  ($settings) => $settings?.active_group_id ?? null,
);

export const activeGroup: Readable<Group | null> = derived(
  [groups, activeGroupId],
  ([$groups, $activeId]) => $groups.find((group) => group.id === $activeId) ?? null,
);

/** Sessions after the user's visibility preferences and the search box. */
export const visibleSessions: Readable<AudioSession[]> = derived(
  [sessions, settings, searchQuery],
  ([$sessions, $settings, $query]) => {
    const prefs = $settings?.ui_prefs;
    return $sessions.filter((session) => {
      if (session.is_system_sounds && prefs && !prefs.show_system_sounds) return false;
      if (session.state === "inactive" && prefs && !prefs.show_inactive) return false;
      return matchesQuery($query, [
        session.display_name,
        session.executable_name,
        session.executable_path,
      ]);
    });
  },
);

/**
 * Display order. Sorting by live peak would make rows swap places while music
 * plays, so ordering uses stable facts only: audible before idle, then name.
 */
export function sortSessions(list: AudioSession[]): AudioSession[] {
  return [...list].sort((a, b) => {
    if (a.is_system_sounds !== b.is_system_sounds) return a.is_system_sounds ? 1 : -1;
    if ((a.state === "active") !== (b.state === "active")) return a.state === "active" ? -1 : 1;
    return a.display_name.localeCompare(b.display_name, undefined, { sensitivity: "base" });
  });
}

export const sortedSessions: Readable<AudioSession[]> = derived(visibleSessions, sortSessions);

/* ── Toasts ─────────────────────────────────────────────────────────────── */

export interface ToastMessage {
  id: number;
  text: string;
  tone: "info" | "success" | "danger";
}

export const toasts: Writable<ToastMessage[]> = writable([]);
let toastSeq = 0;

export function pushToast(text: string, tone: ToastMessage["tone"] = "info"): void {
  const id = ++toastSeq;
  toasts.update((list) => [...list, { id, text, tone }]);
  setTimeout(() => dismissToast(id), tone === "danger" ? 6000 : 3200);
}

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((toast) => toast.id !== id));
}

/* ── Session mutations ──────────────────────────────────────────────────── */

/** Optimistic paint during a drag. No IPC — `commitSessionVolume` sends it. */
export function previewSessionVolume(liveId: string, volume: number): void {
  pendingVolumes.update((map) => ({ ...map, [liveId]: volume }));
}

export async function commitSessionVolume(liveId: string, volume: number): Promise<void> {
  previewSessionVolume(liveId, volume);
  try {
    await setSessionVolume(liveId, volume);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    // Drop the optimistic value so the next backend event wins.
    pendingVolumes.update(({ [liveId]: _dropped, ...rest }) => rest);
  }
}

export async function toggleSessionMute(session: AudioSession): Promise<void> {
  const next = !session.muted;
  sessions.update((list) =>
    list.map((item) => (item.live_id === session.live_id ? { ...item, muted: next } : item)),
  );
  try {
    await setSessionMute(session.live_id, next);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    sessions.update((list) =>
      list.map((item) =>
        item.live_id === session.live_id ? { ...item, muted: session.muted } : item,
      ),
    );
  }
}

export function previewMasterVolume(volume: number): void {
  master.update((state) => (state ? { ...state, volume } : state));
}

export async function commitMasterVolume(volume: number): Promise<void> {
  previewMasterVolume(volume);
  try {
    await setMasterVolume(volume);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

export async function toggleMasterMute(): Promise<void> {
  const state = get(master);
  if (!state) return;
  const next = !state.muted;
  master.set({ ...state, muted: next });
  try {
    await setMasterMute(next);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    master.set(state);
  }
}

/* ── Group mutations ────────────────────────────────────────────────────── */

export async function commitGroupVolume(groupId: string, volume: number): Promise<void> {
  settings.update((current) =>
    current
      ? {
          ...current,
          groups: current.groups.map((group) =>
            group.id === groupId ? { ...group, volume } : group,
          ),
        }
      : current,
  );
  try {
    await ipcSetGroupVolume(groupId, volume);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

export async function saveGroup(group: Group): Promise<Group | null> {
  try {
    const saved = await ipcUpsertGroup(group);
    settings.update((current) => {
      if (!current) return current;
      const exists = current.groups.some((item) => item.id === saved.id);
      return {
        ...current,
        groups: exists
          ? current.groups.map((item) => (item.id === saved.id ? saved : item))
          : [...current.groups, saved],
      };
    });
    return saved;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return null;
  }
}

export async function removeGroup(groupId: string): Promise<boolean> {
  try {
    await ipcDeleteGroup(groupId);
    settings.update((current) =>
      current
        ? { ...current, groups: current.groups.filter((group) => group.id !== groupId) }
        : current,
    );
    if (get(selectedGroupId) === groupId) selectedGroupId.set(null);
    return true;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return false;
  }
}

export async function activateGroup(groupId: string | null): Promise<void> {
  try {
    await ipcSetActiveGroup(groupId);
    settings.update((current) => (current ? { ...current, active_group_id: groupId } : current));
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

export async function applyHotkeys(hotkeys: HotkeyBinding[]): Promise<void> {
  try {
    await ipcSetHotkeys(hotkeys);
    settings.update((current) => (current ? { ...current, hotkeys } : current));
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

/* ── Settings persistence ───────────────────────────────────────────────── */

const persistDebounced = debounce((next: AppSettings) => {
  void saveSettings(next).catch((error) => pushToast(errorMessage(error), "danger"));
}, 500);

/**
 * Updates settings locally and schedules a write. Autosave off means the caller
 * must invoke `flushSettings` explicitly, which is what the Save button does.
 */
export function persistSettings(next: AppSettings): void {
  settings.set(next);
  if (next.auto_save) {
    persistDebounced(next);
  }
}

export async function flushSettings(): Promise<void> {
  persistDebounced.cancel();
  const current = get(settings);
  if (!current) return;
  try {
    await saveSettings(current);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

export async function loadSettings(): Promise<void> {
  try {
    settings.set(await getSettings());
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

export async function restoreSettingsBackup(): Promise<boolean> {
  try {
    settings.set(await ipcRestoreBackup());
    return true;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return false;
  }
}

/* ── Snapshot loading ───────────────────────────────────────────────────── */

export async function refreshMixer(): Promise<void> {
  try {
    const snapshot = await listSessions();
    master.set(snapshot.master);
    sessions.set(snapshot.sessions);
    pendingVolumes.set({});
    audioAvailable.set(true);
  } catch {
    audioAvailable.set(false);
  }
}

/* ── Event application ──────────────────────────────────────────────────── */
/* These are pure state transitions so the integration tests can drive them
   without a Tauri runtime. `events.ts` wires them to the real listeners. */

export function applySessionAdded(session: AudioSession): void {
  sessions.update((list) =>
    list.some((item) => item.live_id === session.live_id) ? list : [...list, session],
  );
}

export function applySessionUpdated(session: AudioSession): void {
  sessions.update((list) =>
    list.map((item) => (item.live_id === session.live_id ? session : item)),
  );
  // The authoritative value has landed, so the optimistic one is obsolete.
  pendingVolumes.update(({ [session.live_id]: _dropped, ...rest }) => rest);
}

export function applySessionRemoved(liveId: string): void {
  sessions.update((list) => list.filter((item) => item.live_id !== liveId));
  peaks.update(({ [liveId]: _dropped, ...rest }) => rest);
  pendingVolumes.update(({ [liveId]: _dropped, ...rest }) => rest);
}

export function applyMasterUpdated(state: MasterState): void {
  master.set(state);
}

/** An endpoint swap replaces the world; merging would keep dead sessions. */
export function applyEndpointChanged(snapshot: {
  master: MasterState;
  sessions: AudioSession[];
}): void {
  master.set(snapshot.master);
  sessions.set(snapshot.sessions);
  peaks.set({});
  pendingVolumes.set({});
  audioAvailable.set(true);
}

export function applyPeaks(batch: PeakBatch): void {
  const next: Record<string, number> = {};
  for (const entry of batch.sessions) next[entry.live_id] = entry.peak;
  peaks.set(next);
  masterPeak.set(batch.master_peak);
}
