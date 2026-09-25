import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Settings from "@/views/Settings.svelte";
import { settings } from "@/lib/stores";
import { setLocale } from "@/lib/i18n/index";
import { defaultSettings } from "../helpers/fixtures";

beforeEach(() => {
  setLocale("en");
  settings.set(defaultSettings());
});

/**
 * The Hotkeys section used to crash on open: the shortcut list keyed each key
 * by its label, and "g then g" has two identical keys, so Svelte threw and the
 * panel never replaced Startup. Every section must open and show its controls.
 */
describe("Settings sections", () => {
  it("opens Hotkeys and shows the three hotkey captures", async () => {
    const user = userEvent.setup();
    render(Settings, { props: { onSetTheme: vi.fn(), currentTheme: "dark" } });

    await user.click(screen.getByRole("button", { name: "Hotkeys" }));

    expect(screen.getByRole("heading", { name: "Hotkeys" })).toBeInTheDocument();
    for (const action of ["Volume up", "Volume down", "Toggle mute"]) {
      expect(screen.getByRole("button", { name: new RegExp(`^${action}:`) })).toBeInTheDocument();
    }
    expect(screen.queryByRole("heading", { name: "Startup" })).not.toBeInTheDocument();
  });

  it("opens Appearance and Storage", async () => {
    const user = userEvent.setup();
    render(Settings, { props: { onSetTheme: vi.fn(), currentTheme: "dark" } });

    await user.click(screen.getByRole("button", { name: "Appearance" }));
    expect(screen.getByRole("heading", { name: "Theme" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Storage" }));
    expect(screen.getByRole("heading", { name: "Storage" })).toBeInTheDocument();
  });
});
