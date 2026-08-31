<script lang="ts">
  /**
   * Peak level meter.
   *
   * The backend sends raw peaks in batches; this component owns the ballistics.
   * Peaks attack instantly so a transient is never dropped, then decay
   * exponentially, which is how a hardware VU meter behaves and what makes a
   * level readable rather than a strobe. A hold marker keeps the recent maximum
   * visible so a short spike leaves a trace.
   *
   * Animation runs on rAF driven by the incoming value, not on a standing timer,
   * so a silent app costs nothing.
   */
  import { onDestroy } from "svelte";
  import { clampScalar, decayPeak, meterBand, nextHold } from "../lib/volume";
  import { reducedMotion } from "../lib/ux";

  let {
    peak = 0,
    bars = 5,
    orientation = "horizontal",
    ariaLabel,
  }: {
    /** Raw peak from the backend, 0.0–1.0. */
    peak?: number;
    bars?: number;
    orientation?: "horizontal" | "vertical";
    /** Omit to leave the meter decorative; meters duplicate the volume readout. */
    ariaLabel?: string;
  } = $props();

  const still = reducedMotion();

  let displayed = $state(0);
  let hold = $state({ value: 0, since: 0 });
  let frame = 0;
  let lastTick = 0;

  function tick(now: number): void {
    const deltaMs = lastTick === 0 ? 16 : now - lastTick;
    lastTick = now;

    const target = clampScalar(peak);
    const next = decayPeak(displayed, target, deltaMs);
    displayed = next;
    hold = nextHold(hold, next, now);

    // Stop once the meter has settled at the incoming level; the next non-zero
    // peak restarts the loop through the effect below.
    if (next <= 0.002 && target <= 0.002) {
      frame = 0;
      lastTick = 0;
      displayed = 0;
      hold = { value: 0, since: now };
      return;
    }
    frame = requestAnimationFrame(tick);
  }

  $effect(() => {
    const incoming = clampScalar(peak);
    if (still) {
      displayed = incoming;
      hold = { value: incoming, since: 0 };
      return;
    }
    if (incoming > 0 && frame === 0) {
      frame = requestAnimationFrame(tick);
    }
  });

  onDestroy(() => {
    if (frame !== 0) cancelAnimationFrame(frame);
  });

  let band = $derived(meterBand(displayed));
  let segments = $derived(
    Array.from({ length: bars }, (_, index) => {
      const threshold = (index + 1) / bars;
      // Partial fill on the leading segment keeps a 5-bar meter from looking
      // like a 5-step quantiser.
      const filled = displayed >= threshold ? 1 : Math.max(0, (displayed - index / bars) * bars);
      return { index, filled };
    }),
  );
</script>

<div
  class="meter"
  data-orientation={orientation}
  data-band={band}
  role={ariaLabel ? "meter" : "presentation"}
  aria-label={ariaLabel}
  aria-valuenow={ariaLabel ? Math.round(displayed * 100) : undefined}
  aria-valuemin={ariaLabel ? 0 : undefined}
  aria-valuemax={ariaLabel ? 100 : undefined}
>
  {#each segments as segment (segment.index)}
    <span class="meter-bar">
      <span class="meter-bar-fill" style:--fill="{segment.filled * 100}%"></span>
    </span>
  {/each}
  {#if !still && hold.value > 0.02}
    <span class="meter-hold" style:--hold="{hold.value * 100}%" aria-hidden="true"></span>
  {/if}
</div>

<style>
  .meter {
    position: relative;
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }
  .meter[data-orientation="horizontal"] {
    width: 40px;
    height: 16px;
    align-items: flex-end;
  }
  .meter[data-orientation="vertical"] {
    flex-direction: column-reverse;
    width: 16px;
    height: 48px;
  }

  .meter-bar {
    position: relative;
    flex: 1;
    border-radius: 1px;
    background: var(--meter-idle);
    overflow: hidden;
  }
  .meter[data-orientation="horizontal"] .meter-bar {
    height: 100%;
  }

  .meter-bar-fill {
    position: absolute;
    inset: auto 0 0 0;
    height: var(--fill);
    background: var(--meter-color, var(--meter-low));
    transition: height var(--dur-instant) linear;
  }
  .meter[data-orientation="vertical"] .meter-bar-fill {
    inset: 0 0 auto 0;
    height: auto;
    width: var(--fill);
  }

  .meter[data-band="low"] {
    --meter-color: var(--meter-low);
  }
  .meter[data-band="mid"] {
    --meter-color: var(--meter-mid);
  }
  .meter[data-band="hot"] {
    --meter-color: var(--meter-hot);
  }

  .meter-hold {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: var(--meter-hold);
    transform: translateY(calc(-1 * var(--hold)));
    pointer-events: none;
  }

  @media (forced-colors: active) {
    .meter-bar {
      border: 1px solid CanvasText;
    }
    .meter-bar-fill {
      background: Highlight;
      forced-color-adjust: none;
    }
  }
</style>
