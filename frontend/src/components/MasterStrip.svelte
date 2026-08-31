<script lang="ts">
  /**
   * The endpoint fader, pinned above the per-app list.
   *
   * It reads as a different object from an app row on purpose: larger type, a
   * bigger track and a vertical meter, so it is never mistaken for one more
   * application in the list.
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
    onInput,
    onCommit,
    onToggleMute,
  }: {
    master: MasterState;
    peak?: number;
    onInput: (value: number) => void;
    onCommit: (value: number) => void;
    onToggleMute: () => void;
  } = $props();

  let muteLabel = $derived(
    master.muted ? $t("mixer.unmuteMaster") : $t("mixer.muteMaster"),
  );
</script>

<section class="master edge-accent" class:is-muted={master.muted}>
  <div class="master-head">
    <div class="master-copy">
      <span class="master-label">{$t("mixer.master")}</span>
      <span class="master-device truncate">
        <Speaker size={12} aria-hidden="true" />
        {master.endpoint_name}
      </span>
    </div>
    <output class="display-num master-value" for="">{formatPercent(master.volume)}</output>
  </div>

  <div class="master-controls">
    <button
      class="icon-btn"
      class:is-danger={master.muted}
      onclick={onToggleMute}
      aria-label={muteLabel}
      aria-pressed={master.muted}
      title={muteLabel}
    >
      {#if master.muted}
        <VolumeX size={18} />
      {:else}
        <Volume2 size={18} />
      {/if}
    </button>

    <div class="master-fader">
      <Range
        value={master.volume}
        muted={master.muted}
        size="lg"
        ariaLabel={$t("mixer.masterVolume")}
        ariaValueText={volumeValueText(master.volume, master.muted, $t("mixer.muted"))}
        {onInput}
        {onCommit}
      />
    </div>

    <Meter peak={master.muted ? 0 : peak} bars={6} orientation="vertical" />
  </div>

  <div class="master-presets">
    {#each VOLUME_PRESETS as preset (preset)}
      <button
        class="pill"
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
</section>

<style>
  .master {
    position: relative;
    overflow: hidden;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-2xl);
    padding: var(--space-5) var(--space-5) var(--space-4);
    box-shadow: var(--card-edge);
    margin-bottom: var(--space-5);
  }
  .master.is-muted {
    --edge-color: var(--danger);
  }

  .master-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  .master-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .master-label {
    font-size: var(--fs-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .master-device {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    max-width: 340px;
  }

  .master-value {
    line-height: 1;
    flex-shrink: 0;
  }
  .is-muted .master-value {
    color: var(--text-muted);
  }

  .master-controls {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  .master-fader {
    flex: 1;
    min-width: 0;
  }

  .master-presets {
    display: flex;
    gap: var(--space-1);
    margin-top: var(--space-4);
    flex-wrap: wrap;
  }

  @media (max-width: 640px) {
    .master {
      padding: var(--space-4);
    }
    .master-presets {
      display: none;
    }
  }
</style>
