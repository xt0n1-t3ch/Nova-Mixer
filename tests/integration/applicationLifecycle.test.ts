import { get } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import {
  applicationSections,
  applications,
  applyApplicationAdded,
  applyApplicationRemoved,
  applyApplicationUpdated,
  applyEndpointChanged,
  applyMasterUpdated,
  applyPeaks,
  appPeaks,
  master,
  masterPeak,
  pendingVolumes,
  previewAppVolume,
  sessionPeaks,
  settings,
  sortApplications,
  sortedApplications,
  visibleApplications,
} from "@/lib/stores";
import { defaultSettings, makeApp, makeSession, masterState } from "../helpers/fixtures";

beforeEach(() => {
  applications.set([]);
  appPeaks.set({});
  sessionPeaks.set({});
  pendingVolumes.set({});
  master.set(null);
  settings.set(defaultSettings());
});

/**
 * The v1 interface listed Windows audio *sessions*. Windows gives one
 * application several sessions whenever it likes, so Discord rendered as two
 * identical rows and a row vanished the moment its session expired. These tests
 * pin the v2 model: the application is the row, and sessions are its children.
 */
describe("one application, several sessions", () => {
  it("is a single row no matter how many sessions it owns", () => {
    applyApplicationAdded(
      makeApp({
        app_key: "discord.exe",
        display_name: "Discord",
        sessions: [
          makeSession({ live_id: "ep::discord-1", app_key: "discord.exe" }),
          makeSession({ live_id: "ep::discord-2", app_key: "discord.exe" }),
        ],
      }),
    );

    const list = get(applications);
    expect(list).toHaveLength(1);
    expect(list[0].display_name).toBe("Discord");
    expect(list[0].sessions).toHaveLength(2);
  });

  it("reports the loudest child as the row's peak, never their sum", () => {
    applyPeaks({
      timestamp_ms: 1,
      master_peak: 0.5,
      applications: [
        {
          app_key: "discord.exe",
          peak: 0.8,
          sessions: [
            { live_id: "ep::discord-1", peak: 0.8 },
            { live_id: "ep::discord-2", peak: 0.3 },
          ],
        },
      ],
    });

    expect(get(appPeaks)["discord.exe"]).toBe(0.8);
    expect(get(sessionPeaks)["ep::discord-2"]).toBe(0.3);
  });

  it("carries a mixed flag rather than inventing an average", () => {
    applyApplicationAdded(makeApp({ app_key: "discord.exe", mixed: true, volume: 0 }));
    expect(get(applications)[0].mixed).toBe(true);
  });
});

/** The original bug: an application launched after NovaMixer was uncontrollable. */
describe("an application discovered after startup", () => {
  it("appears without a refresh", () => {
    expect(get(applications)).toHaveLength(0);
    applyApplicationAdded(makeApp({ app_key: "spotify.exe", display_name: "Spotify" }));
    expect(get(applications)).toHaveLength(1);
  });

  it("is controllable straight away", () => {
    applyApplicationAdded(makeApp({ app_key: "spotify.exe" }));
    expect(get(sortedApplications)[0].controllable).toBe(true);
  });

  it("is not duplicated when the backend re-announces it", () => {
    const app = makeApp({ app_key: "spotify.exe" });
    applyApplicationAdded(app);
    applyApplicationAdded(app);
    expect(get(applications)).toHaveLength(1);
  });
});

describe("an application that closes", () => {
  it("stays listed so its settings remain reachable", () => {
    applyApplicationAdded(makeApp({ app_key: "spotify.exe", volume: 0.4, running: true }));
    applyApplicationUpdated(
      makeApp({ app_key: "spotify.exe", volume: 0.4, running: false, sessions: [] }),
    );

    const list = get(applications);
    expect(list).toHaveLength(1);
    expect(list[0].running).toBe(false);
    expect(list[0].volume).toBe(0.4);
  });

  it("disappears only when the user forgets it", () => {
    applyApplicationAdded(makeApp({ app_key: "spotify.exe" }));
    applyApplicationRemoved("spotify.exe");
    expect(get(applications)).toHaveLength(0);
  });

  it("keeps its remembered level across a relaunch", () => {
    applyApplicationAdded(
      makeApp({ app_key: "spotify.exe", volume: 0.25, remembered: true, running: false }),
    );
    applyApplicationUpdated(
      makeApp({
        app_key: "spotify.exe",
        volume: 0.25,
        remembered: true,
        running: true,
        sessions: [makeSession({ live_id: "ep::spotify-new", volume: 0.25 })],
      }),
    );

    const app = get(applications)[0];
    expect(app.running).toBe(true);
    expect(app.volume).toBe(0.25);
    expect(app.sessions[0].volume).toBe(0.25);
  });
});

describe("optimistic values", () => {
  it("are cleared once the authoritative update lands", () => {
    applyApplicationAdded(makeApp({ app_key: "spotify.exe", volume: 0.5 }));
    previewAppVolume("spotify.exe", 0.8);
    expect(get(pendingVolumes)["spotify.exe"]).toBe(0.8);

    applyApplicationUpdated(makeApp({ app_key: "spotify.exe", volume: 0.8 }));
    expect(get(pendingVolumes)["spotify.exe"]).toBeUndefined();
  });

  it("are dropped when the application is forgotten", () => {
    applyApplicationAdded(makeApp({ app_key: "spotify.exe" }));
    previewAppVolume("spotify.exe", 0.3);
    applyApplicationRemoved("spotify.exe");
    expect(get(pendingVolumes)["spotify.exe"]).toBeUndefined();
  });
});

describe("endpoint change", () => {
  it("replaces the world instead of merging, so dead entries cannot linger", () => {
    applyApplicationAdded(makeApp({ app_key: "old.exe" }));
    previewAppVolume("old.exe", 0.2);
    applyPeaks({
      timestamp_ms: 1,
      master_peak: 0.5,
      applications: [{ app_key: "old.exe", peak: 0.4, sessions: [] }],
    });

    applyEndpointChanged({
      master: masterState({ endpoint_id: "new-ep", endpoint_name: "Headphones" }),
      applications: [makeApp({ app_key: "new.exe" })],
    });

    expect(get(applications).map((app) => app.app_key)).toEqual(["new.exe"]);
    expect(get(master)?.endpoint_name).toBe("Headphones");
    expect(get(appPeaks)).toEqual({});
    expect(get(pendingVolumes)).toEqual({});
  });
});

describe("master updates", () => {
  it("store the endpoint state the strip renders", () => {
    applyMasterUpdated(masterState({ volume: 0.33, muted: true }));
    expect(get(master)?.volume).toBe(0.33);
    expect(get(master)?.muted).toBe(true);
  });

  it("carry the endpoint peak separately from the applications", () => {
    applyPeaks({ timestamp_ms: 1, master_peak: 0.65, applications: [] });
    expect(get(masterPeak)).toBe(0.65);
  });
});

describe("visibility preferences", () => {
  it("hide closed applications only when the user asked to", () => {
    applyApplicationAdded(makeApp({ app_key: "a.exe", running: true }));
    applyApplicationAdded(makeApp({ app_key: "b.exe", running: false }));
    expect(get(visibleApplications)).toHaveLength(2);

    const config = defaultSettings();
    settings.set({ ...config, ui_prefs: { ...config.ui_prefs, show_offline: false } });
    expect(get(visibleApplications).map((app) => app.app_key)).toEqual(["a.exe"]);
  });

  it("hide entries the user marked hidden until hidden entries are revealed", () => {
    applyApplicationAdded(makeApp({ app_key: "a.exe" }));
    applyApplicationAdded(makeApp({ app_key: "b.exe", hidden: true }));
    expect(get(visibleApplications).map((app) => app.app_key)).toEqual(["a.exe"]);

    const config = defaultSettings();
    settings.set({ ...config, ui_prefs: { ...config.ui_prefs, show_hidden: true } });
    expect(get(visibleApplications)).toHaveLength(2);
  });

  it("hide system sounds only when the user asked to", () => {
    applyApplicationAdded(makeApp({ app_key: "a.exe" }));
    applyApplicationAdded(makeApp({ app_key: "sys", is_system_sounds: true }));

    const config = defaultSettings();
    settings.set({ ...config, ui_prefs: { ...config.ui_prefs, show_system_sounds: false } });
    expect(get(visibleApplications).map((app) => app.app_key)).toEqual(["a.exe"]);
  });
});

describe("sortApplications", () => {
  it("puts pinned applications first", () => {
    const ordered = sortApplications([
      makeApp({ app_key: "z.exe", display_name: "Zzz" }),
      makeApp({ app_key: "a.exe", display_name: "Aaa", pinned: true }),
    ]);
    expect(ordered.map((app) => app.app_key)).toEqual(["a.exe", "z.exe"]);
  });

  it("puts running applications above closed ones", () => {
    const ordered = sortApplications([
      makeApp({ app_key: "closed.exe", display_name: "Aaa", running: false }),
      makeApp({ app_key: "live.exe", display_name: "Zzz", running: true }),
    ]);
    expect(ordered.map((app) => app.app_key)).toEqual(["live.exe", "closed.exe"]);
  });

  it("sinks system sounds below real applications", () => {
    const ordered = sortApplications([
      makeApp({ app_key: "sys", display_name: "Aaa", is_system_sounds: true }),
      makeApp({ app_key: "app.exe", display_name: "Zzz" }),
    ]);
    expect(ordered.map((app) => app.app_key)).toEqual(["app.exe", "sys"]);
  });

  it("honours the user's manual order before falling back to the name", () => {
    const ordered = sortApplications([
      makeApp({ app_key: "b.exe", display_name: "Aaa", sort_order: 2 }),
      makeApp({ app_key: "a.exe", display_name: "Zzz", sort_order: 1 }),
    ]);
    expect(ordered.map((app) => app.app_key)).toEqual(["a.exe", "b.exe"]);
  });

  it("does not mutate the input array", () => {
    const input = [
      makeApp({ app_key: "b.exe", display_name: "Bbb" }),
      makeApp({ app_key: "a.exe", display_name: "Aaa" }),
    ];
    sortApplications(input);
    expect(input.map((app) => app.app_key)).toEqual(["b.exe", "a.exe"]);
  });
});

describe("applicationSections", () => {
  it("splits the list into pinned, running, and closed", () => {
    applyApplicationAdded(makeApp({ app_key: "pin.exe", pinned: true }));
    applyApplicationAdded(makeApp({ app_key: "live.exe", running: true }));
    applyApplicationAdded(makeApp({ app_key: "closed.exe", running: false }));

    expect(get(applicationSections).map((section) => section.id)).toEqual([
      "pinned",
      "running",
      "offline",
    ]);
  });

  it("omits a section that has nothing in it", () => {
    applyApplicationAdded(makeApp({ app_key: "live.exe", running: true }));
    expect(get(applicationSections).map((section) => section.id)).toEqual(["running"]);
  });
});
