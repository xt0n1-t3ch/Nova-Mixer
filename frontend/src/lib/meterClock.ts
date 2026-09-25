/**
 * One animation clock for every level meter.
 *
 * Each meter used to run its own `requestAnimationFrame` loop at the display
 * rate and restyle up to 48 DOM segments per frame. With a dozen meters that
 * kept the renderer and GPU process busy (measured at ~150% of one core with
 * the window merely open). Meters now register a paint callback here: one loop
 * runs at most 30 times a second, only while at least one meter still has
 * something to draw, and never while the window is hidden.
 */

type Painter = (now: number) => boolean;

const FRAME_MS = 1000 / 30;
const painters = new Set<Painter>();
let frame = 0;
let last = 0;

function tick(now: number): void {
  frame = 0;
  if (now - last < FRAME_MS - 1) {
    frame = requestAnimationFrame(tick);
    return;
  }
  last = now;
  let busy = false;
  for (const paint of painters) {
    if (paint(now)) busy = true;
  }
  // Stop when every meter has settled; the next incoming peak restarts it.
  if (busy && !document.hidden) frame = requestAnimationFrame(tick);
}

/** Starts the shared loop if it is idle. Safe to call on every peak update. */
export function wakeMeters(): void {
  if (frame === 0 && !document.hidden && painters.size > 0) {
    frame = requestAnimationFrame(tick);
  }
}

/** Registers a painter; returns the function that unregisters it. */
export function addMeter(paint: Painter): () => void {
  painters.add(paint);
  wakeMeters();
  return () => {
    painters.delete(paint);
    if (painters.size === 0 && frame !== 0) {
      cancelAnimationFrame(frame);
      frame = 0;
    }
  };
}

if (typeof document !== "undefined") {
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      if (frame !== 0) cancelAnimationFrame(frame);
      frame = 0;
    } else {
      wakeMeters();
    }
  });
}
