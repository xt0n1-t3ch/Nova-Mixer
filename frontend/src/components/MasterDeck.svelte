<script lang="ts">
  /**
   * The master deck: the output section of the console.
   *
   * It spans the desk rather than sitting in a rounded card, because it is not
   * one more item in the list — it is the thing every channel below it feeds
   * into. Its fader shares the same left edge as the channel faders, so the
   * whole desk reads on one set of column guides.
   */
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import Speaker from "@lucide/svelte/icons/speaker";
  import type { MasterState } from "../lib/api";
  import { formatPercent, volumeValueText } from "../lib/volume";
  import { VOLUME_PRESETS } from "../lib/ux";
  import Meter from "./Meter.svelte";
  import Range from "./Range.svelte";
  import { t } from "../lib/i18n/index";

  let {
    master,
    peak = 0,
    runningCount = 0,
    onInput,
    onCommit,
    onToggleMute,
  }: {
    master: MasterState;
    peak?: number;
    runningCount?: number;
    onInput: (value: number) => void;
    onCommit: (value: number) => void;
    onToggleMute: () => void;
  } = $props();

  let muteLabel = $derived(master.muted ? $t("master.unmute") : $t("master.mute"));
</script>

<section class="deck" class:is-muted={master.muted} aria-label={$t("master.label")}>
  <div class="deck-identity">
    <span class="deck-label">{$t("master.label")}</span>
    <span class="deck-endpoint truncate" title={master.endpoint_name}>
      <Speaker size={12} aria-hidden="true" />
      {master.endpoint_name}
    </span>
    <span class="deck-status">
      {$t("master.feeding", { count: runningCount })}
    </span>
  </div>

  <div class="deck-controls">
    <button
      class="deck-mute"
      class:is-danger={master.muted}
      onclick={onToggleMute}
      aria-label={muteLabel}
      aria-pressed={master.muted}
      title={muteLabel}
    >
      {#if master.muted}
        <VolumeX size={19} />
      {:else}
        <Volume2 size={19} />
      {/if}
    </button>

    <div class="deck-fader">
      <Range
        value={master.volume}
        muted={master.muted}
        size="lg"
        ariaLabel={$t("master.volume")}
        ariaValueText={volumeValueText(master.volume, master.muted, $t("app.muted"))}
        {onInput}
        {onCommit}
      />
      <div class="deck-presets">
        {#each VOLUME_PRESETS as preset (preset)}
          <button
            class="preset"
            class:active={Math.round(master.volume * 100) === Math.round(preset * 100)}
            onclick={() => {
              onInput(preset);
              onCommit(preset);
            }}
          >
            {formatPercent(preset)}
          </button>
        {/each}
      </div>
    </div>

    <output class="deck-value display-num" for="">{formatPercent(master.volume)}</output>

    <div class="deck-meter">
      <Meter peak={master.muted ? 0 : peak} bars={9} orientation="vertical" />
    </div>
  </div>
</section>

<style>
  /* Ruled edges rather than a card outline: the deck is part of the console
     surface, and the rule below it is the console's own division. */
  .deck {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    background: var(--deck-bg);
    border-bottom: 1px solid var(--deck-line-strong);
    position: relative;
  }
  /* A leading accent edge marks the master as the driving section; it turns red
     when the whole output is silenced, which is worth noticing from anywhere. */
  .deck::before {
    content: "";
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: linear-gradient(
      to bottom,
      var(--deck-edge, var(--accent)),
      color-mix(in oklab, var(--deck-edge, var(--accent)) 20%, transparent)
    );
  }
  .deck.is-muted {
    --deck-edge: var(--danger);
  }

  .deck-identity {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    flex-wrap: wrap;
    min-width: 0;
  }
  .deck-label {
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .deck-endpoint {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    min-width: 0;
    max-width: 420px;
  }
  .deck-status {
    font-size: var(--fs-xs);
    color: var(--text-placeholder);
    margin-left: auto;
  }

  .deck-controls {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr) auto 24px;
    align-items: center;
    gap: var(--space-4);
  }

  .deck-mute {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: var(--radius-lg);
    color: var(--text-secondary);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .deck-mute:hover {
    color: var(--text-primary);
    border-color: var(--border-hover);
  }
  .deck-mute:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .deck-mute.is-danger {
    color: var(--danger);
    background: var(--danger-dim);
    border-color: transparent;
  }

  .deck-fader {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  .deck-presets {
    display: flex;
    gap: var(--space-1);
  }
  .preset {
    height: 24px;
    padding: 0 10px;
    border-radius: var(--radius-full);
    font-size: var(--fs-2xs);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    background: transparent;
    border: 1px solid var(--border);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .preset:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  .preset.active {
    color: var(--accent);
    background: var(--accent-dim);
    border-color: transparent;
  }
  .preset:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  .deck-value {
    font-size: clamp(30px, 3.4vw, 46px);
    line-height: 1;
    align-self: center;
    font-variant-numeric: tabular-nums;
  }
  .is-muted .deck-value {
    color: var(--text-muted);
  }

  .deck-meter {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 56px;
  }

  /* At the window's minimum height the deck must give its space back to the
     channels rather than keeping desktop padding around a single fader. */
  @media (max-width: 900px), (max-height: 700px) {
    .deck {
      padding: var(--space-3) var(--space-4);
      gap: var(--space-2);
    }
    .deck-presets {
      display: none;
    }
    .deck-status {
      display: none;
    }
    .deck-value {
      font-size: clamp(26px, 3vw, 34px);
    }
    .deck-meter {
      height: 40px;
    }
  }

  @media (max-width: 760px) {
    .deck-controls {
      grid-template-columns: 40px minmax(0, 1fr) auto;
    }
    .deck-meter {
      display: none;
    }
  }
</style>
