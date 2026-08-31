<script lang="ts">
  /**
   * One live Windows session inside an expanded application row.
   *
   * Deliberately quieter than the application row: smaller type, no icon, no
   * actions. It exists for the moment a user asks "which of Discord's two
   * streams is the loud one?", not for everyday mixing.
   */
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import type { AudioSession } from "../lib/api";
  import { formatPercent, volumeValueText } from "../lib/volume";
  import Meter from "./Meter.svelte";
  import Range from "./Range.svelte";
  import { t } from "../lib/i18n/index";

  let {
    session,
    peak = 0,
    onInput,
    onCommit,
    onToggleMute,
  }: {
    session: AudioSession;
    peak?: number;
    onInput: (value: number) => void;
    onCommit: (value: number) => void;
    onToggleMute: () => void;
  } = $props();

  let label = $derived(
    session.display_name.trim() ||
      (session.process_id === null
        ? $t("app.sessionUnnamed")
        : $t("app.sessionPid", { pid: session.process_id })),
  );
</script>

<div class="session-row" class:is-idle={session.state === "inactive"}>
  <span class="session-name truncate" title={label}>{label}</span>

  {#if session.process_id !== null}
    <span class="session-pid mono">{session.process_id}</span>
  {:else}
    <span></span>
  {/if}

  <Meter peak={session.muted ? 0 : peak} bars={3} />

  <div class="session-fader">
    <Range
      value={session.volume}
      disabled={!session.controllable}
      muted={session.muted}
      size="sm"
      ariaLabel={$t("app.sessionVolume", { session: label })}
      ariaValueText={volumeValueText(session.volume, session.muted, $t("app.muted"))}
      {onInput}
      {onCommit}
    />
  </div>

  <output class="session-value mono" for="">{formatPercent(session.volume)}</output>

  <button
    class="icon-btn icon-btn-sm"
    class:is-danger={session.muted}
    disabled={!session.controllable}
    onclick={onToggleMute}
    aria-label={session.muted
      ? $t("app.unmuteSession", { session: label })
      : $t("app.muteSession", { session: label })}
    aria-pressed={session.muted}
  >
    {#if session.muted}
      <VolumeX size={13} />
    {:else}
      <Volume2 size={13} />
    {/if}
  </button>
</div>

<style>
  /* Columns deliberately echo the parent row's rhythm so the disclosure reads
     as a nested detail rather than as a different kind of object. */
  .session-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 48px auto minmax(90px, 1.6fr) 44px 26px;
    align-items: center;
    gap: var(--space-2);
    height: 36px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    transition: background var(--dur-fast) var(--ease);
  }
  .session-row:hover {
    background: var(--bg-elevated);
  }
  .session-row.is-idle {
    opacity: 0.6;
  }
  .session-row.is-idle:hover,
  .session-row.is-idle:focus-within {
    opacity: 1;
  }

  .session-name {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
  }
  .session-pid {
    font-size: var(--fs-2xs);
    color: var(--text-placeholder);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .session-fader {
    min-width: 0;
  }

  .session-value {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 700px) {
    .session-row {
      grid-template-columns: minmax(0, 1fr) minmax(80px, 1.4fr) 44px 26px;
    }
    .session-pid {
      display: none;
    }
  }
</style>
