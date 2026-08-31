<script lang="ts">
  /**
   * The chrome strip: 44px of window controls and live status, not a toolbar.
   *
   * Search is a trigger rather than a permanently mounted field. On a console
   * the horizontal space belongs to the desk, and search is something you reach
   * for, so `/` expands it in place and Escape puts it away again.
   */
  import { onMount, tick } from "svelte";
  import Search from "@lucide/svelte/icons/search";
  import X from "@lucide/svelte/icons/x";
  import Speaker from "@lucide/svelte/icons/speaker";
  import { currentView, master, searchQuery } from "../lib/stores";
  import { t } from "../lib/i18n/index";

  let searchInput = $state<HTMLInputElement>();
  let searchOpen = $state(false);
  let maximized = $state(false);

  let showSearch = $derived($currentView === "applications");

  async function openSearch(): Promise<void> {
    searchOpen = true;
    await tick();
    searchInput?.focus();
  }

  function closeSearch(): void {
    searchQuery.set("");
    searchOpen = false;
  }

  onMount(() => {
    const onKeydown = (event: KeyboardEvent): void => {
      const target = document.activeElement;
      const typing = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
      if (event.key === "/" && !typing && showSearch) {
        event.preventDefault();
        void openSearch();
      }
      if (event.key === "Escape" && target === searchInput) {
        closeSearch();
      }
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

  // A search left empty on blur collapses again, so the strip returns to status.
  function onBlur(): void {
    if (!$searchQuery.trim()) searchOpen = false;
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

<header class="chrome" data-tauri-drag-region>
  <div class="chrome-left" data-tauri-drag-region>
    <span class="context" data-tauri-drag-region>{$t("nav." + $currentView)}</span>
    {#if $master}
      <span class="endpoint truncate" title={$master.endpoint_name}>
        <Speaker size={11} aria-hidden="true" />
        {$master.endpoint_name}
      </span>
    {/if}
  </div>

  <div class="chrome-right">
    {#if showSearch}
      {#if searchOpen}
        <div class="search-field">
          <Search class="search-icon" size={13} aria-hidden="true" />
          <input
            bind:this={searchInput}
            type="search"
            bind:value={$searchQuery}
            placeholder={$t("chrome.search.placeholder")}
            aria-label={$t("chrome.search.placeholder")}
            onblur={onBlur}
          />
          <button class="search-clear" onclick={closeSearch} aria-label={$t("common.close")}>
            <X size={12} />
          </button>
        </div>
      {:else}
        <button
          class="chrome-btn"
          onclick={() => void openSearch()}
          aria-label={$t("chrome.search.placeholder")}
          title="{$t('chrome.search.placeholder')} · /"
        >
          <Search size={14} />
        </button>
      {/if}
    {/if}

    <div class="window-controls">
      <button
        class="win-btn"
        onclick={minimize}
        aria-label={$t("chrome.minimize")}
        title={$t("chrome.minimize")}
      >
        <svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true">
          <line x1="2" y1="6" x2="10" y2="6" stroke="currentColor" stroke-width="1.4" />
        </svg>
      </button>
      <button
        class="win-btn"
        onclick={toggleMaximize}
        aria-label={maximized ? $t("chrome.restore") : $t("chrome.maximize")}
        title={maximized ? $t("chrome.restore") : $t("chrome.maximize")}
      >
        <svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true">
          <rect
            x="2.5"
            y="2.5"
            width="7"
            height="7"
            rx="1"
            fill="none"
            stroke="currentColor"
            stroke-width="1.4"
          />
        </svg>
      </button>
      <button
        class="win-btn win-close"
        onclick={closeWindow}
        aria-label={$t("chrome.closeWindow")}
        title={$t("chrome.closeWindow")}
      >
        <svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true">
          <line x1="2.5" y1="2.5" x2="9.5" y2="9.5" stroke="currentColor" stroke-width="1.4" />
          <line x1="9.5" y1="2.5" x2="2.5" y2="9.5" stroke="currentColor" stroke-width="1.4" />
        </svg>
      </button>
    </div>
  </div>
</header>

<style>
  .chrome {
    height: var(--chrome-height);
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px 0 var(--space-4);
    background: var(--glass-2);
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    border-bottom: 1px solid var(--border);
    gap: var(--space-3);
    user-select: none;
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .chrome {
      background: var(--glass-fallback);
    }
  }

  .chrome-left {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
    flex: 1;
  }

  /* The context label and endpoint replace a page heading: on a console you
     always know which desk you are at and what it is driving. */
  .context {
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .endpoint {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-xs);
    color: var(--text-placeholder);
    min-width: 0;
    max-width: 320px;
  }

  .chrome-right {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .chrome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .chrome-btn:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  .chrome-btn:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  .search-field {
    position: relative;
    display: flex;
    align-items: center;
    width: clamp(180px, 26vw, 300px);
  }
  .search-field :global(.search-icon) {
    position: absolute;
    left: 10px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-field input {
    height: 30px;
    padding: 0 28px 0 30px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    border: 1px solid transparent;
    font-size: var(--fs-sm);
  }
  .search-field input:focus {
    background: var(--bg-card);
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .search-clear {
    position: absolute;
    right: 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-full);
    color: var(--text-muted);
  }
  .search-clear:hover {
    color: var(--text-primary);
    background: var(--bg-card-hover);
  }

  .window-controls {
    display: flex;
    gap: 1px;
    margin-left: var(--space-2);
  }
  .win-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 30px;
    border-radius: var(--radius-sm);
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

  @media (max-width: 760px) {
    .endpoint {
      display: none;
    }
  }
</style>
