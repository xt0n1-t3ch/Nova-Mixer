<script lang="ts">
  /**
   * Peak level meter.
   *
   * The backend sends Windows peak readings in batches; this component owns the
   * ballistics. Peaks attack instantly so a transient is never dropped, then
   * decay exponentially, which is how a hardware meter behaves and what makes a
   * level readable rather than a strobe. A hold marker keeps the recent maximum
   * visible so a short spike leaves a trace.
   *
   * It draws into one `<canvas>` from the shared meter clock (`lib/meterClock`):
   * at most 30 paints a second, only while the level is moving, and none while
   * the window is hidden. The earlier version restyled up to 48 elements per
   * meter on every display frame, which kept the renderer and GPU busy.
   *
   * This is the one component allowed to be loud. On a monochrome chassis the
   * meter is the only saturated thing in a row, which is exactly how an engineer
   * finds the source that is actually making noise.
   */
  import { onMount } from "svelte";
  import { clampScalar, decayPeak, meterBand, nextHold } from "../lib/volume";
  import { reducedMotion } from "../lib/ux";
  import { addMeter, wakeMeters } from "../lib/meterClock";

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

  let canvas = $state<HTMLCanvasElement>();
  let displayed = 0;
  let hold = { value: 0, since: 0 };
  let lastPaint = 0;
  let drawn = -1;
  let drawnHold = -1;
  let colours = { idle: "", low: "", mid: "", hot: "", hold: "" };
  let labelValue = $state(0);

  function readColours(): void {
    if (!canvas) return;
    const style = getComputedStyle(canvas);
    colours = {
      idle: style.getPropertyValue("--meter-idle").trim(),
      low: style.getPropertyValue("--meter-low").trim(),
      mid: style.getPropertyValue("--meter-mid").trim(),
      hot: style.getPropertyValue("--meter-hot").trim(),
      hold: style.getPropertyValue("--meter-hold").trim(),
    };
    drawn = -1;
  }

  function draw(): void {
    const el = canvas;
    if (!el) return;
    const ratio = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(el.clientWidth * ratio));
    const height = Math.max(1, Math.round(el.clientHeight * ratio));
    if (el.width !== width || el.height !== height) {
      el.width = width;
      el.height = height;
      drawn = -1;
    }
    const level = displayed;
    const holdLevel = still ? 0 : hold.value;
    if (level === drawn && holdLevel === drawnHold) return;
    drawn = level;
    drawnHold = holdLevel;

    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, width, height);
    const vertical = orientation === "vertical";
    const span = vertical ? height : width;
    const gap = Math.max(1, Math.round(2 * ratio));
    const segment = (span - gap * (bars - 1)) / bars;
    const band = meterBand(level);
    const fillColour = band === "hot" ? colours.hot : band === "mid" ? colours.mid : colours.low;

    for (let index = 0; index < bars; index += 1) {
      const start = index * (segment + gap);
      const threshold = (index + 1) / bars;
      // Partial fill on the leading segment keeps a coarse meter from reading
      // like a stepped quantiser.
      const filled = level >= threshold ? 1 : Math.max(0, (level - index / bars) * bars);
      if (vertical) {
        const y = height - start - segment;
        ctx.fillStyle = colours.idle;
        ctx.fillRect(0, y, width, segment);
        if (filled > 0) {
          ctx.fillStyle = fillColour;
          ctx.fillRect(0, y + segment * (1 - filled), width, segment * filled);
        }
      } else {
        ctx.fillStyle = colours.idle;
        ctx.fillRect(start, 0, segment, height);
        if (filled > 0) {
          ctx.fillStyle = fillColour;
          ctx.fillRect(start, 0, segment * filled, height);
        }
      }
    }

    if (holdLevel > 0.02) {
      ctx.fillStyle = colours.hold;
      const at = holdLevel * span;
      if (vertical) ctx.fillRect(0, height - at, width, Math.max(1, ratio * 2));
      else ctx.fillRect(Math.min(width - ratio * 2, at), 0, Math.max(1, ratio * 2), height);
    }
  }

  /** One clock tick. Returns true while the meter still has motion to show. */
  function paint(now: number): boolean {
    const target = clampScalar(peak);
    if (still) {
      displayed = target;
    } else {
      const delta = lastPaint === 0 ? 33 : now - lastPaint;
      displayed = decayPeak(displayed, target, delta);
      hold = nextHold(hold, displayed, now);
    }
    lastPaint = now;
    draw();
    if (ariaLabel) labelValue = Math.round(displayed * 100);
    const settled = displayed <= 0.002 && target <= 0.002 && hold.value <= 0.02;
    if (settled) {
      displayed = 0;
      hold = { value: 0, since: now };
      lastPaint = 0;
      draw();
    }
    return !settled && !still;
  }

  onMount(() => {
    readColours();
    draw();
    const stop = addMeter(paint);
    // Theme changes swap the token values; re-read them instead of caching stale colours.
    const observer = new MutationObserver(readColours);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    const resize = new ResizeObserver(() => {
      drawn = -1;
      draw();
    });
    if (canvas) resize.observe(canvas);
    return () => {
      stop();
      observer.disconnect();
      resize.disconnect();
    };
  });

  // Each new reading wakes the shared clock; a silent meter costs nothing.
  $effect(() => {
    if (clampScalar(peak) > 0 || displayed > 0) wakeMeters();
  });
</script>

<canvas
  bind:this={canvas}
  class="meter"
  data-orientation={orientation}
  role={ariaLabel ? "meter" : "presentation"}
  aria-label={ariaLabel}
  aria-valuenow={ariaLabel ? labelValue : undefined}
  aria-valuemin={ariaLabel ? 0 : undefined}
  aria-valuemax={ariaLabel ? 100 : undefined}
></canvas>

<style>
  /* The meter fills its container rather than claiming a fixed size, so the
     console's alignment comes from one place. */
  .meter {
    display: block;
    flex-shrink: 0;
  }
  .meter[data-orientation="horizontal"] {
    width: 100%;
    height: 20px;
  }
  .meter[data-orientation="vertical"] {
    width: 10px;
    height: 100%;
  }

  @media (forced-colors: active) {
    .meter {
      forced-color-adjust: none;
      outline: 1px solid CanvasText;
    }
  }
</style>
