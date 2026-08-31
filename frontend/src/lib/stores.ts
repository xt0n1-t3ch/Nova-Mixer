import { derived, get, writable, type Readable, type Writable } from "svelte/store";
import {
  addApplication as ipcAddApplication,
  applyScene as ipcApplyScene,
  captureScene as ipcCaptureScene,
  deleteGroup as ipcDeleteGroup,
  deleteScene as ipcDeleteScene,
  errorMessage,
  getSettings,
  listApplications,
  removeApplication as ipcRemoveApplication,
  reorderApplications as ipcReorderApplications,
  restoreBackup as ipcRestoreBackup,
  saveSettings,
  setActiveGroup as ipcSetActiveGroup,
  setAppMute,
  setAppVolume,
  setGroupVolume as ipcSetGroupVolume,
  setHotkeys as ipcSetHotkeys,
  setMasterMute,
  setMasterVolume,
  setSessionMute,
  setSessionVolume,
  updateApplication as ipcUpdateApplication,
  upsertGroup as ipcUpsertGroup,
  upsertScene as ipcUpsertScene,
  type Application,
  type ApplicationPatch,
  type AppSettings,
  type Group,
  type HotkeyBinding,
  type MasterState,
  type PeakBatch,
  type Scene,
} from "./api";
import { debounce, matchesQuery, type ViewId } from "./ux";

/* ── Navigation and chrome ──────────────────────────────────────────────── */

export const currentView: Writable<ViewId> = writable("applications");
export const searchQuery: Writable<string> = writable("");
export const selectedGroupId: Writable<string | null> = writable(null);
export const shortcutOverlayOpen: Writable<boolean> = writable(false);
export const commandPaletteOpen: Writable<boolean> = writable(false);

/** Application whose detail panel is open, or null. */
export const inspectedAppKey: Writable<string | null> = writable(null);

/** Applications whose session breakdown is expanded. */
export const expandedApps: Writable<Set<string>> = writable(new Set());

export function toggleExpanded(appKey: string): void {
  expandedApps.update((current) => {
    const next = new Set(current);
    if (next.has(appKey)) {
      next.delete(appKey);
    } else {
      next.add(appKey);
    }
    return next;
  });
}

/* ── Audio state ────────────────────────────────────────────────────────── */

export const master: Writable<MasterState | null> = writable(null);
export const applications: Writable<Application[]> = writable([]);
export const audioAvailable: Writable<boolean> = writable(true);
export const settings: Writable<AppSettings | null> = writable(null);

/**
 * Peaks live outside `applications` on purpose. They change twenty times a
 * second and would otherwise invalidate every row's props on every tick,
 * re-rendering the whole list instead of only the meters.
 */
export const appPeaks: Writable<Record<string, number>> = writable({});
export const sessionPeaks: Writable<Record<string, number>> = writable({});
export const masterPeak: Writable<number> = writable(0);

/**
 * Volumes the user is dragging right now. The interface reads from here so a
 * slider never jumps back when an `application-updated` event arrives mid-drag
 * carrying the value from two frames ago.
 */
export const pendingVolumes: Writable<Record<string, number>> = writable({});

export const groups: Readable<Group[]> = derived(settings, ($settings) => $settings?.groups ?? []);
export const scenes: Readable<Scene[]> = derived(settings, ($settings) => $settings?.scenes ?? []);

export const activeGroupId: Readable<string | null> = derived(
  settings,
  ($settings) => $settings?.active_group_id ?? null,
);

export const activeGroup: Readable<Group | null> = derived(
  [groups, activeGroupId],
  ([$groups, $activeId]) => $groups.find((group) => group.id === $activeId) ?? null,
);

export const inspectedApp: Readable<Application | null> = derived(
  [applications, inspectedAppKey],
  ([$applications, $key]) => $applications.find((app) => app.app_key === $key) ?? null,
);

/** Applications after the user's visibility preferences and the search box. */
export const visibleApplications: Readable<Application[]> = derived(
  [applications, settings, searchQuery],
  ([$applications, $settings, $query]) => {
    const prefs = $settings?.ui_prefs;
    return $applications.filter((app) => {
      if (app.hidden && !(prefs?.show_hidden ?? false)) return false;
      if (app.is_system_sounds && !(prefs?.show_system_sounds ?? true)) return false;
      if (!app.running && !(prefs?.show_offline ?? true)) return false;
      return matchesQuery($query, [
        app.display_name,
        app.custom_name,
        app.executable_name,
        app.executable_path,
      ]);
    });
  },
);

/**
 * Display order. Sorting by live peak would make rows swap places while music
 * plays, so ordering uses stable facts only: pinned first, then running before
 * offline, then the user's manual order, then name.
 */
export function sortApplications(list: Application[]): Application[] {
  return [...list].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    if (a.is_system_sounds !== b.is_system_sounds) return a.is_system_sounds ? 1 : -1;
    if (a.running !== b.running) return a.running ? -1 : 1;
    if (a.sort_order !== b.sort_order) return a.sort_order - b.sort_order;
    return a.display_name.localeCompare(b.display_name, undefined, { sensitivity: "base" });
  });
}

export const sortedApplications: Readable<Application[]> = derived(
  visibleApplications,
  sortApplications,
);

/** Section headers in the list, so offline entries do not look like a bug. */
export interface AppSection {
  id: "pinned" | "running" | "offline";
  apps: Application[];
}

export const applicationSections: Readable<AppSection[]> = derived(
  sortedApplications,
  ($apps) => {
    const pinned = $apps.filter((app) => app.pinned);
    const running = $apps.filter((app) => !app.pinned && app.running);
    const offline = $apps.filter((app) => !app.pinned && !app.running);
    return (
      [
        { id: "pinned", apps: pinned },
        { id: "running", apps: running },
        { id: "offline", apps: offline },
      ] as AppSection[]
    ).filter((section) => section.apps.length > 0);
  },
);

/* ── Toasts ─────────────────────────────────────────────────────────────── */

export interface ToastMessage {
  id: number;
  text: string;
  tone: "info" | "success" | "danger";
  /** Optional single undo affordance, used by destructive actions. */
  undo?: () => void;
}

export const toasts: Writable<ToastMessage[]> = writable([]);
let toastSeq = 0;

export function pushToast(
  text: string,
  tone: ToastMessage["tone"] = "info",
  undo?: () => void,
): void {
  const id = ++toastSeq;
  toasts.update((list) => [...list, { id, text, tone, undo }]);
  setTimeout(() => dismissToast(id), undo ? 8000 : tone === "danger" ? 6000 : 3200);
}

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((toast) => toast.id !== id));
}

/* ── Application mutations ──────────────────────────────────────────────── */

/** Optimistic paint during a drag. No IPC — `commitAppVolume` sends it. */
export function previewAppVolume(appKey: string, volume: number): void {
  pendingVolumes.update((map) => ({ ...map, [appKey]: volume }));
}

export async function commitAppVolume(appKey: string, volume: number): Promise<void> {
  previewAppVolume(appKey, volume);
  try {
    await setAppVolume(appKey, volume);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    pendingVolumes.update(({ [appKey]: _dropped, ...rest }) => rest);
  }
}

export async function toggleAppMute(app: Application): Promise<void> {
  const next = !app.muted;
  applications.update((list) =>
    list.map((item) => (item.app_key === app.app_key ? { ...item, muted: next } : item)),
  );
  try {
    await setAppMute(app.app_key, next);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    applications.update((list) =>
      list.map((item) => (item.app_key === app.app_key ? { ...item, muted: app.muted } : item)),
    );
  }
}

export function previewSessionVolume(liveId: string, volume: number): void {
  pendingVolumes.update((map) => ({ ...map, [liveId]: volume }));
}

export async function commitSessionVolume(liveId: string, volume: number): Promise<void> {
  previewSessionVolume(liveId, volume);
  try {
    await setSessionVolume(liveId, volume);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    pendingVolumes.update(({ [liveId]: _dropped, ...rest }) => rest);
  }
}

export async function toggleSessionMute(liveId: string, muted: boolean): Promise<void> {
  try {
    await setSessionMute(liveId, muted);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

export async function patchApplication(
  appKey: string,
  patch: ApplicationPatch,
): Promise<void> {
  // Applied locally first so a pin or rename lands on the next frame rather
  // than after a round trip.
  applications.update((list) =>
    list.map((item) => (item.app_key === appKey ? { ...item, ...patch } : item)),
  );
  try {
    const saved = await ipcUpdateApplication(appKey, patch);
    applications.update((list) =>
      list.map((item) => (item.app_key === appKey ? saved : item)),
    );
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    await refreshMixer();
  }
}

export async function addApplicationFromPath(path: string): Promise<Application | null> {
  try {
    const added = await ipcAddApplication(path);
    applications.update((list) =>
      list.some((item) => item.app_key === added.app_key)
        ? list.map((item) => (item.app_key === added.app_key ? added : item))
        : [...list, added],
    );
    return added;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return null;
  }
}

/**
 * Forgets NovaMixer's settings for an application. The application itself is
 * untouched, which the confirmation copy states explicitly.
 */
export async function forgetApplication(app: Application): Promise<boolean> {
  try {
    await ipcRemoveApplication(app.app_key);
    applications.update((list) => list.filter((item) => item.app_key !== app.app_key));
    if (get(inspectedAppKey) === app.app_key) inspectedAppKey.set(null);
    return true;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return false;
  }
}

export async function reorder(appKeys: string[]): Promise<void> {
  applications.update((list) => {
    const order = new Map(appKeys.map((key, index) => [key, index]));
    return list.map((item) => ({
      ...item,
      sort_order: order.get(item.app_key) ?? item.sort_order,
    }));
  });
  try {
    await ipcReorderApplications(appKeys);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

/* ── Master ─────────────────────────────────────────────────────────────── */

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

/* ── Groups ─────────────────────────────────────────────────────────────── */

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

/* ── Scenes ─────────────────────────────────────────────────────────────── */

export async function saveScene(scene: Scene): Promise<Scene | null> {
  try {
    const saved = await ipcUpsertScene(scene);
    settings.update((current) => {
      if (!current) return current;
      const exists = current.scenes.some((item) => item.id === saved.id);
      return {
        ...current,
        scenes: exists
          ? current.scenes.map((item) => (item.id === saved.id ? saved : item))
          : [...current.scenes, saved],
      };
    });
    return saved;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return null;
  }
}

export async function removeScene(sceneId: string): Promise<boolean> {
  try {
    await ipcDeleteScene(sceneId);
    settings.update((current) =>
      current
        ? { ...current, scenes: current.scenes.filter((scene) => scene.id !== sceneId) }
        : current,
    );
    return true;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return false;
  }
}

export async function runScene(sceneId: string): Promise<void> {
  try {
    await ipcApplyScene(sceneId);
  } catch (error) {
    pushToast(errorMessage(error), "danger");
  }
}

/** Snapshots the levels the user has already dialled in by ear. */
export async function captureCurrentAsScene(name: string): Promise<Scene | null> {
  try {
    const created = await ipcCaptureScene(name);
    settings.update((current) =>
      current ? { ...current, scenes: [...current.scenes, created] } : current,
    );
    return created;
  } catch (error) {
    pushToast(errorMessage(error), "danger");
    return null;
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
    const snapshot = await listApplications();
    master.set(snapshot.master);
    applications.set(snapshot.applications);
    pendingVolumes.set({});
    audioAvailable.set(true);
  } catch {
    audioAvailable.set(false);
  }
}

/* ── Event application ──────────────────────────────────────────────────── */
/* Pure state transitions, so the integration tests can drive them without a
   Tauri runtime. `events.ts` wires them to the real listeners. */

export function applyApplicationAdded(app: Application): void {
  applications.update((list) =>
    list.some((item) => item.app_key === app.app_key)
      ? list.map((item) => (item.app_key === app.app_key ? app : item))
      : [...list, app],
  );
}

export function applyApplicationUpdated(app: Application): void {
  applications.update((list) =>
    list.map((item) => (item.app_key === app.app_key ? app : item)),
  );
  // The authoritative value has landed, so the optimistic one is obsolete.
  pendingVolumes.update(({ [app.app_key]: _dropped, ...rest }) => rest);
}

export function applyApplicationRemoved(appKey: string): void {
  applications.update((list) => list.filter((item) => item.app_key !== appKey));
  appPeaks.update(({ [appKey]: _dropped, ...rest }) => rest);
  pendingVolumes.update(({ [appKey]: _dropped, ...rest }) => rest);
}

export function applyMasterUpdated(state: MasterState): void {
  master.set(state);
}

/** An endpoint swap replaces the world; merging would keep dead sessions. */
export function applyEndpointChanged(snapshot: {
  master: MasterState;
  applications: Application[];
}): void {
  master.set(snapshot.master);
  applications.set(snapshot.applications);
  appPeaks.set({});
  sessionPeaks.set({});
  pendingVolumes.set({});
  audioAvailable.set(true);
}

export function applyPeaks(batch: PeakBatch): void {
  const nextApps: Record<string, number> = {};
  const nextSessions: Record<string, number> = {};
  for (const entry of batch.applications) {
    nextApps[entry.app_key] = entry.peak;
    for (const session of entry.sessions) {
      nextSessions[session.live_id] = session.peak;
    }
  }
  appPeaks.set(nextApps);
  sessionPeaks.set(nextSessions);
  masterPeak.set(batch.master_peak);
}
