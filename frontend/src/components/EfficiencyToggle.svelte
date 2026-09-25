<script lang="ts">
  /**
   * Windows Efficiency Mode.
   *
   * Task Manager shows the green leaf only when a process has both a low base
   * priority and EcoQoS; the backend applies both and exempts the audio worker
   * so the mixer stays responsive. This control reports what actually happened
   * rather than pretending the setting took effect, because on an unsupported
   * Windows build it silently would not.
   */
  import { onMount } from "svelte";
  import Leaf from "@lucide/svelte/icons/leaf";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { getEfficiencyStatus, setEfficiencyMode, type EfficiencyStatus } from "../lib/api";
  import { t } from "../lib/i18n/index";

  let { enabled, onChange }: { enabled: boolean; onChange: (enabled: boolean) => void } = $props();

  // Seeded from the persisted preference so the control paints correctly on the
  // first frame, then replaced by what the process actually reports.
  let status = $state<EfficiencyStatus>({ supported: true, enabled: false, detail: null });
  let busy = $state(false);

  onMount(async () => {
    status = { supported: true, enabled, detail: null };
    try {
      // An empty reply keeps the persisted value instead of blanking the toggle.
      status = (await getEfficiencyStatus()) ?? status;
    } catch {
      // Leave the persisted value; toggling will surface any real problem.
    }
  });

  async function toggle(next: boolean): Promise<void> {
    busy = true;
    try {
      status = (await setEfficiencyMode(next)) ?? status;
      onChange(status.enabled);
    } catch (error) {
      status = {
        supported: status.supported,
        enabled: status.enabled,
        detail: error instanceof Error ? error.message : String(error),
      };
    } finally {
      busy = false;
    }
  }
</script>

<div class="setting-row">
  <div class="setting-copy">
    <div class="setting-label">
      <span class="leaf" class:is-on={status.enabled} aria-hidden="true">
        <Leaf size={13} />
      </span>
      {$t("settings.efficiency")}
      {#if status.enabled}
        <span class="chip chip-success eco-chip">EcoQoS</span>
      {/if}
    </div>
    <div class="setting-hint">{$t("settings.efficiencyHint")}</div>
    {#if status.detail}
      <p class="setting-note">
        <TriangleAlert size={12} aria-hidden="true" />
        {status.detail}
      </p>
    {/if}
  </div>

  <label class="toggle">
    <input
      type="checkbox"
      checked={status.enabled}
      disabled={!status.supported || busy}
      aria-label={$t("settings.efficiency")}
      onchange={(event) => void toggle(event.currentTarget.checked)}
    />
    <span class="toggle-slider"></span>
  </label>
</div>

<style>
  .setting-label {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  /* The leaf greys out when off and takes Windows' own green when on, so the
     control reads as the same idea as the Task Manager column. */
  .leaf {
    display: inline-flex;
    color: var(--text-faint);
    transition: color var(--dur-normal) var(--ease);
  }
  .leaf.is-on {
    color: var(--success);
  }

  .eco-chip {
    font-family: var(--font-mono);
    letter-spacing: 0;
    text-transform: none;
  }

  .setting-note {
    display: flex;
    align-items: flex-start;
    gap: 5px;
    margin-top: 6px;
    font-size: var(--fs-xs);
    color: var(--warning);
    line-height: var(--lh-snug);
  }
</style>
