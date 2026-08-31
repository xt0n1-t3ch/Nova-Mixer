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
   * Dragging fires `onInput` on every frame for optical feedback; `onCommit`
   * fires once when the pointer or key is released. Callers send IPC from
   * `onCommit` and repaint from `onInput`, which keeps a drag from flooding the
   * audio worker with hundreds of calls.
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
  data-size={size}
  data-tone={tone}
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

  .range-fill {
    height: 100%;
    width: var(--range-percent);
    border-radius: var(--radius-full);
    background: var(--range-color, var(--accent));
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
    height: 20px;
  }
  .range[data-size="sm"] .range-track {
    height: 3px;
  }
  .range[data-size="lg"] .range-track {
    height: 6px;
  }

  .range:hover:not(.is-disabled) .range-track,
  .range.is-dragging .range-track {
    height: var(--slider-height-hover);
    background: var(--slider-track-hover);
  }
  .range[data-size="lg"]:hover:not(.is-disabled) .range-track,
  .range[data-size="lg"].is-dragging .range-track {
    height: 8px;
  }

  .range.is-muted .range-fill {
    background: var(--slider-fill-muted);
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

  input[type="range"]::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: var(--slider-thumb-size);
    height: var(--slider-thumb-size);
    border-radius: 50%;
    background: var(--slider-thumb);
    border: none;
    box-shadow:
      0 1px 3px var(--slider-thumb-ring),
      0 0 0 1px var(--slider-thumb-ring);
    transition:
      width var(--dur-fast) var(--spring),
      height var(--dur-fast) var(--spring),
      opacity var(--dur-fast) var(--ease);
    /* Thumb stays hidden until the row is engaged so a dense list reads as
       bars, not as a wall of dots. It appears on hover, focus, and drag. */
    opacity: 0;
  }

  input[type="range"]::-moz-range-thumb {
    width: var(--slider-thumb-size);
    height: var(--slider-thumb-size);
    border-radius: 50%;
    background: var(--slider-thumb);
    border: none;
    box-shadow:
      0 1px 3px var(--slider-thumb-ring),
      0 0 0 1px var(--slider-thumb-ring);
    opacity: 0;
  }
  input[type="range"]::-moz-range-track {
    height: 100%;
    background: transparent;
    border: none;
  }

  .range:hover input[type="range"]::-webkit-slider-thumb,
  .range.is-dragging input[type="range"]::-webkit-slider-thumb,
  input[type="range"]:focus-visible::-webkit-slider-thumb {
    opacity: 1;
  }
  .range:hover input[type="range"]::-moz-range-thumb,
  .range.is-dragging input[type="range"]::-moz-range-thumb,
  input[type="range"]:focus-visible::-moz-range-thumb {
    opacity: 1;
  }

  .range.is-dragging input[type="range"]::-webkit-slider-thumb {
    width: var(--slider-thumb-size-hover);
    height: var(--slider-thumb-size-hover);
  }

  .range.is-disabled input[type="range"]::-webkit-slider-thumb {
    opacity: 0;
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
