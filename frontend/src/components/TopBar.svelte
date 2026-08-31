<script lang="ts">
  /**
   * Custom window chrome. The Tauri window has `decorations: false`, so this
   * bar owns dragging and the minimize/maximize/close controls.
   */
  import { onMount } from "svelte";
  import Search from "@lucide/svelte/icons/search";
  import Moon from "@lucide/svelte/icons/moon";
  import Sun from "@lucide/svelte/icons/sun";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import { currentView, searchQuery, shortcutOverlayOpen } from "../lib/stores";
  import { t } from "../lib/i18n/index";

  let { onToggleTheme, theme }: { onToggleTheme: () => void; theme: string } = $props();

  let searchInput = $state<HTMLInputElement>();
  let maximized = $state(false);

  onMount(() => {
    const onKeydown = (event: KeyboardEvent): void => {
      const target = document.activeElement;
      const typing = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
      if (event.key === "/" && !typing) {
        event.preventDefault();
        searchInput?.focus();
      }
      if (event.key === "Escape" && target === searchInput) {
        searchQuery.set("");
        searchInput?.blur();
      }
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

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

  let showSearch = $derived($currentView === "mixer");
</script>

<header class="topbar" data-tauri-drag-region>
  <div class="topbar-left" data-tauri-drag-region>
    {#if showSearch}
      <div class="search-wrap">
        <Search class="search-icon" size={14} aria-hidden="true" />
        <input
          bind:this={searchInput}
          type="search"
          placeholder={$t("chrome.search.placeholder")}
          aria-label={$t("chrome.search.placeholder")}
          bind:value={$searchQuery}
        />
        <span class="kbd" aria-hidden="true">/</span>
      </div>
    {/if}
  </div>

  <div class="topbar-right">
    <button
      class="icon-btn"
      onclick={() => shortcutOverlayOpen.set(true)}
      title={$t("shortcut.title")}
      aria-label={$t("shortcut.title")}
    >
      <Keyboard size={15} />
    </button>

    <button
      class="icon-btn"
      onclick={onToggleTheme}
      title={$t("chrome.toggleTheme")}
      aria-label={$t("chrome.toggleTheme")}
    >
      {#if theme === "dark"}
        <Moon size={15} />
      {:else}
        <Sun size={15} />
      {/if}
    </button>

    <div class="window-controls">
      <button class="win-btn" onclick={minimize} aria-label={$t("chrome.minimize")} title={$t("chrome.minimize")}>
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <line x1="2" y1="6" x2="10" y2="6" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </button>
      <button
        class="win-btn"
        onclick={toggleMaximize}
        aria-label={maximized ? $t("chrome.restore") : $t("chrome.maximize")}
        title={maximized ? $t("chrome.restore") : $t("chrome.maximize")}
      >
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <rect x="2" y="2" width="8" height="8" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </button>
      <button class="win-btn win-close" onclick={closeWindow} aria-label={$t("chrome.closeWindow")} title={$t("chrome.closeWindow")}>
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <line x1="2" y1="2" x2="10" y2="10" stroke="currentColor" stroke-width="1.5" />
          <line x1="10" y1="2" x2="2" y2="10" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </button>
    </div>
  </div>
</header>

<style>
  .topbar {
    height: var(--topbar-height);
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 0 20px;
    background: var(--glass-2);
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    border-bottom: 1px solid var(--border);
    box-shadow: var(--glass-edge);
    gap: 16px;
    user-select: none;
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .topbar {
      background: var(--glass-fallback);
    }
  }

  .topbar-left {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
  }

  .search-wrap {
    position: relative;
    width: 100%;
    max-width: 320px;
    display: flex;
    align-items: center;
  }
  .search-wrap :global(.search-icon) {
    position: absolute;
    left: 14px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-wrap input {
    width: 100%;
    height: 34px;
    padding: 0 38px 0 36px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    border: 1px solid transparent;
    font-size: var(--fs-sm);
    color: var(--text-primary);
  }
  .search-wrap input:hover {
    background: var(--bg-card-hover);
  }
  .search-wrap input:focus {
    background: var(--bg-card);
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .search-wrap .kbd {
    position: absolute;
    right: 12px;
    pointer-events: none;
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .topbar-right .icon-btn {
    width: 30px;
    height: 30px;
  }

  .window-controls {
    display: flex;
    gap: 2px;
    margin-left: 8px;
    padding-left: 8px;
    border-left: 1px solid var(--border);
  }
  .win-btn {
    width: 34px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    color: var(--text-placeholder);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .win-btn:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  .win-btn:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .win-close:hover {
    color: #fff;
    background: var(--danger);
  }
</style>
