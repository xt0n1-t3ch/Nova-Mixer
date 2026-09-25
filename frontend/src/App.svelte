<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import type { Theme } from "./lib/api";
  import { setMeteringActive } from "./lib/api";
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
  <TopBar {theme} onToggleTheme={toggleTheme} />

  <main class="app-main">
    <div class="main-inner" class:is-desk={$currentView === "applications"}>
      {#if $currentView === "applications"}
        <!-- The mixer desk owns its own scrolling and full width; it is not a
             page centred in a content column. -->
        <div class="view-fade" data-testid="view-applications">
          <Applications />
        </div>
      {:else if $currentView === "groups"}
        <div
          class="view-pane"
          in:fly={{ y: 8, duration: motionDuration(200) }}
          data-testid="view-groups"
        >
          <Groups />
        </div>
      {:else if $currentView === "settings"}
        <div
          class="view-pane"
          in:fly={{ y: 8, duration: motionDuration(200) }}
          data-testid="view-settings"
        >
          <Settings onSetTheme={applyTheme} currentTheme={theme} />
        </div>
      {:else if $currentView === "about"}
        <div
          class="view-pane"
          in:fly={{ y: 8, duration: motionDuration(200) }}
          data-testid="view-about"
        >
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
    grid-template-rows: var(--chrome-height) minmax(0, 1fr);
    overflow: hidden;
    background: var(--bg-app);
  }
  .app-shell :global(.topbar) {
    position: relative;
    z-index: 70;
  }
  /* There is deliberately no ambient layer here. An earlier version painted
     three drifting indigo radials plus an overlay-blended noise texture across
     the whole window to keep a dark chassis from "reading as dead". It did the
     opposite: it tinted every neutral surface purple, put a 64-second animation
     behind a real-time meter, and left signal colour nothing to contrast
     against. A mixing console is a flat, unlit slab; the depth comes from the
     surface ladder and the meters, not from the backdrop. */

  .app-main {
    position: relative;
    z-index: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  /* One frame for every view: the same gutter, the same header, the same
     content width. The mixer only differs in that it spans the full width and
     scrolls its own bay, so its header stays put while the strips move.

     The height chain matters: each wrapper passes its height down, otherwise a
     view asking for `min-height: 100%` measures against an auto-sized parent
     and collapses to its content, leaving a dead band under the last card. */
  .main-inner {
    flex: 1;
    min-height: 0;
    max-width: var(--content-max);
    width: 100%;
    margin: 0 auto;
    padding: var(--page-gutter-y) var(--page-gutter-x);
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
    display: flex;
    flex-direction: column;
  }
  /* The wrapper must be a flex column itself, not merely stretched: a stretched
     block still sizes its child to content, which is what left a dead band
     under the last card on every secondary screen. */
  .view-pane,
  .view-fade {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  /* Only the content after the shared header grows; the header keeps its own
     height, or the two split the pane and push the content down. */
  .view-pane > :global(:not(.view-header)),
  .view-fade > :global(:not(.view-header)) {
    flex: 1;
    min-height: 0;
  }
  .main-inner.is-desk {
    max-width: none;
    padding-bottom: var(--space-4);
    overflow: hidden;
  }

  /* Peer views crossfade rather than each flying in from the same offset, which
     read as four copies of one page. */
  .view-fade {
    animation: view-fade var(--dur-fast) var(--ease-out);
  }
  @keyframes view-fade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .view-fade {
      animation: none;
    }
  }

</style>
