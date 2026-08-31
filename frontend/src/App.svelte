<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import type { Theme } from "./lib/api";
  import { setMeteringActive } from "./lib/api";
  import Sidebar from "./components/Sidebar.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Toast from "./components/Toast.svelte";
  import ShortcutOverlay from "./components/ShortcutOverlay.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import Applications from "./views/Applications.svelte";
  import Groups from "./views/Groups.svelte";
  import Settings from "./views/Settings.svelte";
  import About from "./views/About.svelte";
  import {
    commandPaletteOpen,
    currentView,
    loadSettings,
    persistSettings,
    refreshMixer,
    settings,
    shortcutOverlayOpen,
    toggleMasterMute,
  } from "./lib/stores";
  import { installEventListeners } from "./lib/events";
  import { isViewId, motionDuration } from "./lib/ux";
  import { isLocale, localeFromNavigator, setLocale } from "./lib/i18n/index";

  const THEME_KEY = "novamixer-theme";

  /**
   * Theme is stored twice on purpose. localStorage is read synchronously at
   * boot so the first paint is already correct — waiting for the backend would
   * flash the wrong theme. The backend copy is the durable one and wins once
   * settings load.
   */
  let theme = $state<Theme>(
    (typeof localStorage !== "undefined" && localStorage.getItem(THEME_KEY)) === "light"
      ? "light"
      : "dark",
  );

  function applyTheme(next: Theme): void {
    theme = next;
    document.documentElement.setAttribute("data-theme", next);
    try {
      localStorage.setItem(THEME_KEY, next);
    } catch {
      /* storage disabled; the backend copy still persists it */
    }
    const current = $settings;
    if (current) {
      persistSettings({ ...current, ui_prefs: { ...current.ui_prefs, theme: next } });
    }
  }

  function toggleTheme(): void {
    applyTheme(theme === "dark" ? "light" : "dark");
  }

  let density = $derived($settings?.ui_prefs.density ?? "comfy");

  $effect(() => {
    document.documentElement.setAttribute("data-density", density);
  });

  onMount(() => {
    document.documentElement.setAttribute("data-theme", theme);
    let cleanup: (() => void) | undefined;
    let disposed = false;

    void (async () => {
      // Listeners go up before the first snapshot so a session created during
      // startup cannot slip through the gap between the two.
      const dispose = await installEventListeners();
      if (disposed) {
        dispose();
        return;
      }
      cleanup = dispose;

      await loadSettings();
      const prefs = $settings?.ui_prefs;
      if (prefs) {
        if (prefs.theme !== theme) applyTheme(prefs.theme);
        setLocale(isLocale(prefs.language) ? prefs.language : localeFromNavigator());
      } else {
        setLocale(localeFromNavigator());
      }

      await refreshMixer();
    })();

    return () => {
      disposed = true;
      cleanup?.();
    };
  });

  /** Meters only need to run while the application list is on screen. */
  $effect(() => {
    const active = $currentView === "applications";
    void setMeteringActive(active).catch(() => {
      /* backend not ready yet; the next view change re-sends it */
    });
  });

  onMount(() => {
    const onVisibility = (): void => {
      void setMeteringActive($currentView === "applications" && !document.hidden).catch(() => {});
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  });

  /** `g` then a letter navigates; a single letter acts on the mixer. */
  let pendingGo = false;

  onMount(() => {
    let goTimer: ReturnType<typeof setTimeout> | null = null;

    const onKeydown = (event: KeyboardEvent): void => {
      const target = document.activeElement;
      const typing =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        (target instanceof HTMLElement && target.isContentEditable);

      // The palette is reachable from anywhere, including a text field, because
      // it is the fastest route to any action in the app.
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        commandPaletteOpen.update((open) => !open);
        return;
      }

      if (typing || event.metaKey || event.ctrlKey || event.altKey) return;

      if (event.key === "?") {
        event.preventDefault();
        shortcutOverlayOpen.update((open) => !open);
        return;
      }

      if (pendingGo) {
        pendingGo = false;
        if (goTimer) clearTimeout(goTimer);
        const map: Record<string, string> = {
          a: "applications",
          g: "groups",
          s: "settings",
          i: "about",
        };
        const view = map[event.key.toLowerCase()];
        if (view && isViewId(view)) {
          event.preventDefault();
          currentView.set(view);
        }
        return;
      }

      if (event.key.toLowerCase() === "g") {
        pendingGo = true;
        goTimer = setTimeout(() => (pendingGo = false), 900);
        return;
      }

      if (event.key.toLowerCase() === "m") {
        event.preventDefault();
        void toggleMasterMute();
      } else if (event.key.toLowerCase() === "d") {
        const current = $settings;
        if (!current) return;
        event.preventDefault();
        persistSettings({
          ...current,
          ui_prefs: {
            ...current.ui_prefs,
            density: current.ui_prefs.density === "comfy" ? "compact" : "comfy",
          },
        });
      }
    };

    window.addEventListener("keydown", onKeydown);
    return () => {
      window.removeEventListener("keydown", onKeydown);
      if (goTimer) clearTimeout(goTimer);
    };
  });
</script>

<div class="app-shell">
  <div class="app-ambient" aria-hidden="true">
    <div class="ambient-mesh"></div>
    <div class="ambient-grain"></div>
  </div>

  <Sidebar />
  <TopBar onToggleTheme={toggleTheme} {theme} />

  <main class="app-main">
    <div class="main-inner">
      {#if $currentView === "applications"}
        <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-applications">
          <Applications />
        </div>
      {:else if $currentView === "groups"}
        <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-groups">
          <Groups />
        </div>
      {:else if $currentView === "settings"}
        <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-settings">
          <Settings onSetTheme={applyTheme} currentTheme={theme} />
        </div>
      {:else if $currentView === "about"}
        <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-about">
          <About />
        </div>
      {/if}
    </div>
  </main>
</div>

<Toast />
<ShortcutOverlay />
<CommandPalette />

<style>
  .app-shell {
    position: fixed;
    inset: 0;
    z-index: 1;
    display: grid;
    grid-template-rows: var(--topbar-height) 1fr;
    grid-template-columns: auto minmax(0, 1fr);
    overflow: hidden;
    background: transparent;
  }
  .app-shell :global(.sidebar) {
    grid-row: 1 / -1;
    grid-column: 1;
    z-index: 70;
  }
  .app-shell :global(.topbar) {
    grid-row: 1;
    grid-column: 2;
    position: relative;
    z-index: 70;
  }

  /* A slow, very low-contrast mesh keeps a full-black window from reading as
     dead, without competing with the content. */
  .app-ambient {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    background: var(--bg-base);
  }
  .ambient-mesh {
    position: absolute;
    inset: -20%;
    background:
      radial-gradient(ellipse 72% 62% at 5% 0%, var(--ambient-glow-1), transparent 60%),
      radial-gradient(ellipse 60% 55% at 98% 6%, var(--ambient-glow-2), transparent 62%),
      radial-gradient(ellipse 82% 78% at 0% 100%, var(--ambient-glow-3), transparent 60%);
    animation: ambient-drift 64s ease-in-out infinite alternate;
  }
  @keyframes ambient-drift {
    from {
      transform: translate3d(0, 0, 0) scale(1);
    }
    to {
      transform: translate3d(2.5%, -2%, 0) scale(1.1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ambient-mesh {
      animation: none;
    }
  }
  .ambient-grain {
    position: absolute;
    inset: 0;
    background-image: var(--noise-url);
    opacity: 0.4;
    mix-blend-mode: overlay;
  }

  .app-main {
    grid-row: 2;
    grid-column: 2;
    position: relative;
    z-index: 1;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
  }
  .main-inner {
    max-width: var(--content-max);
    padding: clamp(18px, 2.4vw, 32px) clamp(18px, 3vw, 40px) clamp(24px, 3vw, 40px);
    margin: 0 auto;
  }

  @media (max-width: 720px) {
    .main-inner {
      padding: 16px 16px 24px;
    }
  }
</style>
