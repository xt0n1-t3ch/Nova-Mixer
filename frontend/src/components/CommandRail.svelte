<script lang="ts">
  /**
   * Collapsible mixer navigation.
   *
   * At 64px this is a compact instrument rail. At 208px it reveals labels and
   * turns the running count and locale into contained trailing pills. The
   * toggle is a first-class row below the brand, not a hidden preference: if a
   * user cannot see how to reveal navigation labels, the rail is not usable.
   */
  import { onMount } from "svelte";
  import type { Component } from "svelte";
  import SlidersVertical from "@lucide/svelte/icons/sliders-vertical";
  import Layers from "@lucide/svelte/icons/layers";
  import Settings from "@lucide/svelte/icons/settings";
  import Info from "@lucide/svelte/icons/info";
  import Command from "@lucide/svelte/icons/command";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import Moon from "@lucide/svelte/icons/moon";
  import Sun from "@lucide/svelte/icons/sun";
  import Languages from "@lucide/svelte/icons/languages";
  import PanelLeftOpen from "@lucide/svelte/icons/panel-left-open";
  import PanelLeftClose from "@lucide/svelte/icons/panel-left-close";
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

  let {
    onToggleTheme,
    theme,
    expanded,
    forcedCollapsed,
    onToggleExpanded,
  }: {
    onToggleTheme: () => void;
    theme: string;
    expanded: boolean;
    forcedCollapsed: boolean;
    onToggleExpanded: () => void;
  } = $props();

  type RailItem = { id: ViewId; icon: Component<{ size?: number }>; badge?: boolean };

  const primary: RailItem[] = [
    { id: "applications", icon: SlidersVertical, badge: true },
    { id: "groups", icon: Layers },
  ];
  const secondary: RailItem[] = [
    { id: "settings", icon: Settings },
    { id: "about", icon: Info },
  ];

  let appVersion = $state("dev");
  onMount(async () => {
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      appVersion = await getVersion();
    } catch {
      appVersion = "dev";
    }
  });

  let runningCount = $derived($applications.filter((app) => app.running).length);

  function switchLanguage(): void {
    const next: Locale = $locale === "en" ? "es" : "en";
    setLocale(next);
    const current = $settings;
    if (current) {
      persistSettings({ ...current, ui_prefs: { ...current.ui_prefs, language: next } });
    }
  }
</script>

<nav class="rail" class:expanded aria-label={$t("app.name")}>
  <div class="rail-brand" data-tauri-drag-region>
    <span class="brand-mark" aria-hidden="true">
      <SlidersVertical size={19} />
    </span>
    {#if expanded}
      <span class="brand-copy" data-tauri-drag-region>
        <span class="brand-name">NovaMixer</span>
        <span class="brand-version mono">v{appVersion}</span>
      </span>
    {/if}
  </div>

  <button
    class="rail-item rail-toggle"
    aria-expanded={expanded}
    aria-label={expanded ? $t("nav.hideLabels") : $t("nav.showLabels")}
    title={forcedCollapsed ? $t("nav.labelsNeedSpace") : expanded ? $t("nav.hideLabels") : $t("nav.showLabels")}
    disabled={forcedCollapsed}
    onclick={onToggleExpanded}
  >
    <span class="item-icon">
      {#if expanded}<PanelLeftClose size={18} />{:else}<PanelLeftOpen size={18} />{/if}
    </span>
    {#if expanded}<span class="item-label">{$t("nav.hideLabels")}</span>{/if}
  </button>

  {#snippet railButton(item: RailItem)}
    {@const Icon = item.icon}
    {@const active = $currentView === item.id}
    <button
      class="rail-item"
      class:active
      data-testid="nav-{item.id}"
      aria-current={active ? "page" : undefined}
      aria-label={item.badge && runningCount > 0
        ? $t("nav.withCount", { label: $t("nav." + item.id), count: runningCount })
        : $t("nav." + item.id)}
      title={!expanded ? $t("nav." + item.id) : undefined}
      onclick={() => currentView.set(item.id)}
    >
      <span class="item-icon"><Icon size={19} /></span>
      {#if expanded}<span class="item-label">{$t("nav." + item.id)}</span>{/if}
      {#if item.badge && runningCount > 0}
        <span class="item-badge mono" aria-hidden="true">
          {expanded ? $t("nav.liveCount", { count: runningCount }) : runningCount > 99 ? "99+" : runningCount}
        </span>
      {/if}
    </button>
  {/snippet}

  <div class="rail-section">
    {#each primary as item (item.id)}{@render railButton(item)}{/each}
  </div>

  <div class="rail-rule" aria-hidden="true"></div>

  <div class="rail-section">
    <button
      class="rail-item"
      aria-label={$t("palette.title")}
      title={!expanded ? $t("palette.title") : undefined}
      onclick={() => commandPaletteOpen.set(true)}
    >
      <span class="item-icon"><Command size={18} /></span>
      {#if expanded}<span class="item-label">{$t("nav.commands")}</span><span class="item-shortcut kbd">Ctrl K</span>{/if}
    </button>
    <button
      class="rail-item"
      aria-label={$t("shortcut.title")}
      title={!expanded ? $t("shortcut.title") : undefined}
      onclick={() => shortcutOverlayOpen.set(true)}
    >
      <span class="item-icon"><Keyboard size={18} /></span>
      {#if expanded}<span class="item-label">{$t("shortcut.title")}</span>{/if}
    </button>
  </div>

  <div class="rail-spacer"></div>

  <div class="rail-section">
    <button class="rail-item" aria-label={$t("chrome.toggleTheme")} title={!expanded ? $t("chrome.toggleTheme") : undefined} onclick={onToggleTheme}>
      <span class="item-icon">{#if theme === "dark"}<Moon size={18} />{:else}<Sun size={18} />{/if}</span>
      {#if expanded}<span class="item-label">{theme === "dark" ? $t("settings.themeDark") : $t("settings.themeLight")}</span>{/if}
    </button>
    <button class="rail-item" aria-label="{$t('chrome.language')}: {LOCALE_LABELS[$locale]}" title={!expanded ? $t("chrome.language") : undefined} onclick={switchLanguage}>
      <span class="item-icon"><Languages size={18} /></span>
      {#if expanded}<span class="item-label">{LOCALE_LABELS[$locale]}</span>{/if}
      <span class="item-badge mono is-locale" aria-hidden="true">{$locale.toUpperCase()}</span>
    </button>
  </div>

  <div class="rail-rule" aria-hidden="true"></div>
  <div class="rail-section rail-section-end">
    {#each secondary as item (item.id)}{@render railButton(item)}{/each}
  </div>
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-1);
    width: 100%;
    height: 100%;
    padding: 0 8px var(--space-3);
    background: var(--glass-1);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-right: 1px solid var(--border);
    overflow: hidden;
    z-index: 3;
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .rail { background: var(--glass-fallback); }
  }

  .rail-brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: var(--chrome-height);
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    margin: 0 -8px var(--space-1);
    padding: 0 14px;
  }
  .brand-mark,
  .item-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    flex: 0 0 32px;
  }
  .brand-mark {
    border-radius: var(--radius-md);
    background: linear-gradient(145deg, var(--bg-input), var(--bg-elevated));
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--text-primary) 26%, transparent), var(--shadow-sm);
  }
  .brand-copy { display: flex; flex-direction: column; min-width: 0; line-height: var(--lh-tight); }
  .brand-name { font-size: var(--fs-sm); font-weight: 700; color: var(--text-primary); }
  .brand-version { font-size: 9px; color: var(--text-muted); }

  .rail-section { display: flex; flex-direction: column; gap: 2px; }
  .rail-spacer { flex: 1; }
  .rail-rule { height: 1px; background: var(--border); margin: var(--space-1) 8px; flex-shrink: 0; }

  .rail-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    height: 40px;
    padding: 0 4px;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    text-align: left;
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .rail-item:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-elevated); }
  .rail-item:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .rail-item:disabled { opacity: 0.35; cursor: not-allowed; }
  .rail-item.active { color: var(--accent); background: var(--accent-dim); }
  .rail-item.active::before {
    content: "";
    position: absolute;
    left: -8px;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 0 var(--radius-full) var(--radius-full) 0;
    background: var(--accent);
  }
  .item-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--fs-sm); font-weight: 500; }
  .item-shortcut { margin-right: 4px; flex-shrink: 0; }
  .item-badge {
    min-width: 16px;
    max-width: 74px;
    height: 16px;
    padding: 0 5px;
    border-radius: var(--radius-full);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 9px;
    font-weight: 700;
    line-height: 16px;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    flex-shrink: 0;
  }
  .item-badge.is-locale { background: var(--bg-elevated); color: var(--text-muted); margin-right: 4px; }
  .rail:not(.expanded) .item-badge {
    position: absolute;
    top: 2px;
    right: 2px;
    max-width: 24px;
    padding: 0 4px;
  }
  .rail:not(.expanded) .item-badge.is-locale { top: 1px; right: 1px; }

  @media (prefers-reduced-motion: reduce) {
    .rail-item { transition: none; }
  }
</style>
