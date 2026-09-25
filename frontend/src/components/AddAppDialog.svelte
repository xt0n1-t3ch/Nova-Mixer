<script lang="ts">
  /**
   * Adds an application to the registry.
   *
   * Two routes, because there are two situations. If the application is playing
   * audio right now it is already in the candidate list and one click adds it.
   * If it is closed — the case the previous version could not handle at all —
   * the user browses for its executable.
   */
  import Search from "@lucide/svelte/icons/search";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import Check from "@lucide/svelte/icons/check";
  import Plus from "@lucide/svelte/icons/plus";
  import AudioLines from "@lucide/svelte/icons/audio-lines";
  import type { AppCandidate } from "../lib/api";
  import { matchesQuery } from "../lib/ux";
  import AppIcon from "./AppIcon.svelte";
  import Dialog from "./Dialog.svelte";
  import EmptyState from "./EmptyState.svelte";
  import { t } from "../lib/i18n/index";

  let {
    candidates,
    onClose,
    onAddCandidates,
    onBrowse,
  }: {
    candidates: AppCandidate[];
    onClose: () => void;
    onAddCandidates: (paths: string[]) => void;
    onBrowse: () => void;
  } = $props();

  let query = $state("");
  let picked = $state<Set<string>>(new Set());

  // Running applications first: they are what the user is most likely adding,
  // and their identity is exact rather than guessed from a path.
  let filtered = $derived(
    candidates
      .filter((candidate) =>
        matchesQuery(query, [candidate.display_name, candidate.executable_name]),
      )
      .sort((a, b) => {
        if (a.already_managed !== b.already_managed) return a.already_managed ? 1 : -1;
        if (a.running !== b.running) return a.running ? -1 : 1;
        return a.display_name.localeCompare(b.display_name, undefined, { sensitivity: "base" });
      }),
  );

  let selectable = $derived(filtered.filter((candidate) => !candidate.already_managed));

  function toggle(candidate: AppCandidate): void {
    if (candidate.already_managed || !candidate.executable_path) return;
    const next = new Set(picked);
    if (next.has(candidate.executable_path)) {
      next.delete(candidate.executable_path);
    } else {
      next.add(candidate.executable_path);
    }
    picked = next;
  }
</script>

<Dialog
  title={$t("add.title")}
  description={$t("add.subtitle")}
  width="540px"
  {onClose}
>
  <div class="add-search">
    <Search class="add-search-icon" size={14} aria-hidden="true" />
    <input
      type="search"
      bind:value={query}
      placeholder={$t("add.placeholder")}
      aria-label={$t("add.placeholder")}
    />
  </div>

  {#if filtered.length === 0}
    <EmptyState
      icon={AudioLines}
      title={query.trim() ? $t("add.noMatch") : $t("add.empty")}
      body={$t("add.emptyHint")}
    />
  {:else}
    <ul class="add-list" role="listbox" aria-multiselectable="true" aria-label={$t("add.title")}>
      {#each filtered as candidate (candidate.app_key)}
        {@const isPicked =
          candidate.executable_path !== null && picked.has(candidate.executable_path)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={isPicked}
            class="add-item"
            class:is-picked={isPicked}
            class:is-managed={candidate.already_managed}
            disabled={candidate.already_managed || !candidate.executable_path}
            onclick={() => toggle(candidate)}
          >
            <AppIcon
              src={candidate.icon}
              name={candidate.display_name}
              size={30}
              dimmed={!candidate.running}
            />
            <span class="add-text">
              <span class="add-name truncate">{candidate.display_name}</span>
              <span class="add-meta truncate">
                {#if candidate.already_managed}
                  {$t("add.alreadyAdded")}
                {:else if candidate.running}
                  <span class="live-dot" aria-hidden="true"></span>
                  {$t("add.playingNow")}
                {:else if candidate.executable_name}
                  {candidate.executable_name}
                {/if}
              </span>
            </span>
            <span class="add-check" aria-hidden="true">
              {#if candidate.already_managed}
                <Check size={13} />
              {:else if isPicked}
                <Check size={14} />
              {/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#snippet footer()}
    <button class="btn btn-sm" onclick={onBrowse}>
      <FolderOpen size={13} />
      {$t("add.browse")}
    </button>
    <span class="add-count">
      {picked.size > 0 ? $t("add.selected", { count: picked.size }) : ""}
    </span>
    <button class="btn btn-ghost" onclick={onClose}>{$t("common.cancel")}</button>
    <button
      class="btn btn-primary"
      disabled={picked.size === 0 && selectable.length > 0}
      onclick={() => onAddCandidates([...picked])}
    >
      <Plus size={13} />
      {$t("common.add")}
    </button>
  {/snippet}
</Dialog>

<style>
  .add-search {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: var(--space-3);
  }
  .add-search :global(.add-search-icon) {
    position: absolute;
    left: 12px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .add-search input {
    padding-left: 34px;
    height: 36px;
  }

  .add-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 360px;
    overflow-y: auto;
  }

  .add-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    text-align: left;
    transition: background var(--dur-fast) var(--ease);
  }
  .add-item:hover:not(:disabled) {
    background: var(--bg-card-hover);
  }
  .add-item:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .add-item.is-picked {
    background: var(--accent-dim);
  }
  .add-item.is-managed {
    opacity: 0.5;
    cursor: default;
  }

  .add-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
    line-height: var(--lh-tight);
  }
  .add-name {
    font-size: var(--fs-sm);
    font-weight: 500;
    color: var(--text-primary);
  }
  .add-meta {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-xs);
    color: var(--text-muted);
    margin-top: 2px;
  }
  .live-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--success);
    flex-shrink: 0;
  }

  .add-check {
    width: 18px;
    display: inline-flex;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .add-count {
    margin-right: auto;
    font-size: var(--fs-xs);
    color: var(--text-muted);
    align-self: center;
  }
</style>
