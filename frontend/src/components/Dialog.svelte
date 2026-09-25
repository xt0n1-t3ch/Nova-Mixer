<script lang="ts">
  /**
   * The one floating-surface primitive. Every modal in the app is this
   * component plus content, so blur level, elevation, close affordance, focus
   * behaviour and Escape handling stay identical everywhere.
   */
  import type { Snippet } from "svelte";
  import { fade, scale } from "svelte/transition";
  import X from "@lucide/svelte/icons/x";
  import { focusTrap } from "../actions/focusTrap";
  import { motionDuration } from "../lib/ux";
  import { t } from "../lib/i18n/index";

  let {
    title,
    description,
    onClose,
    width = "440px",
    tone,
    children,
    footer,
  }: {
    title: string;
    description?: string;
    onClose: () => void;
    width?: string;
    /** Colours the leading edge, e.g. a destructive confirmation. */
    tone?: "danger" | "success" | "warning";
    /** Optional: a confirmation dialog is title, description, and buttons only. */
    children?: Snippet;
    footer?: Snippet;
  } = $props();

  const titleId = `dialog-title-${Math.random().toString(36).slice(2, 9)}`;
  const descId = `dialog-desc-${Math.random().toString(36).slice(2, 9)}`;

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
    }
  }

  let edgeColor = $derived(
    tone === "danger"
      ? "var(--danger)"
      : tone === "success"
        ? "var(--success)"
        : tone === "warning"
          ? "var(--warning)"
          : undefined,
  );
</script>

<div
  class="dialog-backdrop"
  role="presentation"
  transition:fade={{ duration: motionDuration(140) }}
  onclick={onClose}
></div>

<div
  class="dialog glass-dialog"
  style:--dialog-width={width}
  style:--edge-color={edgeColor}
  role="dialog"
  aria-modal="true"
  aria-labelledby={titleId}
  aria-describedby={description ? descId : undefined}
  tabindex="-1"
  onkeydown={onKeydown}
  use:focusTrap
  transition:scale={{ duration: motionDuration(160), start: 0.97, opacity: 0 }}
>
  <button class="dialog-close" onclick={onClose} aria-label={$t("common.close")}>
    <X size={16} />
  </button>

  <div class="dialog-head">
    <h2 class="dialog-title" id={titleId}>{title}</h2>
    {#if description}
      <p class="dialog-desc" id={descId}>{description}</p>
    {/if}
  </div>

  {#if children}
    <div class="dialog-body">
      {@render children()}
    </div>
  {/if}

  {#if footer}
    <div class="dialog-footer">
      {@render footer()}
    </div>
  {/if}
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 240;
    background: rgba(0, 0, 0, 0.55);
    border: none;
  }
  :global([data-theme="light"]) .dialog-backdrop {
    background: rgba(15, 23, 42, 0.28);
  }

  .dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 241;
    width: min(var(--dialog-width), 92vw);
    max-height: 84vh;
    display: flex;
    flex-direction: column;
  }

  .dialog-head {
    padding: var(--space-5) var(--space-5) var(--space-3) var(--space-5);
  }
  .dialog-title {
    font-size: var(--fs-lg);
    font-weight: 650;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
    /* Leaves room for the close button without shifting the text. */
    padding-right: 36px;
  }
  .dialog-desc {
    margin-top: 6px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    line-height: var(--lh-snug);
  }

  .dialog-body {
    padding: 0 var(--space-5) var(--space-5);
    overflow-y: auto;
    min-height: 0;
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-5);
    border-top: 1px solid var(--border);
    background: var(--bg-sunken);
  }
  /* A confirmation has no body, so the head would otherwise sit tight against
     the footer rule. */
  .dialog-head + .dialog-footer {
    margin-top: var(--space-2);
  }
</style>
