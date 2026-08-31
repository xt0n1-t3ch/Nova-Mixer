<script lang="ts">
  /**
   * Shared empty and error placeholder. Every "nothing here" surface uses it so
   * the app never shows a blank rectangle with no explanation.
   */
  import type { Component, Snippet } from "svelte";

  let {
    icon,
    title,
    body,
    tone = "neutral",
    action,
  }: {
    icon?: Component<{ size?: number }>;
    title: string;
    body?: string;
    tone?: "neutral" | "danger";
    action?: Snippet;
  } = $props();
</script>

<div class="empty" data-tone={tone}>
  {#if icon}
    {@const Icon = icon}
    <span class="empty-icon aura-badge" aria-hidden="true">
      <Icon size={22} />
    </span>
  {/if}
  <h3 class="empty-title">{title}</h3>
  {#if body}
    <p class="empty-body">{body}</p>
  {/if}
  {#if action}
    <div class="empty-action">{@render action()}</div>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: var(--space-8) var(--space-5);
    border: 1px dashed var(--border);
    border-radius: var(--radius-2xl);
    background: var(--bg-cap);
  }

  .empty-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-lg);
    margin-bottom: var(--space-4);
  }
  .empty[data-tone="danger"] .empty-icon {
    background: var(--danger-dim);
    color: var(--danger);
    border-color: transparent;
  }

  .empty-title {
    font-size: var(--fs-lg);
    font-weight: 600;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
  }

  .empty-body {
    margin-top: 6px;
    font-size: var(--fs-sm);
    color: var(--text-muted);
    line-height: var(--lh-snug);
    max-width: 380px;
  }

  .empty-action {
    margin-top: var(--space-4);
  }
</style>
