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
  <span class="session-name truncate" title={label}>
    {label}
    {#if session.process_id !== null}<span class="session-pid mono">{session.process_id}</span>{/if}
  </span>

  <div class="session-level">
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
    <div class="session-meter">
      <Meter peak={session.muted ? 0 : peak} bars={24} />
    </div>
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
  /* The parent's grid minus its identity width, so each stream's fader sits
     under the application's fader. */
  .session-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(160px, 2.6fr) 44px 34px;
    align-items: center;
    gap: var(--space-4);
    min-height: 36px;
    padding: 2px 62px 2px 0;
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
    margin-left: 6px;
    font-size: var(--fs-2xs);
    color: var(--text-faint);
  }
  .session-level {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .session-meter {
    height: 3px;
    padding: 0 2px;
  }
  .session-meter :global(.meter) {
    height: 100%;
  }
  .session-value {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>