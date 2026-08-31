<script lang="ts">
  /**
   * Singleton toast stack.
   *
   * The region is a polite live region so a confirmation is announced without
   * interrupting whatever the user is doing; errors escalate to assertive
   * because they change what the user should do next.
   */
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import X from "@lucide/svelte/icons/x";
  import Check from "@lucide/svelte/icons/check";
  import Info from "@lucide/svelte/icons/info";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { dismissToast, toasts } from "../lib/stores";
  import { motionDuration } from "../lib/ux";
  import { t } from "../lib/i18n/index";
</script>

<div class="toast-region" aria-live="polite" aria-atomic="false">
  {#each $toasts as toast (toast.id)}
    <div
      class="toast glass-dialog"
      data-tone={toast.tone}
      role={toast.tone === "danger" ? "alert" : "status"}
      animate:flip={{ duration: motionDuration(200) }}
      in:fly={{ y: 12, duration: motionDuration(180) }}
      out:fly={{ y: 8, duration: motionDuration(140) }}
    >
      <span class="toast-icon" aria-hidden="true">
        {#if toast.tone === "success"}
          <Check size={15} />
        {:else if toast.tone === "danger"}
          <TriangleAlert size={15} />
        {:else}
          <Info size={15} />
        {/if}
      </span>
      <span class="toast-text">{toast.text}</span>
      <button
        class="icon-btn icon-btn-sm"
        onclick={() => dismissToast(toast.id)}
        aria-label={$t("toast.dismiss")}
      >
        <X size={13} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toast-region {
    position: fixed;
    bottom: var(--space-5);
    left: 50%;
    transform: translateX(-50%);
    z-index: 320;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: center;
    pointer-events: none;
    width: max-content;
    max-width: min(520px, 92vw);
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 10px 10px 10px 16px;
    border-radius: var(--radius-full);
    pointer-events: auto;
    min-width: 240px;
  }
  .toast[data-tone="success"] {
    --edge-color: var(--success);
  }
  .toast[data-tone="danger"] {
    --edge-color: var(--danger);
  }

  .toast-icon {
    display: inline-flex;
    color: var(--edge-color, var(--accent));
    flex-shrink: 0;
  }
  .toast-text {
    font-size: var(--fs-sm);
    color: var(--text-primary);
    line-height: var(--lh-snug);
    flex: 1;
    min-width: 0;
  }
</style>
