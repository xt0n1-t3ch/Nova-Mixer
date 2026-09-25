<script lang="ts">
  /**
   * The output: the one control that sums every channel below it.
   *
   * It is the channel row's big sibling, on the same grammar — identity, a
   * fader with its meter fused beneath it, the readout, mute — so it reads as
   * the same instrument, only louder. The presets are a stepped control for the
   * value the fader already sets.
   *
   * The meter is the endpoint's own `IAudioMeterInformation::GetPeakValue`,
   * one value for the device. It is drawn once; two ladders would pretend to
   * show left and right channels that the backend does not measure.
   */
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import Headphones from "@lucide/svelte/icons/headphones";
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

  let muteLabel = $derived(master.muted ? $t("master.unmute") : $t("master.mute"));
</script>

<section class="master surface" class:is-muted={master.muted} aria-label={$t("master.label")}>
  <div class="master-id">
    <span class="master-badge" aria-hidden="true"><Headphones size={18} /></span>
    <span class="master-text">
      <span class="master-title">{$t("master.label")}</span>
      <span class="master-endpoint truncate" title={master.endpoint_name}>
        {master.endpoint_name}
      </span>
    </span>
  </div>

  <div class="master-level">
    <Range
      value={master.volume}
      muted={master.muted}
      size="lg"
      resetTo={1}
      ariaLabel={$t("master.volume")}
      ariaValueText={volumeValueText(master.volume, master.muted, $t("app.muted"))}
      {onInput}
      {onCommit}
    />
    <div class="master-meter">
      <Meter peak={master.muted ? 0 : peak} bars={48} />
    </div>
  </div>

  <output class="master-value mono" for="">{formatPercent(master.volume)}</output>

  <button
    class="master-mute"
    class:is-danger={master.muted}
    onclick={onToggleMute}
    aria-label={muteLabel}
    aria-pressed={master.muted}
    title={muteLabel}
  >
    {#if master.muted}<VolumeX size={17} />{:else}<Volume2 size={17} />{/if}
  </button>

  <div class="presets" role="group" aria-label={$t("master.presets")}>
    {#each VOLUME_PRESETS as preset (preset)}
      {@const active = Math.round(master.volume * 100) === Math.round(preset * 100)}
      <button
        class="preset mono"
        class:active
        aria-pressed={active}
        onclick={() => {
          onInput(preset);
          onCommit(preset);
        }}
      >
        {Math.round(preset * 100)}
      </button>
    {/each}
  </div>
</section>

<style>
  /* The output row uses the channel grid's rhythm with a wider identity and a
     presets column, so its fader visibly heads the column of faders below. */
  .master {
    display: grid;
    grid-template-columns: minmax(200px, 0.9fr) minmax(200px, 2.4fr) 56px 40px auto;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4) var(--space-3) var(--space-5);
  }
  /* Muted output is the state a user is most likely to miss, so it marks the
     whole panel, not only the button. */
  .master.is-muted {
    border-color: var(--danger);
  }

  .master-id {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }
  .master-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg);
  }
  .master.is-muted .master-badge {
    background: var(--danger);
    color: var(--danger-on);
  }
  .master-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .master-title {
    font-size: var(--fs-md);
    font-weight: 650;
    color: var(--text-primary);
    line-height: var(--lh-tight);
  }
  .master-endpoint {
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }

  .master-level {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .master-meter {
    height: 7px;
    padding: 0 2px;
  }
  .master-meter :global(.meter) {
    height: 100%;
  }

  .master-value {
    font-size: var(--fs-lg);
    font-weight: 650;
    color: var(--text-primary);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .master.is-muted .master-value {
    color: var(--text-faint);
  }

  .master-mute {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .master-mute:hover {
    background: var(--bg-elevated-2);
    color: var(--text-primary);
  }
  /* `--danger-on`, not `--accent-fg`: the glyph sits on a solid red fill and
     the readable ink there differs per theme. */
  .master-mute.is-danger {
    color: var(--danger-on);
    background: var(--danger);
    border-color: transparent;
  }

  .presets {
    display: inline-flex;
    padding: 2px;
    gap: 1px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }
  .preset {
    height: 26px;
    padding: 0 8px;
    border-radius: var(--radius-xs);
    font-size: var(--fs-2xs);
    font-weight: 600;
    color: var(--text-muted);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .preset:hover {
    color: var(--text-primary);
  }
  .preset.active {
    color: var(--accent-fg);
    background: var(--accent);
  }

  /* The presets go first on a narrow window: they are a shortcut for a value
     the fader already sets. */
  @media (max-width: 1100px) {
    .master {
      grid-template-columns: minmax(150px, 0.8fr) minmax(160px, 2.4fr) 52px 40px;
    }
    .presets {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .preset,
    .master-mute {
      transition: none;
    }
  }
</style>
