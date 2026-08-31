import { get } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import type { AudioSession, MasterState } from "@/lib/api";
import {
  applyEndpointChanged,
  applyMasterUpdated,
  applyPeaks,
  applySessionAdded,
  applySessionRemoved,
  applySessionUpdated,
  master,
  peaks,
  pendingVolumes,
  previewSessionVolume,
  searchQuery,
  sessions,
  settings,
  sortSessions,
  sortedSessions,
  visibleSessions,
} from "@/lib/stores";
import { defaultSettings, makeSession, masterState } from "../helpers/fixtures";

beforeEach(() => {
  sessions.set([]);
  peaks.set({});
  pendingVolumes.set({});
  master.set(null);
  searchQuery.set("");
  settings.set(defaultSettings());
});

/**
 * The v1 implementation enumerated audio sessions once at startup and cached
 * the result, so an application launched afterwards was invisible and its
 * slider did nothing. These tests pin the frontend half of the fix: a session
 * that arrives by event must become a real, controllable row.
 */
describe("a session created after startup", () => {
  it("appears in the list without a refresh", () => {
    expect(get(sessions)).toHaveLength(0);

    applySessionAdded(makeSession({ live_id: "ep::spotify-1", display_name: "Spotify" }));

    expect(get(sessions)).toHaveLength(1);
    expect(get(sessions)[0].display_name).toBe("Spotify");
  });

  it("is rendered as controllable, so its slider is live", () => {
    applySessionAdded(makeSession({ live_id: "ep::spotify-1" }));
    expect(get(sortedSessions)[0].controllable).toBe(true);
  });

  it("is not duplicated when the backend re-announces it", () => {
    const session = makeSession({ live_id: "ep::spotify-1" });
    applySessionAdded(session);
    applySessionAdded(session);
    expect(get(sessions)).toHaveLength(1);
  });

  it("survives the close-and-relaunch cycle that broke v1", () => {
    applySessionAdded(makeSession({ live_id: "ep::spotify-1", volume: 0.4 }));
    applySessionRemoved("ep::spotify-1");
    expect(get(sessions)).toHaveLength(0);

    // A relaunch is a new session instance with the same durable app key.
    applySessionAdded(
      makeSession({ live_id: "ep::spotify-2", app_key: "spotify.exe", volume: 0.4 }),
    );

    const live = get(sessions);
    expect(live).toHaveLength(1);
    expect(live[0].live_id).toBe("ep::spotify-2");
    expect(live[0].app_key).toBe("spotify.exe");
    expect(live[0].controllable).toBe(true);
  });
});

describe("session updates", () => {
  it("replaces the matching session and leaves the others alone", () => {
    applySessionAdded(makeSession({ live_id: "a", volume: 0.5 }));
    applySessionAdded(makeSession({ live_id: "b", volume: 0.5 }));

    applySessionUpdated(makeSession({ live_id: "b", volume: 0.9 }));

    expect(get(sessions).find((s) => s.live_id === "a")?.volume).toBe(0.5);
    expect(get(sessions).find((s) => s.live_id === "b")?.volume).toBe(0.9);
  });

  it("clears the optimistic value so the slider stops fighting the backend", () => {
    applySessionAdded(makeSession({ live_id: "a", volume: 0.5 }));
    previewSessionVolume("a", 0.8);
    expect(get(pendingVolumes).a).toBe(0.8);

    applySessionUpdated(makeSession({ live_id: "a", volume: 0.8 }));
    expect(get(pendingVolumes).a).toBeUndefined();
  });

  it("keeps an idle session listed and controllable, matching the Windows mixer", () => {
    applySessionAdded(makeSession({ live_id: "a", state: "active" }));
    applySessionUpdated(makeSession({ live_id: "a", state: "inactive" }));

    const live = get(sessions);
    expect(live).toHaveLength(1);
    expect(live[0].state).toBe("inactive");
    expect(live[0].controllable).toBe(true);
  });
});

describe("session removal", () => {
  it("drops the row and every trace of it", () => {
    applySessionAdded(makeSession({ live_id: "a" }));
    previewSessionVolume("a", 0.3);
    applyPeaks({ timestamp_ms: 1, master_peak: 0.2, sessions: [{ live_id: "a", peak: 0.7 }] });

    applySessionRemoved("a");

    expect(get(sessions)).toHaveLength(0);
    expect(get(peaks).a).toBeUndefined();
    expect(get(pendingVolumes).a).toBeUndefined();
  });
});

describe("endpoint change", () => {
  it("replaces the world instead of merging, so dead sessions cannot linger", () => {
    applySessionAdded(makeSession({ live_id: "old-ep::a" }));
    previewSessionVolume("old-ep::a", 0.2);
    applyPeaks({ timestamp_ms: 1, master_peak: 0.5, sessions: [{ live_id: "old-ep::a", peak: 0.4 }] });

    const replacement: MasterState = masterState({
      endpoint_id: "new-ep",
      endpoint_name: "Headphones",
    });
    applyEndpointChanged({
      master: replacement,
      sessions: [makeSession({ live_id: "new-ep::a" })],
    });

    expect(get(sessions).map((s) => s.live_id)).toEqual(["new-ep::a"]);
    expect(get(master)?.endpoint_name).toBe("Headphones");
    expect(get(peaks)).toEqual({});
    expect(get(pendingVolumes)).toEqual({});
  });
});

describe("master updates", () => {
  it("stores the endpoint state the strip renders", () => {
    applyMasterUpdated(masterState({ volume: 0.33, muted: true }));
    expect(get(master)?.volume).toBe(0.33);
    expect(get(master)?.muted).toBe(true);
  });
});

describe("peak batches", () => {
  it("index peaks by live id and carry the master level", () => {
    applyPeaks({
      timestamp_ms: 100,
      master_peak: 0.6,
      sessions: [
        { live_id: "a", peak: 0.4 },
        { live_id: "b", peak: 0.9 },
      ],
    });

    expect(get(peaks)).toEqual({ a: 0.4, b: 0.9 });
  });

  it("replaces the previous batch, so a stopped app does not keep a stale bar", () => {
    applyPeaks({ timestamp_ms: 1, master_peak: 0.5, sessions: [{ live_id: "a", peak: 0.8 }] });
    applyPeaks({ timestamp_ms: 2, master_peak: 0, sessions: [] });
    expect(get(peaks)).toEqual({});
  });
});

describe("visibility preferences", () => {
  it("hides idle sessions only when the user asked to", () => {
    applySessionAdded(makeSession({ live_id: "a", state: "active" }));
    applySessionAdded(makeSession({ live_id: "b", state: "inactive" }));
    expect(get(visibleSessions)).toHaveLength(2);

    const config = defaultSettings();
    settings.set({ ...config, ui_prefs: { ...config.ui_prefs, show_inactive: false } });
    expect(get(visibleSessions).map((s) => s.live_id)).toEqual(["a"]);
  });

  it("hides system sounds only when the user asked to", () => {
    applySessionAdded(makeSession({ live_id: "a" }));
    applySessionAdded(makeSession({ live_id: "sys", is_system_sounds: true }));

    const config = defaultSettings();
    settings.set({ ...config, ui_prefs: { ...config.ui_prefs, show_system_sounds: false } });
    expect(get(visibleSessions).map((s) => s.live_id)).toEqual(["a"]);
  });

  it("filters by the search box across name and executable", () => {
    applySessionAdded(
      makeSession({ live_id: "a", display_name: "Spotify", executable_name: "Spotify.exe" }),
    );
    applySessionAdded(
      makeSession({ live_id: "b", display_name: "Edge", executable_name: "msedge.exe" }),
    );

    searchQuery.set("msedge");
    expect(get(visibleSessions).map((s) => s.live_id)).toEqual(["b"]);
  });
});

describe("sortSessions", () => {
  it("puts audible applications above idle ones", () => {
    const ordered = sortSessions([
      makeSession({ live_id: "idle", display_name: "Aaa", state: "inactive" }),
      makeSession({ live_id: "live", display_name: "Zzz", state: "active" }),
    ]);
    expect(ordered.map((s) => s.live_id)).toEqual(["live", "idle"]);
  });

  it("sinks system sounds below real applications", () => {
    const ordered = sortSessions([
      makeSession({ live_id: "sys", display_name: "Aaa", is_system_sounds: true }),
      makeSession({ live_id: "app", display_name: "Zzz" }),
    ]);
    expect(ordered.map((s) => s.live_id)).toEqual(["app", "sys"]);
  });

  it("orders peers by name so rows do not swap while audio plays", () => {
    const ordered = sortSessions([
      makeSession({ live_id: "c", display_name: "Chrome" }),
      makeSession({ live_id: "a", display_name: "Ableton" }),
      makeSession({ live_id: "b", display_name: "brave" }),
    ]);
    expect(ordered.map((s) => s.display_name)).toEqual(["Ableton", "brave", "Chrome"]);
  });

  it("does not mutate the input array", () => {
    const input = [
      makeSession({ live_id: "b", display_name: "Bbb" }),
      makeSession({ live_id: "a", display_name: "Aaa" }),
    ];
    sortSessions(input);
    expect(input.map((s) => s.live_id)).toEqual(["b", "a"]);
  });
});

describe("group name resolution", () => {
  it("carries the backend-resolved group id through to the row", () => {
    const session: AudioSession = makeSession({ live_id: "a", group_id: "group-1" });
    applySessionAdded(session);
    expect(get(sessions)[0].group_id).toBe("group-1");
  });
});
