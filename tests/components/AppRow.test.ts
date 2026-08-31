import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AppRow from "@/components/AppRow.svelte";
import { setLocale } from "@/lib/i18n/index";
import { makeSession } from "../helpers/fixtures";

function renderRow(overrides = {}, props = {}) {
  const session = makeSession(overrides);
  return {
    session,
    ...render(AppRow, {
      props: {
        session,
        volume: session.volume,
        peak: 0,
        onInput: vi.fn(),
        onCommit: vi.fn(),
        onToggleMute: vi.fn(),
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

  it("flips the mute affordance when the session is silenced", () => {
    renderRow({ display_name: "Spotify", muted: true });
    const button = screen.getByRole("button", { name: "Unmute Spotify" });
    expect(button).toHaveAttribute("aria-pressed", "true");
  });

  it("announces mute in the slider value text", () => {
    renderRow({ muted: true }, { volume: 0.5 });
    expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext", "50%, muted");
  });

  it("disables both controls when Windows refuses the session", () => {
    renderRow({ controllable: false });
    expect(screen.getByRole("slider")).toBeDisabled();
    expect(screen.getByRole("button")).toBeDisabled();
    expect(screen.getByText("Locked")).toBeInTheDocument();
  });

  it("keeps an idle session interactive, matching the Windows mixer", () => {
    renderRow({ state: "inactive" });
    expect(screen.getByRole("slider")).toBeEnabled();
    expect(screen.getByText("Idle")).toBeInTheDocument();
  });

  it("names the system sounds session instead of showing a host executable", () => {
    renderRow({ is_system_sounds: true, display_name: "", executable_name: null });
    expect(screen.getByText("System Sounds")).toBeInTheDocument();
  });

  it("shows the owning group when the session belongs to one", () => {
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
