import { describe, expect, it, vi } from "vitest";
import {
  debounce,
  formatAccelerator,
  initialFor,
  isViewId,
  matchesQuery,
  tintForKey,
  VIEW_IDS,
} from "@/lib/ux";

describe("matchesQuery", () => {
  it("treats an empty or whitespace query as no filter", () => {
    expect(matchesQuery("", ["Spotify"])).toBe(true);
    expect(matchesQuery("   ", ["Spotify"])).toBe(true);
  });

  it("matches case-insensitively across every supplied field", () => {
    expect(matchesQuery("spot", ["Spotify", "Spotify.exe"])).toBe(true);
    expect(matchesQuery("EXE", ["Spotify", "Spotify.exe"])).toBe(true);
  });

  it("tolerates null and undefined fields, which unreadable metadata produces", () => {
    expect(matchesQuery("spot", ["Spotify", null, undefined])).toBe(true);
    expect(matchesQuery("edge", [null, undefined])).toBe(false);
  });

  it("reports no match when nothing contains the needle", () => {
    expect(matchesQuery("firefox", ["Spotify", "Spotify.exe"])).toBe(false);
  });
});

describe("tintForKey", () => {
  it("is stable, so an app keeps its colour between launches", () => {
    expect(tintForKey("spotify.exe")).toBe(tintForKey("spotify.exe"));
  });

  it("spreads distinct keys across more than one tint", () => {
    const keys = ["a.exe", "b.exe", "c.exe", "d.exe", "e.exe", "f.exe", "g.exe", "h.exe"];
    expect(new Set(keys.map(tintForKey)).size).toBeGreaterThan(1);
  });
});

describe("initialFor", () => {
  it("uppercases the first letter", () => {
    expect(initialFor("spotify")).toBe("S");
  });

  it("ignores leading whitespace", () => {
    expect(initialFor("  edge")).toBe("E");
  });

  it("falls back rather than rendering an empty badge", () => {
    expect(initialFor("")).toBe("?");
    expect(initialFor("   ")).toBe("?");
  });
});

describe("isViewId", () => {
  it("accepts every declared view", () => {
    for (const id of VIEW_IDS) expect(isViewId(id)).toBe(true);
  });

  it("rejects an unknown id so a bad shortcut cannot blank the shell", () => {
    expect(isViewId("nope")).toBe(false);
  });
});

describe("formatAccelerator", () => {
  it("returns null for an unbound action", () => {
    expect(formatAccelerator(null)).toBeNull();
  });

  it("splits a camel-case Tauri key into readable words", () => {
    expect(formatAccelerator("AudioVolumeUp")).toBe("Audio Volume Up");
  });

  it("spaces the plus signs in a chord", () => {
    expect(formatAccelerator("Control+Alt+Up")).toBe("Control + Alt + Up");
  });
});

describe("debounce", () => {
  it("runs once with the last arguments after the wait", () => {
    vi.useFakeTimers();
    const spy = vi.fn();
    const run = debounce(spy, 500);

    run(1);
    run(2);
    run(3);
    expect(spy).not.toHaveBeenCalled();

    vi.advanceTimersByTime(500);
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith(3);
    vi.useRealTimers();
  });

  it("flush runs the pending call immediately, which the Save button relies on", () => {
    vi.useFakeTimers();
    const spy = vi.fn();
    const run = debounce(spy, 500);

    run("pending");
    run.flush();
    expect(spy).toHaveBeenCalledWith("pending");

    vi.advanceTimersByTime(500);
    expect(spy).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("cancel drops the pending call entirely", () => {
    vi.useFakeTimers();
    const spy = vi.fn();
    const run = debounce(spy, 500);

    run("dropped");
    run.cancel();
    vi.advanceTimersByTime(1000);
    expect(spy).not.toHaveBeenCalled();
    vi.useRealTimers();
  });
});
