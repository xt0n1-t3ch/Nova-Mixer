import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AppRow from "@/components/AppRow.svelte";
import { setLocale } from "@/lib/i18n/index";
import { makeApp, makeSession } from "../helpers/fixtures";

function renderRow(overrides = {}, props = {}) {
  const app = makeApp(overrides);
  return {
    app,
    ...render(AppRow, {
      props: {
        app,
        volume: app.volume,
        peak: 0,
        sessionPeaks: {},
        onInput: vi.fn(),
        onCommit: vi.fn(),
        onToggleMute: vi.fn(),
        onToggleExpanded: vi.fn(),
        onInspect: vi.fn(),
        onTogglePin: vi.fn(),
        onSessionInput: vi.fn(),
        onSessionCommit: vi.fn(),
        onSessionMute: vi.fn(),
        ...props,
      },
    }),
  };
}

beforeEach(() => {
  setLocale("en");
});

describe("AppRow", () => {
  it("shows the application name and its executable", () => {
    renderRow({ display_name: "Spotify", executable_name: "Spotify.exe" });
    expect(screen.getByText("Spotify")).toBeInTheDocument();
    expect(screen.getByText("Spotify.exe")).toBeInTheDocument();
  });

  it("labels the slider with the application, so several rows stay distinguishable", () => {
    renderRow({ display_name: "Spotify" });
    expect(screen.getByRole("slider", { name: "Volume for Spotify" })).toBeInTheDocument();
  });

  it("renders the volume as the same percentage the Windows mixer shows", () => {
    renderRow({}, { volume: 0.625 });
    expect(screen.getByText("63%")).toBeInTheDocument();
  });

  it("uses the pending value during a drag rather than the backend value", () => {
    renderRow({ volume: 0.2 }, { volume: 0.8 });
    expect(screen.getByText("80%")).toBeInTheDocument();
  });

  it("offers mute and reports its pressed state", async () => {
    const user = userEvent.setup();
    const onToggleMute = vi.fn();
    renderRow({ display_name: "Spotify" }, { onToggleMute });

    const button = screen.getByRole("button", { name: "Mute Spotify" });
    expect(button).toHaveAttribute("aria-pressed", "false");

    await user.click(button);
    expect(onToggleMute).toHaveBeenCalledTimes(1);
  });

  it("flips the mute affordance when the application is silenced", () => {
    renderRow({ display_name: "Spotify", muted: true });
    const button = screen.getByRole("button", { name: "Unmute Spotify" });
    expect(button).toHaveAttribute("aria-pressed", "true");
  });

  it("announces mute in the slider value text", () => {
    renderRow({ muted: true }, { volume: 0.5 });
    expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext", "50%, muted");
  });

  it("disables the slider when Windows refuses the session", () => {
    renderRow({ controllable: false, running: true });
    expect(screen.getByRole("slider")).toBeDisabled();
    expect(screen.getByText("Locked")).toBeInTheDocument();
  });

  it("keeps a closed application listed, with its settings reachable", () => {
    renderRow({ running: false, display_name: "Spotify" });
    expect(screen.getByText("Spotify")).toBeInTheDocument();
    expect(screen.getByText("Closed")).toBeInTheDocument();
    // The settings button is what makes an offline row worth showing at all.
    expect(screen.getByRole("button", { name: "Configure Spotify" })).toBeInTheDocument();
  });

  it("names the system sounds session instead of showing a host executable", () => {
    renderRow({ is_system_sounds: true, display_name: "", executable_name: null });
    expect(screen.getByText("System Sounds")).toBeInTheDocument();
  });

  it("shows the owning group when the application belongs to one", () => {
    renderRow({ group_id: "group-1" }, { groupName: "Music" });
    expect(screen.getByText("In Music")).toBeInTheDocument();
  });

  it("drops the secondary line in compact density", () => {
    renderRow({ executable_name: "Spotify.exe" }, { compact: true });
    expect(screen.queryByText("Spotify.exe")).not.toBeInTheDocument();
  });

  it("translates its labels", () => {
    setLocale("es");
    renderRow({ display_name: "Spotify" });
    expect(screen.getByRole("slider", { name: "Volumen de Spotify" })).toBeInTheDocument();
  });
});

/** The Discord defect: one application, several sessions, one row. */
describe("an application with several sessions", () => {
  const twoSessions = {
    app_key: "discord.exe",
    display_name: "Discord",
    sessions: [
      makeSession({ live_id: "ep::d1", app_key: "discord.exe", display_name: "Voice" }),
      makeSession({ live_id: "ep::d2", app_key: "discord.exe", display_name: "Stream" }),
    ],
  };

  it("renders one row and says how many streams it owns", () => {
    renderRow(twoSessions);
    expect(screen.getAllByText("Discord")).toHaveLength(1);
    expect(screen.getByText("2 streams")).toBeInTheDocument();
  });

  it("offers a disclosure for the streams", async () => {
    const user = userEvent.setup();
    const onToggleExpanded = vi.fn();
    renderRow(twoSessions, { onToggleExpanded });

    const toggle = screen.getByRole("button", { name: "Show the audio streams of Discord" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    await user.click(toggle);
    expect(onToggleExpanded).toHaveBeenCalledTimes(1);
  });

  it("keeps the streams hidden until expanded", () => {
    renderRow(twoSessions, { expanded: false });
    expect(screen.queryByText("Voice")).not.toBeInTheDocument();
  });

  it("lists each stream once expanded", () => {
    renderRow(twoSessions, { expanded: true });
    expect(screen.getByText("Voice")).toBeInTheDocument();
    expect(screen.getByText("Stream")).toBeInTheDocument();
  });

  it("offers no disclosure for a single session, which has nothing to reveal", () => {
    renderRow({ sessions: [makeSession({ live_id: "ep::one" })] });
    expect(
      screen.queryByRole("button", { name: /Show the audio streams/ }),
    ).not.toBeInTheDocument();
  });

  it("shows an indeterminate control when sessions disagree", () => {
    renderRow({ mixed: true });
    expect(screen.getByText("Mixed")).toBeInTheDocument();
    expect(screen.getByRole("slider")).toHaveAttribute(
      "aria-valuetext",
      "Sessions are at different levels",
    );
  });
});

describe("row management actions", () => {
  it("offers pin and reports its pressed state", async () => {
    const user = userEvent.setup();
    const onTogglePin = vi.fn();
    renderRow({ display_name: "Spotify" }, { onTogglePin });

    const pin = screen.getByRole("button", { name: "Pin Spotify" });
    expect(pin).toHaveAttribute("aria-pressed", "false");
    await user.click(pin);
    expect(onTogglePin).toHaveBeenCalledTimes(1);
  });

  it("flips the pin label once pinned", () => {
    renderRow({ display_name: "Spotify", pinned: true });
    expect(screen.getByRole("button", { name: "Unpin Spotify" })).toBeInTheDocument();
  });

  it("opens the inspector, which is where renaming and removal live", async () => {
    const user = userEvent.setup();
    const onInspect = vi.fn();
    renderRow({ display_name: "Spotify" }, { onInspect });

    await user.click(screen.getByRole("button", { name: "Configure Spotify" }));
    expect(onInspect).toHaveBeenCalledTimes(1);
  });
});
