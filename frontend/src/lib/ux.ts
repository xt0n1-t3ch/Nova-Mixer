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

export type ViewId = "applications" | "groups" | "settings" | "about";

export const VIEW_IDS: readonly ViewId[] = ["applications", "groups", "settings", "about"];

export function isViewId(value: string): value is ViewId {
  return (VIEW_IDS as readonly string[]).includes(value);
}

/** Case-insensitive substring match over an item's user-visible identifiers. */
export function matchesQuery(
  query: string,
  fields: readonly (string | null | undefined)[],
): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return fields.some((field) => field != null && field.toLowerCase().includes(needle));
}

/* A deterministic per-application tint used to live here, colouring the
   lettered avatar of any application whose icon could not be extracted. It was
   removed with the monochrome chassis: a saturated tile sits in the same row as
   a level meter, and in this interface colour in a channel means signal. The
   fallback avatar is now neutral and identity comes from the letter. */

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
  { keys: ["mod", "k"], descriptionKey: "shortcut.commandPalette" },
  { keys: ["g", "a"], descriptionKey: "shortcut.goApplications" },
  { keys: ["g", "g"], descriptionKey: "shortcut.goGroups" },
  { keys: ["g", "s"], descriptionKey: "shortcut.goSettings" },
  { keys: ["m"], descriptionKey: "shortcut.toggleMasterMute" },
  { keys: ["d"], descriptionKey: "shortcut.toggleDensity" },
  { keys: ["esc"], descriptionKey: "shortcut.closeDialog" },
];

/**
 * Ranks items by subsequence match, so `"dsc"` finds `"Discord"`.
 *
 * A plain `includes` would miss the abbreviations people actually type into a
 * command palette. Items are scored by how tightly the query is packed, which
 * puts a contiguous match ahead of characters scattered across a long title.
 */
export function fuzzyRank<T>(query: string, items: readonly T[], text: (item: T) => string): T[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...items];

  const scored: { item: T; score: number }[] = [];
  for (const item of items) {
    const score = subsequenceScore(needle, text(item).toLowerCase());
    if (score !== null) scored.push({ item, score });
  }
  scored.sort((a, b) => a.score - b.score);
  return scored.map((entry) => entry.item);
}

/** Lower is better. Returns null when the query is not a subsequence at all. */
function subsequenceScore(needle: string, haystack: string): number | null {
  const direct = haystack.indexOf(needle);
  if (direct >= 0) return direct === 0 ? 0 : 1 + direct;

  let index = 0;
  let firstHit = -1;
  let lastHit = -1;
  for (let position = 0; position < haystack.length && index < needle.length; position++) {
    if (haystack[position] === needle[index]) {
      if (firstHit < 0) firstHit = position;
      lastHit = position;
      index += 1;
    }
  }
  if (index !== needle.length) return null;
  // Penalise a match spread thinly across the string.
  return 100 + (lastHit - firstHit) + firstHit;
}

/** Human label for a Tauri accelerator, e.g. `"AudioVolumeUp"` → `"Audio Volume Up"`. */
/** The key fields of a `KeyboardEvent` the accelerator mapping reads. */
export interface KeyChord {
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

/** Media and function keys that are safe to bind with no modifier. */
const STANDALONE_KEYS =
  /^(AudioVolume(Up|Down|Mute)|MediaTrack(Next|Previous)|MediaPlayPause|MediaStop|F([1-9]|1\d|2[0-4]))$/;

/**
 * Builds a global-shortcut accelerator from a key press, or returns null when
 * the press cannot be a shortcut on its own.
 *
 * It reads `event.code`, the physical key, rather than `event.key`, the typed
 * character. `event.key` changes with the keyboard layout and with Shift (on a
 * Spanish layout Shift+7 is "/"), which produced accelerators such as
 * `Control+Shift+/` that the backend rejects. `KeyA`, `Digit7` and `Minus` are
 * the names the backend parses, on every layout.
 */
export function acceleratorFromEvent(event: KeyChord): string | null {
  if (["Control", "Shift", "Alt", "Meta", "AltGraph", "OS"].includes(event.key)) return null;
  // Media keys report an empty or unidentified `code` on some keyboards; their
  // `key` carries the standard name.
  const media = STANDALONE_KEYS.test(event.key) ? event.key : null;
  const key = media ?? (event.code && event.code !== "Unidentified" ? event.code : null);
  if (!key) return null;

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");

  // A bare letter bound globally would swallow that key in every application,
  // so anything but a media or function key needs a modifier.
  if (parts.length === 0 && !STANDALONE_KEYS.test(key)) return null;
  parts.push(key);
  return parts.join("+");
}

/** Readable form of an accelerator for display, e.g. `Control + Alt + A`. */
export function formatAccelerator(accelerator: string | null): string | null {
  if (!accelerator) return null;
  return accelerator
    .split("+")
    .map((part) =>
      part
        .replace(/^Key([A-Z])$/, "$1")
        .replace(/^Digit(\d)$/, "$1")
        .replace(/^Arrow(Up|Down|Left|Right)$/, "$1")
        .replace(/([a-z])([A-Z])/g, "$1 $2"),
    )
    .join(" + ");
}
