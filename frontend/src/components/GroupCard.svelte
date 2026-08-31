<script lang="ts">
  import Pencil from "@lucide/svelte/icons/pencil";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Star from "@lucide/svelte/icons/star";
  import type { Group } from "../lib/api";
  import { formatPercent } from "../lib/volume";
  import AppIcon from "./AppIcon.svelte";
  import { t } from "../lib/i18n/index";

  let {
    group,
    isActive,
    isSelected,
    canDelete,
    onSelect,
    onRename,
    onDelete,
  }: {
    group: Group;
    isActive: boolean;
    isSelected: boolean;
    canDelete: boolean;
    onSelect: () => void;
    onRename: () => void;
    onDelete: () => void;
  } = $props();

  // Three faces plus a counter reads faster than a long wrapping row of icons.
  const PREVIEW_LIMIT = 3;
  let preview = $derived(group.apps.slice(0, PREVIEW_LIMIT));
  let overflow = $derived(Math.max(0, group.apps.length - PREVIEW_LIMIT));
</script>

<div class="group-card edge-accent" class:is-selected={isSelected} class:is-active={isActive}>
  <!-- The whole card selects; the icon buttons sit outside it so they are not
       nested inside a button, which is invalid and breaks keyboard order. -->
  <button class="group-hit" onclick={onSelect} aria-pressed={isSelected}>
    <span class="group-head">
      <span class="group-name truncate">{group.name}</span>
      {#if isActive}
        <span class="chip chip-accent">{$t("groups.active")}</span>
      {:else if group.is_default}
        <span class="chip chip-neutral">{$t("common.default")}</span>
      {/if}
    </span>

    <span class="group-meta">
      <span class="group-count">{$t("groups.apps", { count: group.apps.length })}</span>
      <span class="group-volume mono">{formatPercent(group.volume)}</span>
    </span>

    <span class="group-apps">
      {#each preview as app (app.app_key)}
        <AppIcon src={null} name={app.display_name} appKey={app.app_key} size={22} />
      {/each}
      {#if overflow > 0}
        <span class="group-overflow mono">+{overflow}</span>
      {/if}
    </span>
  </button>

  <div class="group-actions">
    <button class="icon-btn icon-btn-sm" onclick={onRename} aria-label={$t("groups.renameTitle")} title={$t("common.rename")}>
      <Pencil size={13} />
    </button>
    <button
      class="icon-btn icon-btn-sm"
      onclick={onDelete}
      disabled={!canDelete}
      aria-label={$t("groups.deleteTitle", { group: group.name })}
      title={canDelete ? $t("common.delete") : $t("groups.cannotDeleteLast")}
    >
      <Trash2 size={13} />
    </button>
  </div>

  {#if isActive}
    <span class="group-star" aria-hidden="true"><Star size={12} /></span>
  {/if}
</div>

<style>
  .group-card {
    position: relative;
    overflow: hidden;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    transition:
      border-color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .group-card:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-hover);
  }
  .group-card.is-selected {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .group-card.is-active {
    --edge-color: var(--accent);
  }
  .group-card:not(.is-active)::before {
    opacity: 0;
  }

  .group-hit {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-4) 76px var(--space-4) var(--space-4);
    text-align: left;
    border-radius: var(--radius-lg);
  }
  .group-hit:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  .group-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .group-name {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
  }

  .group-meta {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }
  .group-volume {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .group-volume::before {
    content: "·";
    margin-right: var(--space-2);
    color: var(--text-placeholder);
  }

  .group-apps {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 22px;
  }
  .group-overflow {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
  }

  .group-actions {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease);
  }
  .group-card:hover .group-actions,
  .group-card:focus-within .group-actions {
    opacity: 1;
  }

  .group-star {
    position: absolute;
    bottom: var(--space-3);
    right: var(--space-4);
    color: var(--accent);
  }
</style>
