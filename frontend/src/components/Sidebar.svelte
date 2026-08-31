<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import SlidersVertical from "@lucide/svelte/icons/sliders-vertical";
  import Layers from "@lucide/svelte/icons/layers";
  import Settings from "@lucide/svelte/icons/settings";
  import Info from "@lucide/svelte/icons/info";
  import Languages from "@lucide/svelte/icons/languages";
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import type { Component } from "svelte";
  import { currentView, settings, persistSettings, sortedSessions } from "../lib/stores";
  import { motionDuration, type ViewId } from "../lib/ux";
  import { locale, LOCALE_LABELS, setLocale, t, type Locale } from "../lib/i18n/index";

  type NavItem = { id: ViewId; icon: Component<{ size?: number }>; count?: boolean };

  const audioGroup: NavItem[] = [
    { id: "mixer", icon: SlidersVertical, count: true },
    { id: "groups", icon: Layers },
  ];
  const generalGroup: NavItem[] = [
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

  let collapsed = $derived($settings?.ui_prefs.sidebar_collapsed ?? false);
  let sessionCount = $derived($sortedSessions.length);

  function toggleCollapsed(): void {
    const current = $settings;
    if (!current) return;
    persistSettings({
      ...current,
      ui_prefs: { ...current.ui_prefs, sidebar_collapsed: !collapsed },
    });
  }

  function switchLanguage(): void {
    const next: Locale = $locale === "en" ? "es" : "en";
    setLocale(next);
    const current = $settings;
    if (current) {
      persistSettings({ ...current, ui_prefs: { ...current.ui_prefs, language: next } });
    }
  }
</script>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-brand" data-tauri-drag-region>
    <span class="brand-pill" aria-hidden="true">
      <SlidersVertical size={20} />
    </span>
    {#if !collapsed}
      <span class="brand-wrap">
        <span class="brand-name">{$t("app.name")}</span>
        <span class="brand-version mono">v{appVersion}</span>
      </span>
    {/if}
  </div>

  {#snippet navItem(item: NavItem, index: number)}
    {@const Icon = item.icon}
    <button
      class="nav-pill"
      class:active={$currentView === item.id}
      data-testid="nav-{item.id}"
      aria-current={$currentView === item.id ? "page" : undefined}
      title={$t("nav." + item.id)}
      onclick={() => currentView.set(item.id)}
    >
      <Icon size={20} />
      {#if !collapsed}
        <span class="nav-label-text" in:fly={{ x: -8, duration: motionDuration(180), delay: index * 24 }}>
          {$t("nav." + item.id)}
        </span>
      {/if}
      {#if item.count && sessionCount > 0}
        <span class="nav-count" class:is-collapsed={collapsed} aria-hidden="true">{sessionCount}</span>
      {/if}
    </button>
  {/snippet}

  <nav class="sidebar-nav" aria-label={$t("app.name")}>
    {#if !collapsed}<div class="nav-label">{$t("nav.group.audio")}</div>{/if}
    {#each audioGroup as item, index (item.id)}
      {@render navItem(item, index)}
    {/each}
    {#if !collapsed}<div class="nav-label">{$t("nav.group.general")}</div>{/if}
    {#each generalGroup as item, index (item.id)}
      {@render navItem(item, audioGroup.length + index)}
    {/each}
  </nav>

  <button
    class="nav-pill lang-switcher"
    onclick={switchLanguage}
    title={$t("chrome.language")}
    aria-label="{$t('chrome.language')}: {LOCALE_LABELS[$locale]}"
  >
    <Languages size={20} />
    {#if !collapsed}
      <span class="nav-label-text">{$t("chrome.language")}</span>
    {/if}
    <span class="lang-pill mono" class:is-collapsed={collapsed}>{$locale.toUpperCase()}</span>
  </button>

  <button
    class="sidebar-toggle"
    onclick={toggleCollapsed}
    title={collapsed ? $t("nav.expand") : $t("nav.collapse")}
    aria-label={collapsed ? $t("nav.expand") : $t("nav.collapse")}
  >
    {#if collapsed}
      <ChevronRight size={18} />
    {:else}
      <ChevronLeft size={18} />
      <span class="toggle-label">{$t("nav.collapseLabel")}</span>
    {/if}
  </button>
</aside>

<style>
  .sidebar {
    position: relative;
    flex-shrink: 0;
    width: var(--sidebar-width);
    height: 100%;
    background: var(--glass-1);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-right: 1px solid var(--border);
    box-shadow: var(--glass-edge);
    display: flex;
    flex-direction: column;
    z-index: 2;
    transition: width var(--dur-normal) var(--ease);
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .sidebar {
      background: var(--glass-fallback);
    }
  }
  .sidebar.collapsed {
    width: var(--sidebar-width-collapsed);
  }

  .sidebar-brand {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: var(--topbar-height);
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border);
  }
  .sidebar.collapsed .sidebar-brand {
    padding: 0;
    justify-content: center;
  }
  .brand-pill {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-lg);
    background: linear-gradient(145deg, var(--bg-input), var(--bg-elevated));
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow:
      inset 0 0 0 1px color-mix(in oklab, var(--text-primary) 28%, transparent),
      var(--shadow-sm);
  }
  .brand-wrap {
    display: flex;
    flex-direction: column;
    line-height: var(--lh-tight);
    min-width: 0;
  }
  .brand-name {
    font-size: var(--fs-lg);
    font-weight: 700;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
  }
  .brand-version {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    margin-top: 2px;
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-4);
    overflow-y: auto;
  }
  .sidebar.collapsed .sidebar-nav {
    padding: var(--space-4) var(--space-2);
    align-items: center;
  }

  .nav-label {
    font-size: var(--fs-2xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    padding: var(--space-3) var(--space-3) var(--space-1);
  }
  .nav-label:not(:first-child) {
    margin-top: var(--space-2);
  }

  .nav-pill {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    height: 42px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--fs-md);
    font-weight: 500;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease),
      transform var(--dur-fast) var(--spring);
  }
  .sidebar.collapsed .nav-pill {
    width: 42px;
    height: 42px;
    justify-content: center;
    padding: 0;
  }
  .nav-pill:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
    transform: translateX(2px);
  }
  .nav-pill:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .nav-pill.active {
    background: var(--accent-dim);
    color: var(--accent);
    font-weight: 600;
    transform: none;
  }
  .nav-pill.active::before {
    content: "";
    position: absolute;
    left: calc(-1 * var(--space-3));
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--accent);
  }
  .sidebar.collapsed .nav-pill.active::before {
    left: -7px;
  }
  .nav-label-text {
    flex: 1;
    text-align: left;
  }

  .nav-count,
  .lang-pill {
    font-family: var(--font-mono);
    font-size: var(--fs-2xs);
    font-weight: 700;
    padding: 1px 7px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-muted);
    min-width: 18px;
    text-align: center;
    line-height: var(--lh-tight);
    flex-shrink: 0;
  }
  .nav-pill.active .nav-count {
    background: var(--bg-card);
    color: var(--accent);
  }
  .nav-count.is-collapsed,
  .lang-pill.is-collapsed {
    position: absolute;
    top: 2px;
    right: 2px;
    padding: 0 4px;
    min-width: 14px;
  }

  .lang-switcher {
    margin: auto var(--space-4) 0;
    width: auto;
    border-top: 1px solid var(--border);
    border-radius: 0;
    padding: var(--space-4) var(--space-3) var(--space-3);
    height: auto;
  }
  .sidebar.collapsed .lang-switcher {
    width: 42px;
    height: auto;
    margin: auto var(--space-2) 0;
    padding: var(--space-4) 0 0;
    justify-content: center;
    border-radius: 0;
  }
  .lang-switcher:hover {
    transform: none;
  }

  .sidebar-toggle {
    margin: var(--space-2) var(--space-4) var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    height: 40px;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    font-weight: 600;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .sidebar-toggle:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }
  .sidebar-toggle:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .sidebar.collapsed .sidebar-toggle {
    width: 42px;
    height: 42px;
    padding: 0;
    margin: var(--space-2) var(--space-2) var(--space-4);
    justify-content: center;
  }
  .toggle-label {
    white-space: nowrap;
  }
</style>
