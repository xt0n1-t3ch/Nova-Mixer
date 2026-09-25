import { describe, expect, it, vi } from "vitest";
import {
  debounce,
  acceleratorFromEvent,
  formatAccelerator,
  initialFor,
  isViewId,
  matchesQuery,
  VIEW_IDS,
  type KeyChord,
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

  it("shows physical key codes as the key a user sees", () => {
    expect(formatAccelerator("Control+Shift+KeyM")).toBe("Control + Shift + M");
    expect(formatAccelerator("Alt+Digit7")).toBe("Alt + 7");
    expect(formatAccelerator("Control+ArrowUp")).toBe("Control + Up");
  });
});

describe("acceleratorFromEvent", () => {
  const chord = (over: Partial<KeyChord>): KeyChord => ({
    key: "",
    code: "",
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...over,
  });

  it("uses the physical key, so a Spanish layout still gives a valid accelerator", () => {
    // On a Spanish layout Shift+7 types "/": `event.key` would build
    // `Control+Shift+/`, which the global-shortcut parser rejects.
    expect(
      acceleratorFromEvent(chord({ key: "/", code: "Digit7", ctrlKey: true, shiftKey: true })),
    ).toBe("Control+Shift+Digit7");
    expect(acceleratorFromEvent(chord({ key: "m", code: "KeyM", ctrlKey: true, altKey: true }))).toBe(
      "Control+Alt+KeyM",
    );
  });

  it("binds media and function keys without a modifier", () => {
    expect(acceleratorFromEvent(chord({ key: "AudioVolumeUp", code: "" }))).toBe("AudioVolumeUp");
    expect(acceleratorFromEvent(chord({ key: "F9", code: "F9" }))).toBe("F9");
  });

  it("refuses a bare letter, which would swallow that key in every application", () => {
    expect(acceleratorFromEvent(chord({ key: "m", code: "KeyM" }))).toBeNull();
  });

  it("waits for a real key while only modifiers are held", () => {
    expect(acceleratorFromEvent(chord({ key: "Control", code: "ControlLeft", ctrlKey: true }))).toBeNull();
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
