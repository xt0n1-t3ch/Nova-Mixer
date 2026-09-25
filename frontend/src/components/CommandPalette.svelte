<script lang="ts">
  /**
   * Keyboard-first control surface, opened with Ctrl+K.
   *
   * It indexes applications and scenes alongside static actions, so reaching
   * "mute Discord" is three keystrokes instead of a hunt down a list. Matching
   * is subsequence-based, which is what makes "dsc" find Discord.
   */
  import { tick } from "svelte";
  import Search from "@lucide/svelte/icons/search";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import Play from "@lucide/svelte/icons/play";
  import SlidersVertical from "@lucide/svelte/icons/sliders-vertical";
  import Layers from "@lucide/svelte/icons/layers";
  import Settings from "@lucide/svelte/icons/settings";
  import Info from "@lucide/svelte/icons/info";
  import CornerDownLeft from "@lucide/svelte/icons/corner-down-left";
  import { focusTrap } from "../actions/focusTrap";
  import { fuzzyRank, motionDuration, type ViewId } from "../lib/ux";
  import {
    applications,
    commandPaletteOpen,
    commitAppVolume,
    currentView,
    inspectedAppKey,
    runScene,
    scenes,
    toggleAppMute,
    toggleMasterMute,
  } from "../lib/stores";
  import { formatPercent } from "../lib/volume";
  import { t } from "../lib/i18n/index";
  import { fade, scale } from "svelte/transition";

  interface Command {
    id: string;
    title: string;
    hint?: string;
    group: string;
    icon: "volume" | "mute" | "scene" | "view" | "settings" | "about";
    run: () => void;
  }

  let query = $state("");
  let activeIndex = $state(0);
  let input = $state<HTMLInputElement>();

  function go(view: ViewId): void {
    currentView.set(view);
    close();
  }

  let commands = $derived.by<Command[]>(() => {
    const list: Command[] = [
      {
        id: "nav.applications",
        title: $t("nav.applications"),
        group: $t("palette.navigate"),
        icon: "view",
        run: () => go("applications"),
      },
      {
        id: "nav.groups",
        title: $t("nav.groups"),
        group: $t("palette.navigate"),
        icon: "view",
        run: () => go("groups"),
      },
      {
        id: "nav.settings",
        title: $t("nav.settings"),
        group: $t("palette.navigate"),
        icon: "settings",
        run: () => go("settings"),
      },
      {
        id: "nav.about",
        title: $t("nav.about"),
        group: $t("palette.navigate"),
        icon: "about",
        run: () => go("about"),
      },
      {
        id: "action.muteAll",
        title: $t("palette.muteMaster"),
        group: $t("palette.actions"),
        icon: "mute",
        run: () => {
          void toggleMasterMute();
          close();
        },
      },
    ];

    for (const scene of $scenes) {
      list.push({
        id: `scene.${scene.id}`,
        title: $t("palette.applyScene", { name: scene.name }),
        group: $t("scene.label"),
        icon: "scene",
        run: () => {
          void runScene(scene.id);
          close();
        },
      });
    }

    for (const app of $applications) {
      list.push({
        id: `app.mute.${app.app_key}`,
        title: app.muted
          ? $t("app.unmute", { app: app.display_name })
          : $t("app.mute", { app: app.display_name }),
        hint: app.running ? formatPercent(app.volume) : $t("app.offline"),
        group: $t("nav.applications"),
        icon: app.muted ? "mute" : "volume",
        run: () => {
          void toggleAppMute(app);
          close();
        },
      });
      // A few round levels cover most of what a user actually reaches for.
      for (const level of [0.25, 0.5, 1]) {
        list.push({
          id: `app.set.${app.app_key}.${level}`,
          title: $t("palette.setLevel", {
            app: app.display_name,
            level: formatPercent(level),
          }),
          group: $t("nav.applications"),
          icon: "volume",
          run: () => {
            void commitAppVolume(app.app_key, level);
            close();
          },
        });
      }
      list.push({
        id: `app.configure.${app.app_key}`,
        title: $t("app.configure", { app: app.display_name }),
        group: $t("nav.applications"),
        icon: "settings",
        run: () => {
          currentView.set("applications");
          inspectedAppKey.set(app.app_key);
          close();
        },
      });
    }

    return list;
  });

  let results = $derived(fuzzyRank(query, commands, (command) => command.title).slice(0, 40));

  $effect(() => {
    void results;
    activeIndex = 0;
  });

  $effect(() => {
    if ($commandPaletteOpen) {
      query = "";
      void tick().then(() => input?.focus());
    }
  });

  function close(): void {
    commandPaletteOpen.set(false);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      activeIndex = results.length === 0 ? 0 : (activeIndex + 1) % results.length;
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      activeIndex = results.length === 0 ? 0 : (activeIndex - 1 + results.length) % results.length;
    } else if (event.key === "Enter") {
      event.preventDefault();
      results[activeIndex]?.run();
    }
  }
</script>

{#if $commandPaletteOpen}
  <div
    class="palette-backdrop"
    role="presentation"
    transition:fade={{ duration: motionDuration(120) }}
    onclick={close}
  ></div>

  <div
    class="palette glass-dialog"
    role="dialog"
    aria-modal="true"
    aria-label={$t("palette.title")}
    tabindex="-1"
    onkeydown={onKeydown}
    use:focusTrap
    transition:scale={{ duration: motionDuration(140), start: 0.98, opacity: 0 }}
  >
    <div class="palette-input">
      <Search size={15} aria-hidden="true" />
      <input
        bind:this={input}
        type="text"
        bind:value={query}
        placeholder={$t("palette.placeholder")}
        aria-label={$t("palette.placeholder")}
        aria-controls="palette-results"
        autocomplete="off"
        spellcheck="false"
      />
      <span class="kbd">Esc</span>
    </div>

    <ul class="palette-results" id="palette-results" role="listbox" aria-label={$t("palette.title")}>
      {#each results as command, index (command.id)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={index === activeIndex}
            class="palette-item"
            class:active={index === activeIndex}
            onclick={command.run}
            onpointerenter={() => (activeIndex = index)}
          >
            <span class="item-icon" aria-hidden="true">
              {#if command.icon === "mute"}
                <VolumeX size={14} />
              {:else if command.icon === "volume"}
                <Volume2 size={14} />
              {:else if command.icon === "scene"}
                <Play size={14} />
              {:else if command.icon === "settings"}
                <Settings size={14} />
              {:else if command.icon === "about"}
                <Info size={14} />
              {:else if command.group === $t("scene.label")}
                <Layers size={14} />
              {:else}
                <SlidersVertical size={14} />
              {/if}
            </span>
            <span class="item-title truncate">{command.title}</span>
            {#if command.hint}
              <span class="item-hint mono">{command.hint}</span>
            {/if}
            <span class="item-group truncate">{command.group}</span>
          </button>
        </li>
      {/each}

      {#if results.length === 0}
        <li class="palette-empty">{$t("palette.noResults", { query })}</li>
      {/if}
    </ul>

    <footer class="palette-foot">
      <span><span class="kbd">↑</span><span class="kbd">↓</span> {$t("palette.navigateHint")}</span>
      <span><span class="kbd"><CornerDownLeft size={9} /></span> {$t("palette.runHint")}</span>
    </footer>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    z-index: 260;
    background: rgba(0, 0, 0, 0.5);
    border: none;
  }
  :global([data-theme="light"]) .palette-backdrop {
    background: rgba(15, 23, 42, 0.24);
  }

  /* Placed above centre: the eye lands there first, and the results grow
     downward without shifting the input. */
  .palette {
    position: fixed;
    top: 14vh;
    left: 50%;
    transform: translateX(-50%);
    z-index: 261;
    width: min(560px, 92vw);
    max-height: 60vh;
    display: flex;
    flex-direction: column;
  }

  .palette-input {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-4);
    height: 52px;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .palette-input input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    padding: 0;
    font-size: var(--fs-md);
    color: var(--text-primary);
  }
  .palette-input input:focus {
    outline: none;
    box-shadow: none;
  }

  .palette-results {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    list-style: none;
    margin: 0;
    padding: var(--space-2);
  }

  .palette-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    text-align: left;
    color: var(--text-secondary);
  }
  .palette-item.active {
    background: var(--accent-dim);
    color: var(--text-primary);
  }
  .palette-item:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  .item-icon {
    display: inline-flex;
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .palette-item.active .item-icon {
    color: var(--accent);
  }
  .item-title {
    flex: 1;
    min-width: 0;
    font-size: var(--fs-sm);
  }
  .item-hint {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .item-group {
    font-size: var(--fs-2xs);
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: var(--letter-wide);
    max-width: 90px;
    flex-shrink: 0;
  }

  .palette-empty {
    padding: var(--space-5);
    text-align: center;
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }

  .palette-foot {
    display: flex;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    border-top: 1px solid var(--border);
    font-size: var(--fs-2xs);
    color: var(--text-faint);
    flex-shrink: 0;
  }
  .palette-foot span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
</style>
