<script lang="ts">
  /**
   * The app's only slider primitive.
   *
   * A native `input[type=range]` does the work: pointer capture, keyboard
   * stepping, Home/End, page steps, and the `slider` accessibility role all come
   * from the platform. Re-implementing those on a div is where custom sliders
   * usually lose keyboard and screen-reader support, so the native element is
   * kept and only painted over.
   *
   * Dragging fires `onInput` on every frame; `onCommit` fires once when the
   * pointer or key is released. Callers repaint and send a coalesced live level
   * from `onInput` (see `liveSender` in `lib/stores`), so the sound follows the
   * drag with one IPC call in flight, and `onCommit` guarantees the final value.
   */
  import { clampScalar } from "../lib/volume";

  let {
    value = $bindable(0),
    onInput,
    onCommit,
    disabled = false,
    muted = false,
    ariaLabel,
    ariaValueText,
    step = 0.01,
    fineStep = 0.002,
    resetTo,
    size = "md",
    tone = "accent",
    indeterminate = false,
    orientation = "horizontal",
  }: {
    /** Linear scalar, 0.0–1.0. */
    value?: number;
    onInput?: (value: number) => void;
    onCommit?: (value: number) => void;
    disabled?: boolean;
    /** Renders the fill greyed out without disabling the control. */
    muted?: boolean;
    ariaLabel: string;
    ariaValueText?: string;
    step?: number;
    /** Step used while Shift is held, for precise placement. */
    fineStep?: number;
    /** Double-click target. Omit to disable the shortcut. */
    resetTo?: number;
    size?: "sm" | "md" | "lg";
    tone?: "accent" | "success" | "warning" | "danger";
    /**
     * The underlying values disagree and no single level is true yet. The track
     * shows a hatch rather than a fill, because inventing an average would show
     * a number that is not any session's actual volume. The first move resolves
     * it by synchronizing everything.
     */
    indeterminate?: boolean;
    /**
     * A vertical fader is the channel-strip form. It uses the native
     * `writing-mode: vertical-rl` slider rather than a rotated horizontal one,
     * so the browser keeps pointer mapping, keyboard stepping and the slider
     * role intact. A CSS `rotate()` would invert the drag direction and detach
     * the hit area from the painted control.
     */
    orientation?: "horizontal" | "vertical";
  } = $props();

  let dragging = $state(false);
  let shiftHeld = $state(false);

  let percent = $derived(clampScalar(value) * 100);
  let activeStep = $derived(shiftHeld ? fineStep : step);

  /** Keys the native range control responds to; each ends in a commit. */
  const STEP_KEYS = [
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "ArrowDown",
    "Home",
    "End",
    "PageUp",
    "PageDown",
  ];

  function handleInput(event: Event): void {
    if (disabled) return;
    const next = clampScalar(Number((event.currentTarget as HTMLInputElement).value));
    value = next;
    onInput?.(next);
  }

  function commit(): void {
    if (disabled) return;
    onCommit?.(clampScalar(value));
  }

  // A disabled control should never emit anything. Browsers suppress pointer
  // events on a disabled input, but not every engine does, so the guard is here
  // rather than assumed.
  function onPointerDown(): void {
    if (disabled) return;
    dragging = true;
  }

  function onPointerUp(): void {
    if (!dragging) return;
    dragging = false;
    commit();
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === "Shift") shiftHeld = true;
  }

  function onKeyUp(event: KeyboardEvent): void {
    if (event.key === "Shift") shiftHeld = false;
    if (STEP_KEYS.includes(event.key)) commit();
  }

  function onDoubleClick(): void {
    if (disabled || resetTo === undefined) return;
    const next = clampScalar(resetTo);
    value = next;
    onInput?.(next);
    onCommit?.(next);
  }
</script>

<div
  class="range"
  class:is-dragging={dragging}
  class:is-muted={muted}
  class:is-disabled={disabled}
  class:is-indeterminate={indeterminate}
  data-size={size}
  data-tone={tone}
  data-orientation={orientation}
  style:--range-percent="{percent}%"
>
  <div class="range-track" aria-hidden="true">
    <div class="range-fill"></div>
  </div>
  <input
    type="range"
    min="0"
    max="1"
    step={activeStep}
    {value}
    {disabled}
    aria-label={ariaLabel}
    aria-valuetext={ariaValueText}
    oninput={handleInput}
    onpointerdown={onPointerDown}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
    onlostpointercapture={onPointerUp}
    onkeydown={onKeyDown}
    onkeyup={onKeyUp}
    onblur={() => (shiftHeld = false)}
    ondblclick={onDoubleClick}
  />
</div>

<style>
  /* The wrapper owns the visible track so the native input can stay fully
     transparent on top of it — that keeps every platform interaction while the
     paint is entirely ours. */
  .range {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 60px;
    height: var(--slider-hit-area);
    touch-action: none;
  }

  /* The empty portion of the track stays visible at every level, so a row at
     100% is still recognisably a slider rather than a filled bar.

     Fully rounded, unlike the meter's square segments beside it: the round
     capsule is the shape a user reads as "I can drag this", and the contrast
     between the two shapes is what separates the control from the signal. */
  .range-track {
    position: absolute;
    left: 0;
    right: 0;
    height: var(--slider-height);
    border-radius: var(--radius-full);
    background: var(--slider-track);
    overflow: hidden;
    transition:
      height var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }

  /* Flat ink, no gradient and no glow. The fill states a value; a gradient
     would imply the level varies along the bar, which it does not. */
  .range-fill {
    height: 100%;
    width: var(--range-percent);
    border-radius: var(--radius-full);
    background: var(--range-color, var(--slider-fill));
  }

  /* No width transition while dragging: the pointer is already the animation,
     and a lagging fill feels broken. It returns for keyboard and reset. */
  .range:not(.is-dragging) .range-fill {
    transition: width var(--dur-fast) var(--ease-out);
  }

  .range[data-tone="success"] {
    --range-color: var(--success);
  }
  .range[data-tone="warning"] {
    --range-color: var(--warning);
  }
  .range[data-tone="danger"] {
    --range-color: var(--danger);
  }

  .range[data-size="sm"] {
    height: 22px;
  }
  .range[data-size="sm"] .range-track {
    height: 4px;
  }
  .range[data-size="lg"] .range-track {
    height: 8px;
  }

  .range:hover:not(.is-disabled) .range-track,
  .range.is-dragging .range-track {
    height: var(--slider-height-hover);
    background: var(--slider-track-hover);
  }
  .range[data-size="lg"]:hover:not(.is-disabled) .range-track,
  .range[data-size="lg"].is-dragging .range-track {
    height: 10px;
  }

  /* ── Vertical fader ──────────────────────────────────────────────────────
     The channel-strip form. The travel is a recessed slot cut into the strip,
     the fill rises from the bottom, and the thumb is a wide cap that reads as a
     physical grip. Everything below re-maps the horizontal geometry onto the
     block axis; the input itself is switched with `writing-mode`, so pointer
     direction, keyboard stepping and the slider role stay native. */
  .range[data-orientation="vertical"] {
    width: var(--slider-hit-area);
    min-width: 0;
    height: 100%;
    justify-content: center;
  }
  .range[data-orientation="vertical"] .range-track {
    left: 50%;
    right: auto;
    top: 0;
    bottom: 0;
    transform: translateX(-50%);
    width: var(--slider-height);
    height: auto;
    border-radius: var(--radius-full);
    box-shadow: var(--fader-slot-shadow);
    transition:
      width var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .range[data-orientation="vertical"] .range-fill {
    position: absolute;
    inset: auto 0 0 0;
    width: 100%;
    height: var(--range-percent);
  }
  .range[data-orientation="vertical"]:not(.is-dragging) .range-fill {
    transition: height var(--dur-fast) var(--ease-out);
  }
  .range[data-orientation="vertical"]:hover:not(.is-disabled) .range-track,
  .range[data-orientation="vertical"].is-dragging .range-track {
    height: auto;
    width: var(--slider-height-hover);
  }
  .range[data-orientation="vertical"] input[type="range"] {
    writing-mode: vertical-rl;
    direction: rtl;
    width: 100%;
    height: 100%;
  }
  /* A wide, short cap rather than a circle: on a console the fader knob is a
     grip you push, and its width is what makes the travel readable at a
     glance. */
  .range[data-orientation="vertical"] input[type="range"]::-webkit-slider-thumb {
    width: var(--fader-cap-width);
    height: var(--fader-cap-height);
    border-radius: var(--radius-xs);
    background: var(--fader-cap);
    box-shadow: var(--fader-cap-shadow);
  }
  .range[data-orientation="vertical"] input[type="range"]::-moz-range-thumb {
    width: var(--fader-cap-width);
    height: var(--fader-cap-height);
    border-radius: var(--radius-xs);
    background: var(--fader-cap);
    box-shadow: var(--fader-cap-shadow);
  }
  .range[data-orientation="vertical"]:hover input[type="range"]::-webkit-slider-thumb,
  .range[data-orientation="vertical"] input[type="range"]:focus-visible::-webkit-slider-thumb,
  .range[data-orientation="vertical"].is-dragging input[type="range"]::-webkit-slider-thumb {
    width: var(--fader-cap-width);
    height: var(--fader-cap-height);
    background: var(--fader-cap-hover);
    box-shadow: var(--fader-cap-shadow-hover);
  }
  .range[data-orientation="vertical"]:hover input[type="range"]::-moz-range-thumb,
  .range[data-orientation="vertical"] input[type="range"]:focus-visible::-moz-range-thumb {
    background: var(--fader-cap-hover);
    box-shadow: var(--fader-cap-shadow-hover);
  }

  .range.is-muted .range-fill {
    background: var(--slider-fill-muted);
    box-shadow: none;
  }

  /* Sessions disagree: a hatched full-width track says "no single value" far
     more honestly than a fill drawn at a made-up average. */
  .range.is-indeterminate .range-fill {
    width: 100%;
    background: repeating-linear-gradient(
      -45deg,
      var(--slider-fill-muted) 0 4px,
      transparent 4px 8px
    );
  }

  .range.is-disabled {
    opacity: 0.4;
  }
  .range.is-disabled .range-track {
    cursor: not-allowed;
  }

  input[type="range"] {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    background: transparent;
    border: none;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
  }
  input[type="range"]:disabled {
    cursor: not-allowed;
  }

  input[type="range"]::-webkit-slider-runnable-track {
    height: 100%;
    background: transparent;
    border: none;
  }

  /* The thumb is always present: it is the only cue that separates a slider
     from the level meter beside it. It sits small and quiet at rest, then grows
     on hover, focus and drag. The ring is a dark hairline rather than a glow,
     so the thumb stays legible where it overlaps its own fill. */
  input[type="range"]::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: var(--slider-thumb-size);
    height: var(--slider-thumb-size);
    border-radius: 50%;
    background: var(--slider-thumb);
    border: none;
    box-shadow:
      0 0 0 1px var(--slider-thumb-ring),
      var(--shadow-xs);
    transition:
      width var(--dur-fast) var(--spring),
      height var(--dur-fast) var(--spring),
      box-shadow var(--dur-fast) var(--ease);
  }

  input[type="range"]::-moz-range-thumb {
    width: var(--slider-thumb-size);
    height: var(--slider-thumb-size);
    border-radius: 50%;
    background: var(--slider-thumb);
    border: none;
    box-shadow:
      0 0 0 1px var(--slider-thumb-ring),
      var(--shadow-xs);
  }
  input[type="range"]::-moz-range-track {
    height: 100%;
    background: transparent;
    border: none;
  }

  .range:hover input[type="range"]::-webkit-slider-thumb,
  input[type="range"]:focus-visible::-webkit-slider-thumb {
    width: var(--slider-thumb-size-hover);
    height: var(--slider-thumb-size-hover);
    box-shadow:
      0 0 0 1px var(--slider-thumb-ring),
      var(--shadow-sm);
  }
  .range:hover input[type="range"]::-moz-range-thumb,
  input[type="range"]:focus-visible::-moz-range-thumb {
    width: var(--slider-thumb-size-hover);
    height: var(--slider-thumb-size-hover);
    box-shadow:
      0 0 0 1px var(--slider-thumb-ring),
      var(--shadow-sm);
  }

  /* Dragging adds a soft halo in the accent so the grabbed thumb is findable
     under the pointer without the fill itself changing colour. */
  .range.is-dragging input[type="range"]::-webkit-slider-thumb {
    width: var(--slider-thumb-size-hover);
    height: var(--slider-thumb-size-hover);
    box-shadow:
      0 0 0 1px var(--slider-thumb-ring),
      0 0 0 6px var(--accent-dim),
      var(--shadow-sm);
  }

  /* A locked session keeps its thumb so the row still reads as a volume
     control, but the whole track is dimmed by `.is-disabled`. */
  .range.is-muted input[type="range"]::-webkit-slider-thumb {
    background: var(--slider-fill-muted);
  }
  .range.is-muted input[type="range"]::-moz-range-thumb {
    background: var(--slider-fill-muted);
  }

  /* The focus ring goes on the track, not the 24px hit area, so it traces the
     control the user actually sees. */
  input[type="range"]:focus-visible {
    outline: none;
  }
  input[type="range"]:focus-visible ~ :global(*),
  .range:has(input[type="range"]:focus-visible) .range-track {
    box-shadow: var(--shadow-ring);
  }

  @media (forced-colors: active) {
    .range-track {
      border: 1px solid CanvasText;
    }
    .range-fill {
      background: Highlight;
      forced-color-adjust: none;
    }
  }
</style>
