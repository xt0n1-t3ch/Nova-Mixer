<script lang="ts">
  /**
   * Records a global shortcut by listening to a real key press.
   *
   * Building an accelerator from a dropdown of key names is how the previous
   * version did it, and it produced combinations users could not actually type.
   * Capturing the event guarantees the binding is reachable on this keyboard.
   */
  import { onDestroy } from "svelte";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import X from "@lucide/svelte/icons/x";
  import { formatAccelerator } from "../lib/ux";
  import { t } from "../lib/i18n/index";

  let {
    accelerator,
    label,
    onChange,
  }: {
    accelerator: string | null;
    label: string;
    onChange: (accelerator: string | null) => void;
  } = $props();

  let recording = $state(false);

  /** Keys that cannot stand alone as a shortcut. */
  const MODIFIERS = new Set(["Control", "Shift", "Alt", "Meta"]);

  /** DOM `event.key` values that Tauri names differently. */
  const KEY_ALIASES: Record<string, string> = {
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    " ": "Space",
    Escape: "Escape",
    AudioVolumeUp: "MediaVolumeUp",
    AudioVolumeDown: "MediaVolumeDown",
    AudioVolumeMute: "MediaVolumeMute",
  };

  function toAccelerator(event: KeyboardEvent): string | null {
    if (MODIFIERS.has(event.key)) return null;

    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Control");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    if (event.metaKey) parts.push("Super");

    const raw = KEY_ALIASES[event.key] ?? event.key;
    const key = raw.length === 1 ? raw.toUpperCase() : raw;

    // A bare letter would swallow that key system-wide, which is a trap. Media
    // keys are the exception: they carry no other meaning.
    if (parts.length === 0 && !key.startsWith("Media") && !key.startsWith("F")) return null;

    parts.push(key);
    return parts.join("+");
  }

  function onKeydown(event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      recording = false;
      return;
    }
    const next = toAccelerator(event);
    if (next) {
      onChange(next);
      recording = false;
    }
  }

  $effect(() => {
    if (!recording) return;
    window.addEventListener("keydown", onKeydown, true);
    return () => window.removeEventListener("keydown", onKeydown, true);
  });

  onDestroy(() => {
    recording = false;
  });

  let display = $derived(formatAccelerator(accelerator));
</script>

<div class="hotkey">
  <button
    class="hotkey-slot"
    class:is-recording={recording}
    class:is-empty={!accelerator}
    onclick={() => (recording = !recording)}
    aria-label="{label}: {display ?? $t('settings.hotkeyUnset')}"
    aria-pressed={recording}
  >
    {#if recording}
      <Keyboard size={13} />
      <span class="hotkey-hint">{$t("settings.hotkeyRecord")}</span>
    {:else if display}
      <span class="kbd hotkey-kbd">{display}</span>
    {:else}
      <span class="hotkey-hint">{$t("settings.hotkeyUnset")}</span>
    {/if}
  </button>

  {#if accelerator && !recording}
    <button
      class="icon-btn icon-btn-sm"
      onclick={() => onChange(null)}
      aria-label="{$t('settings.hotkeyClear')}: {label}"
      title={$t("settings.hotkeyClear")}
    >
      <X size={12} />
    </button>
  {/if}
</div>

<style>
  .hotkey {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .hotkey-slot {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 150px;
    height: 32px;
    padding: 0 12px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    transition:
      border-color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .hotkey-slot:hover {
    border-color: var(--border-hover);
    color: var(--text-primary);
  }
  .hotkey-slot:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .hotkey-slot.is-recording {
    border-color: var(--accent);
    background: var(--accent-dim);
    color: var(--accent);
    /* A steady pulse says "listening" without stealing attention. */
    animation: hotkey-pulse 1.4s var(--ease) infinite;
  }
  .hotkey-slot.is-empty .hotkey-hint {
    color: var(--text-placeholder);
  }

  .hotkey-kbd {
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: var(--fs-xs);
    height: auto;
    padding: 0;
  }

  @keyframes hotkey-pulse {
    0%,
    100% {
      box-shadow: 0 0 0 0 var(--accent-dim);
    }
    50% {
      box-shadow: 0 0 0 4px var(--accent-dim);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .hotkey-slot.is-recording {
      animation: none;
    }
  }
</style>
