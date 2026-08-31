/**
 * Bridges backend events to the store transitions.
 *
 * Kept apart from `stores.ts` so the transition functions stay importable in
 * tests without dragging in the Tauri event runtime.
 */
import {
  EVENTS,
  type AppSettings,
  type AudioSession,
  type MasterState,
  type MixerSnapshot,
  type PeakBatch,
  type SessionRemovedPayload,
} from "./api";
import {
  applyEndpointChanged,
  applyMasterUpdated,
  applyPeaks,
  applySessionAdded,
  applySessionRemoved,
  applySessionUpdated,
  audioAvailable,
  settings,
} from "./stores";

type Unlisten = () => void;

/**
 * Subscribes to every backend event. Returns a disposer.
 *
 * Outside a Tauri webview (unit tests, `vite preview`) the dynamic import
 * fails, so the app degrades to whatever `list_sessions` returned instead of
 * crashing on load.
 */
export async function installEventListeners(): Promise<Unlisten> {
  let listen: typeof import("@tauri-apps/api/event").listen;
  try {
    ({ listen } = await import("@tauri-apps/api/event"));
  } catch {
    return () => {};
  }

  const disposers = await Promise.all([
    listen<AudioSession>(EVENTS.sessionAdded, (event) => applySessionAdded(event.payload)),
    listen<AudioSession>(EVENTS.sessionUpdated, (event) => applySessionUpdated(event.payload)),
    listen<SessionRemovedPayload>(EVENTS.sessionRemoved, (event) =>
      applySessionRemoved(event.payload.live_id),
    ),
    listen<MasterState>(EVENTS.masterUpdated, (event) => applyMasterUpdated(event.payload)),
    listen<MixerSnapshot>(EVENTS.endpointChanged, (event) => applyEndpointChanged(event.payload)),
    listen<PeakBatch>(EVENTS.peaks, (event) => applyPeaks(event.payload)),
    listen<AppSettings>(EVENTS.settingsUpdated, (event) => settings.set(event.payload)),
  ]);

  audioAvailable.set(true);

  return () => {
    for (const dispose of disposers) dispose();
  };
}
