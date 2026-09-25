import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import TopBar from "@/components/TopBar.svelte";
import { applications, commandPaletteOpen, currentView, settings } from "@/lib/stores";
import { setLocale } from "@/lib/i18n/index";
import { defaultSettings, makeApp, makeSession } from "../helpers/fixtures";

beforeEach(() => {
  setLocale("en");
  currentView.set("applications");
  commandPaletteOpen.set(false);
  settings.set(defaultSettings());
  applications.set([
    makeApp({ app_key: "one.exe", running: true, sessions: [makeSession({ live_id: "a" })] }),
    makeApp({ app_key: "two.exe", running: true, sessions: [makeSession({ live_id: "b" })] }),
    // Open but silent: it owns a session that produces no sound.
    makeApp({
      app_key: "idle.exe",
      running: true,
      sessions: [makeSession({ live_id: "c", state: "inactive" })],
    }),
    makeApp({ app_key: "saved.exe", running: false, sessions: [] }),
  ]);
});

function renderBar() {
  return render(TopBar, { props: { theme: "dark", onToggleTheme: vi.fn() } });
}

/**
 * The top bar replaced a navigation sidebar. It must still name every view,
 * mark the current one, and count only applications that are producing sound:
 * an idle session or a saved application is not "active".
 */
describe("TopBar", () => {
  it("names every view and marks the current one", () => {
    renderBar();
    for (const name of ["Groups", "Settings", "About"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("button", { name: "Applications, 2 active" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("counts only applications with an active session", () => {
    renderBar();
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("switches views", async () => {
    const user = userEvent.setup();
    renderBar();
    await user.click(screen.getByRole("button", { name: "Groups" }));
    expect(get(currentView)).toBe("groups");
  });

  it("opens the command palette", async () => {
    const user = userEvent.setup();
    renderBar();
    await user.click(screen.getByRole("button", { name: "Command palette" }));
    expect(get(commandPaletteOpen)).toBe(true);
  });

  it("offers the window controls, since the window has no system frame", () => {
    renderBar();
    for (const name of ["Minimize", "Maximize", "Close window"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
  });
});
