/**
 * Volume math.
 *
 * The wire format is always a linear amplitude scalar in 0.0–1.0, matching
 * `ISimpleAudioVolume` and `IAudioEndpointVolume::SetMasterVolumeLevelScalar`.
 * The Windows mixer shows that scalar directly as a percentage, so NovaMixer
 * does the same: the number under a slider is the scalar, not a perceptual
 * loudness estimate. Any perceptual curve would make our percentage disagree
 * with sndvol for the same session, which is worse than being technically
 * "correct" about loudness.
 */

/** Percentage shown to the user for a scalar. */
export function toPercent(scalar: number): number {
  return Math.round(clampScalar(scalar) * 100);
}

/** Scalar for a percentage entered or stepped in the UI. */
export function fromPercent(percent: number): number {
  return clampScalar(percent / 100);
}

export function clampScalar(value: number): number {
  // NaN has no position on the scale, so it becomes silence. Infinity does have
  // one — it is above the maximum — so it clamps to full rather than to zero,
  // which would silently mute instead of saturating.
  if (Number.isNaN(value)) return 0;
  if (value < 0) return 0;
  if (value > 1) return 1;
  return value;
}

/** `"42%"`, with no space, matching the Windows mixer. */
export function formatPercent(scalar: number): string {
  return `${toPercent(scalar)}%`;
}

/**
 * Screen-reader text for a volume control. A bare number is ambiguous when the
 * same page carries several sliders, so the label names what is being changed
 * and whether it is currently silenced.
 */
export function volumeValueText(scalar: number, muted: boolean, mutedWord: string): string {
  return muted ? `${formatPercent(scalar)}, ${mutedWord}` : formatPercent(scalar);
}

/**
 * Adaptive hotkey step. A fixed step is too coarse when quiet — going from 5%
 * to 10% doubles the amplitude — and too slow when loud. Smart volume shrinks
 * the step in the bottom third and widens it in the top third.
 */
export function effectiveStep(current: number, step: number, smart: boolean): number {
  if (!smart) return step;
  const level = clampScalar(current);
  if (level <= 0.3) return step * 0.5;
  if (level >= 0.7) return step * 1.5;
  return step;
}

/** Applies a hotkey step and keeps the result inside the legal range. */
export function steppedVolume(current: number, delta: number): number {
  return clampScalar(Math.round((clampScalar(current) + delta) * 1000) / 1000);
}

/**
 * Meter colour band. Kept here rather than in CSS because the thresholds are a
 * product decision shared by the VU bars, the tray icon, and tests.
 */
export type MeterBand = "low" | "mid" | "hot";

export function meterBand(peak: number): MeterBand {
  const level = clampScalar(peak);
  if (level >= 0.89) return "hot";
  if (level >= 0.6) return "mid";
  return "low";
}

/**
 * One frame of meter decay. Peaks attack instantly so a transient is never
 * missed, then fall exponentially, which is how a physical VU meter reads and
 * what makes a level legible instead of flickering.
 */
export function decayPeak(previous: number, incoming: number, deltaMs: number): number {
  const next = clampScalar(incoming);
  if (next >= previous) return next;
  // Half-life of roughly 120 ms: fast enough to follow music, slow enough to read.
  const factor = Math.pow(0.5, deltaMs / 120);
  const decayed = previous * factor;
  return decayed < 0.002 ? next : Math.max(next, decayed);
}

/** Peak-hold marker: keeps the highest recent value visible for `holdMs`. */
export function nextHold(
  hold: { value: number; since: number },
  peak: number,
  now: number,
  holdMs = 800,
): { value: number; since: number } {
  if (peak >= hold.value) return { value: clampScalar(peak), since: now };
  if (now - hold.since >= holdMs) return { value: clampScalar(peak), since: now };
  return hold;
}
