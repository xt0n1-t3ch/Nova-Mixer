/** Shared UI constants and small pure helpers. */

export const ANIMATION_DURATIONS_MS = {
  instant: 80,
  fast: 140,
  normal: 220,
  slow: 360,
  stagger: 12,
} as const;

export const VOLUME_PRESETS = [0.25, 0.5, 0.75, 1] as const;

export function reducedMotion(): boolean {
  return (
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/** Collapses a duration to zero when the user asked for less motion. */
export function motionDuration(ms: number): number {
  return reducedMotion() ? 0 : ms;
}

export type ViewId = "mixer" | "groups" | "settings" | "about";

export const VIEW_IDS: readonly ViewId[] = ["mixer", "groups", "settings", "about"];

export function isViewId(value: string): value is ViewId {
  return (VIEW_IDS as readonly string[]).includes(value);
}

/** Case-insensitive substring match over a session's user-visible identifiers. */
export function matchesQuery(
  query: string,
  fields: readonly (string | null | undefined)[],
): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return fields.some((field) => field != null && field.toLowerCase().includes(needle));
}

/**
 * Deterministic tint for an app that has no icon, so the same application keeps
 * the same colour between launches instead of flickering on every render.
 */
export type BadgeTint = "blue" | "green" | "orange" | "red" | "purple" | "teal";

const BADGE_TINTS: readonly BadgeTint[] = ["blue", "green", "orange", "red", "purple", "teal"];

export function tintForKey(key: string): BadgeTint {
  let hash = 0;
  for (let i = 0; i < key.length; i++) {
    hash = (hash * 31 + key.charCodeAt(i)) >>> 0;
  }
  return BADGE_TINTS[hash % BADGE_TINTS.length];
}

/** First letter shown in place of a missing icon. */
export function initialFor(name: string): string {
  const trimmed = name.trim();
  return trimmed ? trimmed[0].toUpperCase() : "?";
}

/**
 * Trailing-edge debounce. Used for autosave so a slider drag writes settings
 * once instead of on every frame.
 */
export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  waitMs: number,
): ((...args: A) => void) & { flush: () => void; cancel: () => void } {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: A | null = null;

  const run = (...args: A): void => {
    pending = args;
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      const args2 = pending;
      pending = null;
      if (args2) fn(...args2);
    }, waitMs);
  };

  run.flush = (): void => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
    const args = pending;
    pending = null;
    if (args) fn(...args);
  };

  run.cancel = (): void => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    pending = null;
  };

  return run;
}

export interface Shortcut {
  keys: readonly string[];
  descriptionKey: string;
}

export const SHORTCUTS: readonly Shortcut[] = [
  { keys: ["/"], descriptionKey: "shortcut.focusSearch" },
  { keys: ["g", "m"], descriptionKey: "shortcut.goMixer" },
  { keys: ["g", "g"], descriptionKey: "shortcut.goGroups" },
  { keys: ["g", "s"], descriptionKey: "shortcut.goSettings" },
  { keys: ["m"], descriptionKey: "shortcut.toggleMasterMute" },
  { keys: ["d"], descriptionKey: "shortcut.toggleDensity" },
  { keys: ["esc"], descriptionKey: "shortcut.closeDialog" },
];

/** Human label for a Tauri accelerator, e.g. `"AudioVolumeUp"` → `"Audio Volume Up"`. */
export function formatAccelerator(accelerator: string | null): string | null {
  if (!accelerator) return null;
  return accelerator
    .split("+")
    .map((part) => part.replace(/([a-z])([A-Z])/g, "$1 $2"))
    .join(" + ");
}
