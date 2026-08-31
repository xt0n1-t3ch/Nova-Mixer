<script lang="ts">
  /**
   * Picks applications to bind to a group.
   *
   * The list comes from live audio sessions, not from every running process.
   * The previous implementation listed all processes, which buried the handful
   * of applications that actually produce sound under hundreds of services.
   */
  import Search from "@lucide/svelte/icons/search";
  import Check from "@lucide/svelte/icons/check";
  import type { AppBinding, AudioSession } from "../lib/api";
  import { matchesQuery } from "../lib/ux";
  import AppIcon from "./AppIcon.svelte";
  import Dialog from "./Dialog.svelte";
  import EmptyState from "./EmptyState.svelte";
  import AudioLines from "@lucide/svelte/icons/audio-lines";
  import { t } from "../lib/i18n/index";

  let {
    sessions,
    excludeKeys,
    onClose,
    onConfirm,
  }: {
    sessions: AudioSession[];
    /** Keys already bound to this group; shown as selected and inert. */
    excludeKeys: string[];
    onClose: () => void;
    onConfirm: (bindings: AppBinding[]) => void;
  } = $props();

  let query = $state("");
  let picked = $state<Set<string>>(new Set());

  // One row per application, not per session: a browser can own several
  // sessions and the user thinks in applications.
  let candidates = $derived.by(() => {
    const seen = new Map<string, AudioSession>();
    for (const session of sessions) {
      if (session.is_system_sounds) continue;
      if (excludeKeys.includes(session.app_key)) continue;
      if (!seen.has(session.app_key)) seen.set(session.app_key, session);
    }
    return [...seen.values()]
      .filter((session) =>
        matchesQuery(query, [session.display_name, session.executable_name]),
      )
      .sort((a, b) => a.display_name.localeCompare(b.display_name, undefined, { sensitivity: "base" }));
  });

  function toggle(key: string): void {
    const next = new Set(picked);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    picked = next;
  }

  function confirm(): void {
    const bindings: AppBinding[] = candidates
      .filter((session) => picked.has(session.app_key))
      .map((session) => ({
        app_key: session.app_key,
        display_name: session.display_name,
        executable_name: session.executable_name,
        executable_path: session.executable_path,
      }));
    onConfirm(bindings);
  }
</script>

<Dialog title={$t("picker.title")} description={$t("picker.subtitle")} width="520px" {onClose}>
  <div class="picker-search">
    <Search class="picker-search-icon" size={14} aria-hidden="true" />
    <input
      type="search"
      bind:value={query}
      placeholder={$t("picker.placeholder")}
      aria-label={$t("picker.placeholder")}
    />
  </div>

  {#if candidates.length === 0}
    <EmptyState icon={AudioLines} title={$t("picker.empty")} />
  {:else}
    <ul class="picker-list" role="listbox" aria-multiselectable="true" aria-label={$t("picker.title")}>
      {#each candidates as session (session.app_key)}
        {@const isPicked = picked.has(session.app_key)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={isPicked}
            class="picker-item"
            class:is-picked={isPicked}
            onclick={() => toggle(session.app_key)}
          >
            <AppIcon src={session.icon} name={session.display_name} appKey={session.app_key} size={30} />
            <span class="picker-text">
              <span class="picker-name truncate">{session.display_name}</span>
              {#if session.executable_name}
                <span class="picker-exe truncate">{session.executable_name}</span>
              {/if}
            </span>
            <span class="picker-check" aria-hidden="true">
              {#if isPicked}<Check size={14} />{/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#snippet footer()}
    <span class="picker-count">{$t("picker.selected", { count: picked.size })}</span>
    <button class="btn btn-ghost" onclick={onClose}>{$t("common.cancel")}</button>
    <button class="btn btn-primary" disabled={picked.size === 0} onclick={confirm}>
      {$t("common.add")}
    </button>
  {/snippet}
</Dialog>

<style>
  .picker-search {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: var(--space-3);
  }
  .picker-search :global(.picker-search-icon) {
    position: absolute;
    left: 12px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .picker-search input {
    padding-left: 34px;
    height: 36px;
  }

  .picker-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 340px;
    overflow-y: auto;
  }

  .picker-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    text-align: left;
    transition: background var(--dur-fast) var(--ease);
  }
  .picker-item:hover {
    background: var(--bg-card-hover);
  }
  .picker-item:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .picker-item.is-picked {
    background: var(--accent-dim);
  }

  .picker-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
    line-height: var(--lh-tight);
  }
  .picker-name {
    font-size: var(--fs-sm);
    font-weight: 500;
    color: var(--text-primary);
  }
  .picker-exe {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    margin-top: 2px;
  }

  .picker-check {
    width: 18px;
    display: inline-flex;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .picker-count {
    margin-right: auto;
    font-size: var(--fs-xs);
    color: var(--text-muted);
    align-self: center;
  }
</style>
