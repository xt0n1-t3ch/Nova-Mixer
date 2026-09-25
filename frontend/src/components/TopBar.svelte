<script lang="ts">
  /**
   * The one bar at the top of the window: brand, views, commands,
   * preferences, and the window controls.
   *
   * It replaces a 212px navigation sidebar and a separate 40px title bar. Four
   * views do not need a column of their own, and the width the sidebar took
   * belongs to the faders. The window is undecorated, so this bar is also the
   * drag region and carries the only minimize, maximize and close buttons.
   */
  import { onMount } from "svelte";
  import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";
  import Layers from "@lucide/svelte/icons/layers";
  import Settings from "@lucide/svelte/icons/settings";
  import Info from "@lucide/svelte/icons/info";
  import Search from "@lucide/svelte/icons/search";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import Moon from "@lucide/svelte/icons/moon";
  import Sun from "@lucide/svelte/icons/sun";
  import type { Component } from "svelte";
  import logo from "../assets/novamixer-logo.png";
  import {
    applications,
    commandPaletteOpen,
    currentView,
    persistSettings,
    settings,
    shortcutOverlayOpen,
  } from "../lib/stores";
  import type { ViewId } from "../lib/ux";
  import { locale, LOCALE_LABELS, setLocale, t, type Locale } from "../lib/i18n/index";

  let { theme, onToggleTheme }: { theme: string; onToggleTheme: () => void } = $props();

  type Tab = { id: ViewId; icon: Component<{ size?: number }>; count?: boolean };
  const TABS: Tab[] = [
    { id: "applications", icon: SlidersHorizontal, count: true },
    { id: "groups", icon: Layers },
    { id: "settings", icon: Settings },
    { id: "about", icon: Info },
  ];

  // Active means a session is producing sound now, not merely an open app.
  let activeCount = $derived(
    $applications.filter((app) => app.sessions.some((session) => session.state === "active"))
      .length,
  );

  let maximized = $state(false);

  onMount(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      maximized = await getCurrentWindow().isMaximized();
    } catch {
      maximized = false;
    }
  });

  function switchLanguage(): void {
    const next: Locale = $locale === "en" ? "es" : "en";
    setLocale(next);
    const current = $settings;
    if (current) {
      persistSettings({ ...current, ui_prefs: { ...current.ui_prefs, language: next } });
    }
  }

  async function minimize(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }

  async function toggleMaximize(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    if (await win.isMaximized()) {
      await win.unmaximize();
      maximized = false;
    } else {
      await win.maximize();
      maximized = true;
    }
  }

  async function closeWindow(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }
</script>

<header class="topbar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <img class="brand-logo" src={logo} alt="" width="22" height="22" />
    <span class="brand-name" data-tauri-drag-region>NovaMixer</span>
  </div>

  <nav class="tabs" aria-label={$t("app.name")}>
    {#each TABS as tab (tab.id)}
      {@const Icon = tab.icon}
      {@const active = $currentView === tab.id}
      <button
        class="tab"
        class:active
        data-testid="nav-{tab.id}"
        aria-current={active ? "page" : undefined}
        aria-label={tab.count && activeCount > 0
          ? $t("nav.withCount", { label: $t("nav." + tab.id), count: activeCount })
          : $t("nav." + tab.id)}
        title={$t("nav." + tab.id)}
        onclick={() => currentView.set(tab.id)}
      >
        <Icon size={15} />
        <span class="tab-label">{$t("nav." + tab.id)}</span>
        {#if tab.count && activeCount > 0}
          <!-- The bare number; the accessible name carries the full phrase. -->
          <span class="tab-count mono" aria-hidden="true">
            {activeCount > 99 ? "99+" : activeCount}
          </span>
        {/if}
      </button>
    {/each}
  </nav>

  <span class="drag-fill" data-tauri-drag-region></span>

  <div class="tools">
    <!-- The palette finds any application or action, so it reads as search. -->
    <button
      class="command"
      aria-label={$t("palette.title")}
      title={$t("palette.title")}
      onclick={() => commandPaletteOpen.set(true)}
    >
      <Search size={14} aria-hidden="true" />
      <span class="command-label">{$t("nav.commands")}</span>
      <span class="kbd">Ctrl K</span>
    </button>

    <button
      class="tool"
      aria-label={$t("chrome.toggleTheme")}
      title={$t("chrome.toggleTheme")}
      onclick={onToggleTheme}
    >
      {#if theme === "dark"}<Moon size={15} />{:else}<Sun size={15} />{/if}
    </button>
    <button
      class="tool tool-locale mono"
      aria-label="{$t('chrome.language')}: {LOCALE_LABELS[$locale]}"
      title="{$t('chrome.language')}: {LOCALE_LABELS[$locale]}"
      onclick={switchLanguage}
    >
      <span aria-hidden="true">{$locale.toUpperCase()}</span>
    </button>
    <button
      class="tool"
      aria-label={$t("shortcut.title")}
      title={$t("shortcut.title")}
      onclick={() => shortcutOverlayOpen.set(true)}
    >
      <Keyboard size={15} />
    </button>
  </div>

  <div class="window-controls">
    <button
      class="win-btn"
      onclick={minimize}
      aria-label={$t("chrome.minimize")}
      title={$t("chrome.minimize")}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <line x1="0" y1="5" x2="10" y2="5" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
    <button
      class="win-btn"
      onclick={toggleMaximize}
      aria-label={maximized ? $t("chrome.restore") : $t("chrome.maximize")}
      title={maximized ? $t("chrome.restore") : $t("chrome.maximize")}
    >
      {#if maximized}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="currentColor" />
          <path d="M2.5 2.5V0.5h7v7h-2" fill="none" stroke="currentColor" />
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" />
        </svg>
      {/if}
    </button>
    <button
      class="win-btn win-close"
      onclick={closeWindow}
      aria-label={$t("chrome.closeWindow")}
      title={$t("chrome.closeWindow")}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <line x1="0" y1="0" x2="10" y2="10" stroke="currentColor" stroke-width="1" />
        <line x1="10" y1="0" x2="0" y2="10" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: stretch;
    height: var(--chrome-height);
    padding-left: var(--space-4);
    background: var(--bg-sidebar);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding-right: var(--space-5);
    flex-shrink: 0;
  }
  .brand-logo {
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
  }
  /* The mark is white on black. On a light bar that square reads as a hole,
     so the light theme inverts it to black on white, matching the chassis. */
  :global([data-theme="light"]) .brand-logo {
    filter: invert(1);
  }
  .brand-name {
    font-size: var(--fs-sm);
    font-weight: 650;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
  }

  /* Tabs are underlined, not boxed: a boxed tab strip in a title bar reads as
     a second toolbar, while an underline reads as "you are here". */
  .tabs {
    display: flex;
    align-items: stretch;
    gap: 2px;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 var(--space-3);
    font-size: var(--fs-sm);
    color: var(--text-muted);
    transition: color var(--dur-fast) var(--ease);
  }
  .tab:hover {
    color: var(--text-primary);
  }
  .tab.active {
    color: var(--text-primary);
    font-weight: 550;
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: var(--space-3);
    right: var(--space-3);
    bottom: -1px;
    height: 2px;
    border-radius: 2px 2px 0 0;
    background: var(--accent);
  }
  .tab:focus-visible {
    outline: none;
    box-shadow: inset var(--shadow-ring);
  }
  /* A neutral count: colour on this chassis means signal, and a number of
     applications is a fact, not a level. */
  .tab-count {
    min-width: 18px;
    height: 17px;
    padding: 0 5px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated-2);
    color: var(--text-secondary);
    font-size: var(--fs-2xs);
    font-weight: 650;
    line-height: 17px;
    text-align: center;
  }

  .drag-fill {
    flex: 1;
    min-width: var(--space-4);
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: var(--space-2);
  }
  .command {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 28px;
    margin-right: var(--space-2);
    padding: 0 4px 0 10px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--text-muted);
    transition:
      color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .command:hover {
    color: var(--text-primary);
    border-color: var(--border-hover);
  }
  .command .kbd {
    margin-left: var(--space-3);
  }
  .tool {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 28px;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .tool:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  .tool-locale {
    font-size: var(--fs-2xs);
    font-weight: 700;
  }

  /* Full-height caption buttons flush to the frame, as Windows draws its own,
     so the close corner is reachable by throwing the pointer. */
  .window-controls {
    display: flex;
    border-left: 1px solid var(--border);
  }
  .win-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 46px;
    color: var(--text-muted);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .win-btn:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  /* Close is the one window control that gets a colour, because destroying the
     window is the one irreversible thing in this bar. */
  .win-close:hover {
    color: #fff;
    background: var(--danger);
  }

  /* Labels go before any control does: the icons stay, and each tab keeps its
     accessible name and tooltip. */
  @media (max-width: 1060px) {
    .tab-label,
    .command-label,
    .command .kbd {
      display: none;
    }
    .command {
      padding: 0 8px;
      margin-right: 2px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tab,
    .tool,
    .command,
    .win-btn {
      transition: none;
    }
  }
</style>
