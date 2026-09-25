<script lang="ts">
  /**
   * Live status rail.
   *
   * Settings and About were a narrow column on a wide window. Stretching the
   * controls would only have made the toggles wider; this fills the space with
   * facts the user cannot get anywhere else — which endpoint is driving, how
   * many applications are actually controllable, whether efficiency mode really
   * took effect. Every value here is derived from live state, never invented.
   */
  import { onMount } from "svelte";
  import Activity from "@lucide/svelte/icons/activity";
  import Speaker from "@lucide/svelte/icons/speaker";
  import Leaf from "@lucide/svelte/icons/leaf";
  import Lock from "@lucide/svelte/icons/lock";
  import { getEfficiencyStatus, type EfficiencyStatus } from "../lib/api";
  import { applications, audioAvailable, master, settings } from "../lib/stores";
  import { formatPercent } from "../lib/volume";
  import { t } from "../lib/i18n/index";

  let efficiency = $state<EfficiencyStatus | null>(null);

  onMount(async () => {
    try {
      efficiency = await getEfficiencyStatus();
    } catch {
      efficiency = null;
    }
  });

  let managed = $derived($applications.length);
  // Playing means an active session now, not merely an open application.
  let running = $derived(
    $applications.filter((app) => app.sessions.some((session) => session.state === "active"))
      .length,
  );
  let sessions = $derived(
    $applications.reduce((total, app) => total + app.sessions.length, 0),
  );
  let locked = $derived(
    $applications.filter((app) => app.running && !app.controllable).length,
  );
  let remembered = $derived($applications.filter((app) => app.remembered).length);
  let groupCount = $derived($settings?.groups.length ?? 0);
  let sceneCount = $derived($settings?.scenes.length ?? 0);
</script>

<aside class="status-rail" aria-label={$t("status.title")}>
  <section class="status-block">
    <h2 class="status-title">
      <Speaker size={13} aria-hidden="true" />
      {$t("status.output")}
    </h2>
    {#if $master}
      <p class="status-lead truncate" title={$master.endpoint_name}>{$master.endpoint_name}</p>
      <dl class="status-facts">
        <div class="status-fact">
          <dt>{$t("status.level")}</dt>
          <dd class="mono">{formatPercent($master.volume)}</dd>
        </div>
        <div class="status-fact">
          <dt>{$t("status.engine")}</dt>
          <dd>
            <span class="dot" class:is-live={$audioAvailable}></span>
            {$audioAvailable ? $t("status.connected") : $t("status.unavailable")}
          </dd>
        </div>
      </dl>
    {:else}
      <p class="status-lead is-muted">{$t("status.unavailable")}</p>
    {/if}
  </section>

  <section class="status-block">
    <h2 class="status-title">
      <Activity size={13} aria-hidden="true" />
      {$t("status.activity")}
    </h2>
    <div class="status-counts">
      <div class="count">
        <span class="count-value mono">{running}</span>
        <span class="count-label">{$t("status.playing")}</span>
      </div>
      <div class="count">
        <span class="count-value mono">{sessions}</span>
        <span class="count-label">{$t("status.streams")}</span>
      </div>
      <div class="count">
        <span class="count-value mono">{managed}</span>
        <span class="count-label">{$t("status.managed")}</span>
      </div>
    </div>
    <dl class="status-facts">
      <div class="status-fact">
        <dt>{$t("status.remembered")}</dt>
        <dd class="mono">{remembered}</dd>
      </div>
      <div class="status-fact">
        <dt>{$t("status.groups")}</dt>
        <dd class="mono">{groupCount}</dd>
      </div>
      <div class="status-fact">
        <dt>{$t("scene.label")}</dt>
        <dd class="mono">{sceneCount}</dd>
      </div>
    </dl>
    {#if locked > 0}
      <p class="status-note">
        <Lock size={11} aria-hidden="true" />
        {$t("status.lockedNote", { count: locked })}
      </p>
    {/if}
  </section>

  {#if efficiency}
    <section class="status-block">
      <h2 class="status-title">
        <Leaf size={13} aria-hidden="true" class={efficiency.enabled ? "leaf-on" : undefined} />
        {$t("settings.efficiency")}
      </h2>
      <dl class="status-facts">
        <div class="status-fact">
          <dt>{$t("status.state")}</dt>
          <dd>
            <span class="dot" class:is-live={efficiency.enabled}></span>
            {efficiency.enabled ? $t("status.active") : $t("status.off")}
          </dd>
        </div>
        {#if efficiency.enabled}
          <div class="status-fact">
            <dt>{$t("status.priority")}</dt>
            <dd class="mono">Idle</dd>
          </div>
          <div class="status-fact">
            <dt>{$t("status.qos")}</dt>
            <dd class="mono">EcoQoS</dd>
          </div>
          <div class="status-fact">
            <dt>{$t("status.audioThread")}</dt>
            <dd>{$t("status.exempt")}</dd>
          </div>
        {/if}
      </dl>
      {#if !efficiency.supported}
        <p class="status-note">{efficiency.detail ?? $t("status.unsupported")}</p>
      {/if}
    </section>
  {/if}
</aside>

<style>
  /* `align-self: start` matters. The rail sits in a stretch-aligned grid on
     Settings and About, which ran its border to the bottom of the frame and
     left roughly 440px of empty bordered box under the last fact. A panel that
     ends where its content ends reads as finished; one padded out with void
     reads as broken, or as content that failed to load. */
  .status-rail {
    display: flex;
    flex-direction: column;
    align-self: start;
    gap: var(--space-3);
    min-width: 0;
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }

  .status-block + .status-block {
    padding-top: var(--space-4);
    border-top: 1px solid var(--border);
  }

  .status-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    margin-bottom: var(--space-3);
  }
  .status-title :global(.leaf-on) {
    color: var(--success);
  }

  .status-lead {
    font-size: var(--fs-sm);
    color: var(--text-primary);
    margin-bottom: var(--space-3);
  }
  .status-lead.is-muted {
    color: var(--text-muted);
  }

  /* Three live counters read faster than three more label/value rows, and they
     are the numbers a user actually checks. */
  .status-counts {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  .count {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .count-value {
    font-size: var(--fs-xl);
    font-weight: 700;
    line-height: 1;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .count-label {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wide);
    color: var(--text-faint);
  }

  .status-facts {
    display: grid;
    gap: 6px;
    margin: 0;
  }
  .status-fact {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .status-fact dt {
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }
  .status-fact dd {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-faint);
    flex-shrink: 0;
  }
  .dot.is-live {
    background: var(--success);
  }

  .status-note {
    display: flex;
    align-items: flex-start;
    gap: 5px;
    margin-top: var(--space-3);
    font-size: var(--fs-xs);
    color: var(--warning);
    line-height: var(--lh-snug);
  }
</style>
