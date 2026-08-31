import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CommandRail from "@/components/CommandRail.svelte";
import { applications, currentView, settings } from "@/lib/stores";
import { setLocale } from "@/lib/i18n/index";
import { defaultSettings, makeApp } from "../helpers/fixtures";

beforeEach(() => {
  setLocale("en");
  currentView.set("applications");
  settings.set(defaultSettings());
  applications.set([
    makeApp({ app_key: "one.exe", running: true }),
    makeApp({ app_key: "two.exe", running: true }),
    makeApp({ app_key: "closed.exe", running: false }),
  ]);
});

/**
 * The first console-rail redesign shipped icon-only and gave the user no way to
 * reveal labels. These tests pin the visible toggle, labels, responsive forced
 * collapse, and the contained live count.
 */
describe("CommandRail", () => {
  it("shows navigation labels and a contained live pill when expanded", () => {
    render(CommandRail, {
      props: {
        theme: "dark",
        expanded: true,
        forcedCollapsed: false,
        onToggleTheme: vi.fn(),
        onToggleExpanded: vi.fn(),
      },
    });

    expect(screen.getByText("Applications")).toBeInTheDocument();
    expect(screen.getByText("Groups")).toBeInTheDocument();
    expect(screen.getByText("2 live")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Hide labels" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("keeps accurate accessible names while the visible labels are collapsed", () => {
    render(CommandRail, {
      props: {
        theme: "dark",
        expanded: false,
        forcedCollapsed: false,
        onToggleTheme: vi.fn(),
        onToggleExpanded: vi.fn(),
      },
    });

    expect(screen.queryByText("Applications")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Applications, 2 playing" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Show navigation labels" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("calls the shell owner when the disclosure is pressed", async () => {
    const user = userEvent.setup();
    const onToggleExpanded = vi.fn();
    render(CommandRail, {
      props: {
        theme: "dark",
        expanded: false,
        forcedCollapsed: false,
        onToggleTheme: vi.fn(),
        onToggleExpanded,
      },
    });

    await user.click(screen.getByRole("button", { name: "Show navigation labels" }));
    expect(onToggleExpanded).toHaveBeenCalledTimes(1);
  });

  it("forces the disclosure disabled at the window's narrow floor", () => {
    render(CommandRail, {
      props: {
        theme: "dark",
        expanded: false,
        forcedCollapsed: true,
        onToggleTheme: vi.fn(),
        onToggleExpanded: vi.fn(),
      },
    });

    const toggle = screen.getByRole("button", { name: "Show navigation labels" });
    expect(toggle).toBeDisabled();
    expect(toggle).toHaveAttribute("title", "Navigation labels need a wider window");
  });
});
