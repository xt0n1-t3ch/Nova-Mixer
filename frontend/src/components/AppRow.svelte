<script lang="ts">
  /**
   * One application's channel: a horizontal row.
   *
   * Left to right: identity and live status, the fader with its level meter
   * fused beneath it on the same scale, the readout, mute, and row actions.
   *
   * A vertical strip was tried and dropped. It spent the full window height on
   * every source, left each fader's throw far longer than a volume needs, and
   * scrolled sideways past about seven sources. A row costs 48px, so a normal
   * window shows every source at once, and a long fader is the easiest control
   * to set precisely with a mouse.
   *
   * The meter is live Windows data: the backend polls each session's
   * `IAudioMeterInformation::GetPeakValue` every 50ms while this view is open,
   * and the row shows the loudest of the application's sessions.
   *
   * The row represents an *application*, not a Windows audio session. Windows
   * hands one application several sessions whenever it likes, which is why an
   * earlier version showed Discord twice. Sessions live behind the disclosure.
   */
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import Lock from "@lucide/svelte/icons/lock";
  import Pin from "@lucide/svelte/icons/pin";
  import Layers from "@lucide/svelte/icons/layers";
  import Settings2 from "@lucide/svelte/icons/settings-2";
  import type { Application } from "../lib/api";
  import { formatPercent, volumeValueText } from "../lib/volume";
  import AppIcon from "./AppIcon.svelte";
  import Meter from "./Meter.svelte";
  import Range from "./Range.svelte";
  import SessionRow from "./SessionRow.svelte";
  import { t } from "../lib/i18n/index";

  let {
    app,
    peak = 0,
    sessionPeaks = {},
    volume,
    groupName,
    compact = false,
    expanded = false,
    inspected = false,
    onInput,
    onCommit,
    onToggleMute,
    onToggleExpanded,
    onInspect,
    onTogglePin,
    onSessionInput,
    onSessionCommit,
    onSessionMute,
  }: {
    app: Application;
    peak?: number;
    sessionPeaks?: Record<string, number>;
    /** Overrides `app.volume` while a drag is in flight. */
    volume: number;
    groupName?: string | null;
    compact?: boolean;
    expanded?: boolean;
    inspected?: boolean;
    onInput: (value: number) => void;
    onCommit: (value: number) => void;
    onToggleMute: () => void;
    onToggleExpanded: () => void;
    onInspect: () => void;
    onTogglePin: () => void;
    onSessionInput: (liveId: string, value: number) => void;
    onSessionCommit: (liveId: string, value: number) => void;
    onSessionMute: (liveId: string, muted: boolean) => void;
  } = $props();

  let label = $derived(app.is_system_sounds ? $t("app.systemSounds") : app.display_name);
  // Matches `--row-icon`, which the head height is derived from.
  let iconSize = $derived(compact ? 22 : 28);
  let sessionCount = $derived(app.sessions.length);
  let locked = $derived(!app.controllable && app.running);

  let muteLabel = $derived(
    app.muted ? $t("app.unmute", { app: label }) : $t("app.mute", { app: label }),
  );

  /**
   * An application can own sessions and still be silent: Windows keeps a
   * session `inactive` between sounds. Saying so is the honest state; calling
   * it "playing" was not.
   */
  let idle = $derived(
    app.running && app.sessions.length > 0 && app.sessions.every((s) => s.state !== "active"),
  );

  /** The strip carries one secondary fact, chosen by what the user needs most. */
  let subtitle = $derived.by(() => {
    if (locked) return { kind: "locked" as const };
    if (!app.running) return { kind: "offline" as const };
    if (sessionCount > 1) return { kind: "sessions" as const, count: sessionCount };
    if (idle) return { kind: "idle" as const };
    if (groupName) return { kind: "group" as const, name: groupName };
    if (app.executable_name) return { kind: "exe" as const, text: app.executable_name };
    return { kind: "none" as const };
  });
</script>

<div
  class="channel"
  class:is-compact={compact}
  class:is-offline={!app.running}
  class:is-muted={app.muted}
  class:is-locked={locked}
  class:is-inspected={inspected}
  class:is-expanded={expanded}
>
  <!-- Identity: who this is and what Windows says about it right now. -->
  <div class="channel-id">
    <AppIcon
      src={app.icon}
      name={label}
      size={iconSize}
      isSystem={app.is_system_sounds}
      dimmed={!app.running}
    />
    <span class="channel-text">
      <span class="channel-name truncate" title={label}>{label}</span>
      {#if !compact}
        <span class="channel-sub">
          {#if subtitle.kind === "locked"}
            <span class="sub-flag is-warning"><Lock size={10} />{$t("app.lockedShort")}</span>
          {:else if subtitle.kind === "offline"}
            <span class="sub-flag">{$t("app.offline")}</span>
          {:else if subtitle.kind === "idle"}
            <span class="sub-flag">{$t("app.idle")}</span>
          {:else if subtitle.kind === "sessions"}
            <!-- Doubles as the disclosure: the fact and the control that
                 reveals what it counts are the same target. -->
            <button
              class="sub-disclosure"
              class:is-open={expanded}
              onclick={onToggleExpanded}
              aria-expanded={expanded}
              aria-label={$t("app.toggleSessions", { app: label })}
            >
              <Layers size={10} />
              {$t("app.sessionCount", { count: subtitle.count })}
            </button>
          {:else if subtitle.kind === "group"}
            <span class="sub-flag">{$t("app.inGroup", { group: subtitle.name })}</span>
          {:else if subtitle.kind === "exe"}
            <span class="sub-flag truncate">{subtitle.text}</span>
          {/if}
        </span>
      {:else if sessionCount > 1}
        <button
          class="sub-disclosure"
          class:is-open={expanded}
          onclick={onToggleExpanded}
          aria-expanded={expanded}
          aria-label={$t("app.toggleSessions", { app: label })}
        >
          <Layers size={10} />
          {sessionCount}
        </button>
      {/if}
    </span>
  </div>

  <!-- The instrument: the fader, with the live meter as a thin rail directly
       beneath it on the same scale. "How loud it is" sits under "how loud I set
       it", so the two are read in one glance without a second column. -->
  <div class="channel-level">
    <Range
      value={volume}
      disabled={locked}
      muted={app.muted}
      indeterminate={app.mixed}
      resetTo={1}
      ariaLabel={$t("app.volumeFor", { app: label })}
      ariaValueText={app.mixed
        ? $t("app.mixedLevel")
        : volumeValueText(volume, app.muted, $t("app.muted"))}
      {onInput}
      {onCommit}
    />
    <div class="channel-meter">
      <Meter peak={app.muted || !app.running ? 0 : peak} bars={compact ? 24 : 32} />
    </div>
  </div>

  <output class="channel-value mono" for="">
    {app.mixed ? $t("app.mixedShort") : formatPercent(volume)}
  </output>

  <button
    class="channel-mute"
    class:is-danger={app.muted}
    disabled={locked}
    onclick={onToggleMute}
    aria-label={muteLabel}
    aria-pressed={app.muted}
    title={locked ? $t("app.lockedHint") : muteLabel}
  >
    {#if app.muted}
      <VolumeX size={15} />
    {:else if volume < 0.5}
      <Volume1 size={15} />
    {:else}
      <Volume2 size={15} />
    {/if}
  </button>

  <div class="channel-actions">
    <button
      class="icon-btn icon-btn-sm pin-action"
      class:is-on={app.pinned}
      onclick={onTogglePin}
      aria-pressed={app.pinned}
      aria-label={app.pinned ? $t("app.unpin", { app: label }) : $t("app.pin", { app: label })}
      title={app.pinned ? $t("app.unpin", { app: label }) : $t("app.pin", { app: label })}
    >
      <Pin size={13} />
    </button>
    <button
      class="icon-btn icon-btn-sm"
      class:is-on={inspected}
      onclick={onInspect}
      aria-label={$t("app.configure", { app: label })}
      title={$t("app.configure", { app: label })}
    >
      <Settings2 size={13} />
    </button>
  </div>
</div>

{#if expanded && sessionCount > 0}
  <!-- The streams of one application, indented under its row. A diagnostic
       view for "which of Discord's streams is the loud one", not part of
       everyday mixing. -->
  <ul class="session-list" aria-label={$t("app.sessionsFor", { app: label })}>
    {#each app.sessions as session (session.live_id)}
      <li>
        <SessionRow
          {session}
          peak={sessionPeaks[session.live_id] ?? 0}
          onInput={(value) => onSessionInput(session.live_id, value)}
          onCommit={(value) => onSessionCommit(session.live_id, value)}
          onToggleMute={() => onSessionMute(session.live_id, !session.muted)}
        />
      </li>
    {/each}
  </ul>
{/if}

<style>
  /* One channel, one row, on the shared channel grid so every fader, value and
     mute button in the list sits in the same column. */
  .channel {
    display: grid;
    grid-template-columns: var(--grid-channel);
    align-items: center;
    gap: var(--space-4);
    min-height: var(--row-height);
    padding: 4px var(--space-3) 4px var(--space-4);
    border-radius: var(--radius-md);
    transition:
      background var(--dur-fast) var(--ease),
      opacity var(--dur-normal) var(--ease);
  }
  .channel:hover,
  .channel:focus-within {
    background: var(--channel-hover);
  }
  .channel.is-inspected {
    background: var(--channel-selected);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  /* No audio right now: the row stays so its level can be set in advance, but
     it recedes behind everything that is making sound. */
  .channel.is-offline {
    opacity: 0.55;
  }
  .channel.is-offline:hover,
  .channel.is-offline:focus-within {
    opacity: 1;
  }

  .channel-id {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }
  .channel-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .channel-name {
    font-size: var(--fs-base);
    font-weight: 550;
    color: var(--text-primary);
    line-height: var(--lh-tight);
  }
  .is-muted .channel-name {
    color: var(--text-muted);
  }
  .channel-sub {
    display: flex;
    min-width: 0;
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }
  .sub-flag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
  }
  .sub-flag.is-warning {
    color: var(--warning);
  }
  .sub-disclosure {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    padding: 1px 7px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-size: var(--fs-2xs);
    white-space: nowrap;
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .sub-disclosure:hover,
  .sub-disclosure.is-open {
    color: var(--text-primary);
    background: var(--bg-elevated-2);
  }

  /* Fader above, meter rail below, one width: the meter reads on the same
     0–100 scale as the fader it sits under. */
  .channel-level {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    min-width: 0;
  }
  .channel-meter {
    height: 5px;
    padding: 0 2px;
  }
  .channel-meter :global(.meter) {
    height: 100%;
  }

  .channel-value {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-primary);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .is-muted .channel-value,
  .is-offline .channel-value {
    color: var(--text-faint);
  }

  .channel-mute {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 30px;
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .channel-mute:hover:not(:disabled) {
    background: var(--bg-elevated-2);
    color: var(--text-primary);
  }
  .channel-mute:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  /* `--danger-on`, not `--accent-fg`: the glyph sits on a solid red fill and
     the readable ink there differs per theme. */
  .channel-mute.is-danger {
    color: var(--danger-on);
    background: var(--danger);
    border-color: transparent;
  }

  /* Quiet until the row is engaged, so a full list reads as levels rather than
     a field of buttons. Pinned and inspected are state, so they stay visible. */
  .channel-actions {
    display: flex;
    gap: 2px;
  }
  .channel-actions :global(.icon-btn) {
    color: var(--text-faint);
    opacity: 0;
    transition:
      opacity var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .channel:hover .channel-actions :global(.icon-btn),
  .channel:focus-within .channel-actions :global(.icon-btn),
  .channel-actions :global(.is-on) {
    opacity: 1;
    color: var(--text-secondary);
  }

  .channel.is-compact {
    gap: var(--space-3);
    padding-top: 4px;
    padding-bottom: 4px;
  }
  .channel.is-compact .channel-name {
    font-size: var(--fs-sm);
  }

  .session-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    list-style: none;
    margin: 0 0 var(--space-2) calc(var(--space-4) + var(--row-icon) + var(--space-3));
    padding: var(--space-1) 0 var(--space-1) var(--space-3);
    border-left: 1px solid var(--deck-line);
  }

  @media (prefers-reduced-motion: reduce) {
    .channel,
    .channel-actions :global(.icon-btn) {
      transition: none;
    }
  }
  @media (hover: none) {
    .channel-actions :global(.icon-btn) {
      opacity: 1;
    }
  }
</style>