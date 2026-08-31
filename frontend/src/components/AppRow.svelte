<script lang="ts">
  /**
   * One application's channel strip.
   *
   * This row represents an *application*, not a Windows audio session. Windows
   * hands one application several sessions whenever it likes, which is why the
   * previous version showed Discord twice. Sessions live behind the disclosure
   * caret, where they belong: useful for diagnosis, not for everyday mixing.
   *
   * Layout is a fixed grid rather than flex so every row's slider starts and
   * ends at the same x position down the whole list. Ragged sliders make a
   * mixer unreadable, and flex would produce exactly that as names vary.
   */
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import Lock from "@lucide/svelte/icons/lock";
  import Pin from "@lucide/svelte/icons/pin";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
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
  let iconSize = $derived(compact ? 28 : 38);
  let sessionCount = $derived(app.sessions.length);
  /** Only worth disclosing when there is more than one thing to see. */
  let canExpand = $derived(sessionCount > 1);

  let muteLabel = $derived(
    app.muted ? $t("app.unmute", { app: label }) : $t("app.mute", { app: label }),
  );

  /** The secondary line carries one fact, chosen by what the user needs most. */
  let subtitle = $derived.by(() => {
    if (!app.controllable && app.running) return { kind: "locked" as const };
    if (!app.running) return { kind: "offline" as const };
    if (sessionCount > 1) return { kind: "sessions" as const, count: sessionCount };
    if (groupName) return { kind: "group" as const, name: groupName };
    if (app.executable_name) return { kind: "exe" as const, text: app.executable_name };
    return { kind: "none" as const };
  });
</script>

<div class="app-block" class:is-expanded={expanded}>
  <div
    class="app-row"
    class:is-compact={compact}
    class:is-offline={!app.running}
    class:is-muted={app.muted}
    class:is-locked={!app.controllable && app.running}
    class:is-inspected={inspected}
  >
    <div class="row-lead">
      {#if canExpand}
        <button
          class="disclosure"
          class:is-open={expanded}
          onclick={onToggleExpanded}
          aria-expanded={expanded}
          aria-label={$t("app.toggleSessions", { app: label })}
        >
          <ChevronRight size={13} />
        </button>
      {:else}
        <span class="disclosure-spacer" aria-hidden="true"></span>
      {/if}

      <AppIcon
        src={app.icon}
        name={label}
        appKey={app.app_key}
        size={iconSize}
        isSystem={app.is_system_sounds}
        dimmed={!app.running}
      />

      <div class="row-text">
        <!-- No pin badge beside the name: the pin button on the right is always
             visible and carries the state, so a second marker only adds noise. -->
        <span class="row-name truncate" title={label}>{label}</span>
        {#if !compact}
          <span class="row-sub truncate">
            {#if subtitle.kind === "locked"}
              <span class="row-flag is-warning"><Lock size={10} />{$t("app.lockedShort")}</span>
            {:else if subtitle.kind === "offline"}
              <span class="row-flag">{$t("app.offline")}</span>
            {:else if subtitle.kind === "sessions"}
              <span class="row-flag">{$t("app.sessionCount", { count: subtitle.count })}</span>
            {:else if subtitle.kind === "group"}
              <span class="row-flag">{$t("app.inGroup", { group: subtitle.name })}</span>
            {:else if subtitle.kind === "exe"}
              {subtitle.text}
            {/if}
          </span>
        {/if}
      </div>
    </div>

    <div class="row-meter">
      <Meter peak={app.muted || !app.running ? 0 : peak} bars={compact ? 4 : 5} />
    </div>

    <div class="row-fader">
      <Range
        value={volume}
        disabled={!app.controllable && app.running}
        muted={app.muted}
        indeterminate={app.mixed}
        size={compact ? "sm" : "md"}
        resetTo={1}
        ariaLabel={$t("app.volumeFor", { app: label })}
        ariaValueText={app.mixed
          ? $t("app.mixedLevel")
          : volumeValueText(volume, app.muted, $t("app.muted"))}
        {onInput}
        {onCommit}
      />
    </div>

    <output class="row-value mono" for="">
      {app.mixed ? $t("app.mixedShort") : formatPercent(volume)}
    </output>

    <button
      class="icon-btn"
      class:is-danger={app.muted}
      disabled={!app.controllable && app.running}
      onclick={onToggleMute}
      aria-label={muteLabel}
      aria-pressed={app.muted}
      title={!app.controllable && app.running ? $t("app.lockedHint") : muteLabel}
    >
      {#if app.muted}
        <VolumeX size={16} />
      {:else if volume < 0.5}
        <Volume1 size={16} />
      {:else}
        <Volume2 size={16} />
      {/if}
    </button>

    <div class="row-actions">
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
</div>

<style>
  .app-block {
    border-bottom: 1px solid var(--deck-line);
  }
  .app-block:last-child {
    border-bottom: none;
  }

  /* A channel on a console, not a card in a list: rows sit on one continuous
     surface divided by rules, so the eye tracks a column of faders instead of
     re-reading a border around every application.

     Columns: lead | meter | fader | readout | mute | actions. Only the fader
     flexes, so every readout and button lines up down the whole desk. The lead
     has a 168px floor because below that an application name truncates to
     initials, which is what made the first version unreadable when narrow. */
  .app-row {
    display: grid;
    grid-template-columns: minmax(168px, 1.1fr) 40px minmax(140px, 2fr) 48px 32px 60px;
    align-items: center;
    gap: var(--space-3);
    height: var(--row-height);
    padding: 0 var(--space-4) 0 var(--space-2);
    border-left: 2px solid transparent;
    transition:
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease),
      opacity var(--dur-normal) var(--ease);
  }
  .app-row:hover {
    background: var(--channel-hover);
  }
  .app-row.is-inspected {
    background: var(--channel-selected);
    border-left-color: var(--accent);
  }

  .app-row.is-compact {
    grid-template-columns: minmax(150px, 1fr) 36px minmax(130px, 2fr) 44px 32px 60px;
    gap: var(--space-2);
  }

  /* An offline application keeps its settings reachable, so it stays listed but
     recedes behind everything that is actually making sound. */
  .app-row.is-offline {
    opacity: 0.55;
  }
  .app-row.is-offline:hover,
  .app-row.is-offline:focus-within {
    opacity: 1;
  }

  .row-lead {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .disclosure,
  .disclosure-spacer {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }
  .disclosure {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition:
      transform var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .disclosure:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  .disclosure:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .disclosure.is-open {
    transform: rotate(90deg);
    color: var(--accent);
  }

  .row-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: var(--lh-tight);
    margin-left: var(--space-1);
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
  .row-flag.is-warning {
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

  /* Row actions are always visible. Hiding them behind hover was the first
     version's mistake: a user who cannot see that a row can be renamed, pinned
     or removed concludes the application cannot do it. They sit quiet at rest
     and gain contrast on hover, rather than appearing from nothing. */
  .row-actions {
    display: flex;
    gap: 2px;
  }
  .row-actions :global(.icon-btn) {
    color: var(--text-placeholder);
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .app-row:hover .row-actions :global(.icon-btn) {
    color: var(--text-muted);
  }
  .row-actions :global(.icon-btn:hover) {
    color: var(--text-primary);
  }
  .row-actions :global(.is-on) {
    color: var(--accent);
  }

  .session-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    list-style: none;
    margin: 0;
    padding: var(--space-1) var(--space-4) var(--space-2) 34px;
    background: var(--bg-cap);
    border-top: 1px solid var(--deck-line);
  }

  /* Narrowing sheds affordances, never facts. The pin goes first because it
     still lives in the inspector; the meter goes next because the percentage
     already reports the level. The name, its secondary line, the fader, the
     percentage and mute all stay: "In Main" and "2 streams" are the reason the
     row is worth reading, and dropping them was what made the earlier narrow
     layout feel merely squeezed. */
  @media (max-width: 900px) {
    .app-row,
    .app-row.is-compact {
      grid-template-columns: minmax(150px, 1fr) 36px minmax(120px, 1.8fr) 44px 32px 30px;
    }
    .row-actions :global(.pin-action) {
      display: none;
    }
  }

  @media (max-width: 760px) {
    .app-row,
    .app-row.is-compact {
      grid-template-columns: minmax(140px, 1fr) minmax(110px, 1.6fr) 42px 32px 30px;
      padding-right: var(--space-3);
    }
    .row-meter {
      display: none;
    }
  }
</style>
