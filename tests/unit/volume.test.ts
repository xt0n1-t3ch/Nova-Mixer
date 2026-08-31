import { describe, expect, it } from "vitest";
import {
  clampScalar,
  decayPeak,
  effectiveStep,
  formatPercent,
  fromPercent,
  meterBand,
  nextHold,
  steppedVolume,
  toPercent,
  volumeValueText,
} from "@/lib/volume";

describe("scalar conversion", () => {
  it("maps scalars to whole percentages", () => {
    expect(toPercent(0)).toBe(0);
    expect(toPercent(0.625)).toBe(63);
    expect(toPercent(1)).toBe(100);
  });

  it("round-trips the values a preset button produces", () => {
    for (const percent of [0, 25, 50, 75, 100]) {
      expect(toPercent(fromPercent(percent))).toBe(percent);
    }
  });

  it("clamps out-of-range and non-finite input rather than propagating it", () => {
    expect(clampScalar(-1)).toBe(0);
    expect(clampScalar(2)).toBe(1);
    expect(clampScalar(Number.NaN)).toBe(0);
    expect(clampScalar(Number.POSITIVE_INFINITY)).toBe(1);
  });

  it("formats without a space, matching the Windows mixer", () => {
    expect(formatPercent(0.42)).toBe("42%");
  });
});

describe("volumeValueText", () => {
  it("announces the level alone when audible", () => {
    expect(volumeValueText(0.5, false, "muted")).toBe("50%");
  });

  it("announces mute, because the level alone would be misleading", () => {
    expect(volumeValueText(0.5, true, "muted")).toBe("50%, muted");
  });

  it("uses the translated word it is given", () => {
    expect(volumeValueText(0.3, true, "silenciado")).toBe("30%, silenciado");
  });
});

describe("effectiveStep", () => {
  it("returns the raw step when adaptive stepping is off", () => {
    expect(effectiveStep(0.1, 0.05, false)).toBe(0.05);
    expect(effectiveStep(0.9, 0.05, false)).toBe(0.05);
  });

  it("halves the step in the quiet third, where a fixed step doubles loudness", () => {
    expect(effectiveStep(0.1, 0.05, true)).toBeCloseTo(0.025);
    expect(effectiveStep(0.3, 0.05, true)).toBeCloseTo(0.025);
  });

  it("widens the step in the loud third", () => {
    expect(effectiveStep(0.7, 0.05, true)).toBeCloseTo(0.075);
    expect(effectiveStep(1, 0.05, true)).toBeCloseTo(0.075);
  });

  it("keeps the raw step in the middle band", () => {
    expect(effectiveStep(0.5, 0.05, true)).toBe(0.05);
  });
});

describe("steppedVolume", () => {
  it("applies a delta and stays inside the legal range", () => {
    expect(steppedVolume(0.5, 0.05)).toBeCloseTo(0.55);
    expect(steppedVolume(0.98, 0.05)).toBe(1);
    expect(steppedVolume(0.02, -0.05)).toBe(0);
  });

  it("avoids float drift accumulating across repeated steps", () => {
    let level = 0;
    for (let i = 0; i < 20; i++) level = steppedVolume(level, 0.05);
    expect(level).toBe(1);
  });
});

describe("meterBand", () => {
  it("splits the range into the three product bands", () => {
    expect(meterBand(0)).toBe("low");
    expect(meterBand(0.59)).toBe("low");
    expect(meterBand(0.6)).toBe("mid");
    expect(meterBand(0.88)).toBe("mid");
    expect(meterBand(0.89)).toBe("hot");
    expect(meterBand(1)).toBe("hot");
  });
});

describe("decayPeak", () => {
  it("attacks instantly so a transient is never missed", () => {
    expect(decayPeak(0.1, 0.9, 16)).toBe(0.9);
  });

  it("falls gradually instead of snapping to the new value", () => {
    const next = decayPeak(0.8, 0.1, 16);
    expect(next).toBeLessThan(0.8);
    expect(next).toBeGreaterThan(0.1);
  });

  it("halves in roughly 120 ms", () => {
    expect(decayPeak(0.8, 0, 120)).toBeCloseTo(0.4, 2);
  });

  it("settles on the incoming value once the residue is inaudible", () => {
    expect(decayPeak(0.001, 0, 16)).toBe(0);
  });
});

describe("nextHold", () => {
  it("raises the marker immediately on a louder peak", () => {
    expect(nextHold({ value: 0.3, since: 0 }, 0.7, 100)).toEqual({ value: 0.7, since: 100 });
  });

  it("keeps a spike visible for the hold window", () => {
    expect(nextHold({ value: 0.9, since: 0 }, 0.2, 400)).toEqual({ value: 0.9, since: 0 });
  });

  it("drops to the current peak once the window elapses", () => {
    expect(nextHold({ value: 0.9, since: 0 }, 0.2, 900)).toEqual({ value: 0.2, since: 900 });
  });
});
