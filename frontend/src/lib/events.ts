/**
 * Bridges backend events to the store transitions.
 *
 * Kept apart from `stores.ts` so the transition functions stay importable in
 * tests without dragging in the Tauri event runtime.
 */
import {
  EVENTS,
  type Application,
  type ApplicationRemovedPayload,
  type AppSettings,
  type MasterState,
  type MixerSnapshot,
  type PeakBatch,
} from "./api";
import {
  applyApplicationAdded,
  applyApplicationRemoved,
  applyApplicationUpdated,
  applyEndpointChanged,
  applyMasterUpdated,
  applyPeaks,
  audioAvailable,
  settings,
} from "./stores";

type Unlisten = () => void;

/**
 * Subscribes to every backend event. Returns a disposer.
 *
 * Outside a Tauri webview — unit tests, `vite preview`, the design harness —
 * the module resolves but its internals are absent, so `listen` throws. Both
 * that and a failed import land in the same catch, and the app degrades to
 * whatever `list_applications` returned instead of failing to start.
 */
export async function installEventListeners(): Promise<Unlisten> {
  try {
    const { listen } = await import("@tauri-apps/api/event");

    const disposers = await Promise.all([
      listen<Application>(EVENTS.applicationAdded, (event) =>
        applyApplicationAdded(event.payload),
      ),
      listen<Application>(EVENTS.applicationUpdated, (event) =>
        applyApplicationUpdated(event.payload),
      ),
      listen<ApplicationRemovedPayload>(EVENTS.applicationRemoved, (event) =>
        applyApplicationRemoved(event.payload.app_key),
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
  } catch {
    return () => {};
  }
}
