<script lang="ts">
  /**
   * One application's channel strip.
   *
   * Layout is a fixed grid rather than flex so every row's slider starts and
   * ends at the same x position down the whole list. Ragged sliders make a
   * mixer unreadable, and flex would produce exactly that as names vary.
   */
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import Lock from "@lucide/svelte/icons/lock";
  import type { AudioSession } from "../lib/api";
  import { formatPercent, volumeValueText } from "../lib/volume";
  import AppIcon from "./AppIcon.svelte";
  import Meter from "./Meter.svelte";
  import Range from "./Range.svelte";
  import { t } from "../lib/i18n/index";

  let {
    session,
    peak = 0,
    volume,
    groupName,
    compact = false,
    onInput,
    onCommit,
    onToggleMute,
  }: {
    session: AudioSession;
    peak?: number;
    /** Overrides `session.volume` while a drag is in flight. */
    volume: number;
    groupName?: string | null;
    compact?: boolean;
    onInput: (value: number) => void;
    onCommit: (value: number) => void;
    onToggleMute: () => void;
  } = $props();

  let label = $derived(
    session.is_system_sounds ? $t("mixer.systemSounds") : session.display_name,
  );
  let idle = $derived(session.state === "inactive");
  let iconSize = $derived(compact ? 28 : 38);

  let muteLabel = $derived(
    session.muted ? $t("mixer.unmute", { app: label }) : $t("mixer.mute", { app: label }),
  );
</script>

<div
  class="app-row"
  class:is-compact={compact}
  class:is-idle={idle}
  class:is-muted={session.muted}
  class:is-locked={!session.controllable}
>
  <div class="row-identity">
    <AppIcon
      src={session.icon}
      name={label}
      appKey={session.app_key}
      size={iconSize}
      isSystem={session.is_system_sounds}
    />
    <div class="row-text">
      <span class="row-name truncate" title={label}>{label}</span>
      {#if !compact}
        <span class="row-sub truncate">
          {#if !session.controllable}
            <span class="row-flag">
              <Lock size={10} />
              {$t("mixer.notControllableShort")}
            </span>
          {:else if idle}
            <span class="row-flag">{$t("mixer.inactive")}</span>
          {:else if groupName}
            <span class="row-flag">{$t("mixer.inGroup", { group: groupName })}</span>
          {:else if session.executable_name}
            {session.executable_name}
          {/if}
        </span>
      {/if}
    </div>
  </div>

  <div class="row-meter">
    <Meter peak={session.muted ? 0 : peak} bars={compact ? 4 : 5} />
  </div>

  <div class="row-fader">
    <Range
      value={volume}
      disabled={!session.controllable}
      muted={session.muted}
      size={compact ? "sm" : "md"}
      resetTo={1}
      ariaLabel={$t("mixer.sessionVolume", { app: label })}
      ariaValueText={volumeValueText(volume, session.muted, $t("mixer.muted"))}
      {onInput}
      {onCommit}
    />
  </div>

  <output class="row-value mono" for="">{formatPercent(volume)}</output>

  <button
    class="icon-btn"
    class:is-danger={session.muted}
    disabled={!session.controllable}
    onclick={onToggleMute}
    aria-label={muteLabel}
    aria-pressed={session.muted}
    title={!session.controllable ? $t("mixer.notControllable") : muteLabel}
  >
    {#if session.muted}
      <VolumeX size={16} />
    {:else if volume < 0.5}
      <Volume1 size={16} />
    {:else}
      <Volume2 size={16} />
    {/if}
  </button>
</div>

<style>
  /* Columns: identity | meter | fader | readout | mute.
     Only the fader flexes, so every readout and mute button lines up. */
  .app-row {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) auto minmax(120px, 2fr) 46px 32px;
    align-items: center;
    gap: var(--space-3);
    height: var(--row-height);
    padding: 0 var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: 1px solid var(--border);
    transition:
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease),
      opacity var(--dur-normal) var(--ease);
  }
  .app-row:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-hover);
  }
  .app-row:focus-within {
    border-color: var(--border-hover);
  }

  .app-row.is-compact {
    grid-template-columns: minmax(0, 1fr) auto minmax(110px, 2fr) 42px 32px;
    gap: var(--space-2);
    padding: 0 var(--space-3);
  }

  /* Idle rows stay listed and controllable, matching the Windows mixer, but
     recede so the audible applications read first. */
  .app-row.is-idle {
    opacity: 0.62;
  }
  .app-row.is-idle:hover,
  .app-row.is-idle:focus-within {
    opacity: 1;
  }

  .row-identity {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }
  .is-compact .row-identity {
    gap: var(--space-2);
  }

  .row-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: var(--lh-tight);
  }
  .row-name {
    font-size: var(--fs-base);
    font-weight: 500;
    color: var(--text-primary);
  }
  .is-muted .row-name {
    color: var(--text-muted);
  }
  .row-sub {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    margin-top: 3px;
  }
  .row-flag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .is-locked .row-flag {
    color: var(--warning);
  }

  .row-meter {
    display: flex;
    align-items: center;
  }

  .row-fader {
    min-width: 0;
  }

  .row-value {
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .is-muted .row-value {
    color: var(--text-placeholder);
  }

  /* Narrow windows drop the meter before the fader: the number and the slider
     carry the information, the meter is a secondary cue. */
  @media (max-width: 640px) {
    .app-row,
    .app-row.is-compact {
      grid-template-columns: minmax(0, 1fr) minmax(90px, 1.6fr) 42px 32px;
    }
    .row-meter {
      display: none;
    }
  }
</style>
