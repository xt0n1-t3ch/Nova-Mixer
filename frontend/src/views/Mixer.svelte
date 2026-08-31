<script lang="ts">
  /**
   * The mixer.
   *
   * Rows are keyed by `live_id` and animated with `flip`, so when an
   * application launches after NovaMixer its row slides in on its own. That
   * animation is the visible proof of the fix: the previous implementation
   * cached a one-time session enumeration and never saw the new application at
   * all.
   */
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import SearchX from "@lucide/svelte/icons/search-x";
  import AudioLines from "@lucide/svelte/icons/audio-lines";
  import VolumeOff from "@lucide/svelte/icons/volume-off";
  import Rows3 from "@lucide/svelte/icons/rows-3";
  import Rows2 from "@lucide/svelte/icons/rows-2";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import Bell from "@lucide/svelte/icons/bell";
  import AppRow from "../components/AppRow.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import MasterStrip from "../components/MasterStrip.svelte";
  import {
    audioAvailable,
    commitMasterVolume,
    commitSessionVolume,
    groups,
    master,
    masterPeak,
    peaks,
    pendingVolumes,
    persistSettings,
    previewMasterVolume,
    previewSessionVolume,
    refreshMixer,
    searchQuery,
    settings,
    sortedSessions,
    toggleMasterMute,
    toggleSessionMute,
  } from "../lib/stores";
  import { motionDuration } from "../lib/ux";
  import { t } from "../lib/i18n/index";

  let prefs = $derived($settings?.ui_prefs);
  let compact = $derived(prefs?.density === "compact");

  function groupNameFor(groupId: string | null): string | null {
    if (!groupId) return null;
    return $groups.find((group) => group.id === groupId)?.name ?? null;
  }

  function setPref<K extends keyof NonNullable<typeof prefs>>(
    key: K,
    value: NonNullable<typeof prefs>[K],
  ): void {
    const current = $settings;
    if (!current) return;
    persistSettings({ ...current, ui_prefs: { ...current.ui_prefs, [key]: value } });
  }
</script>

<div class="view">
  <div class="view-header">
    <div>
      <h1 class="view-title">{$t("view.mixer.title")}</h1>
      <p class="view-subtitle">{$t("view.mixer.subtitle")}</p>
    </div>
    <div class="header-actions">
      <button
        class="icon-btn"
        class:is-on={prefs?.show_inactive}
        aria-pressed={prefs?.show_inactive ?? false}
        onclick={() => setPref("show_inactive", !(prefs?.show_inactive ?? true))}
        title={$t("mixer.showInactive")}
        aria-label={$t("mixer.showInactive")}
      >
        <EyeOff size={15} />
      </button>
      <button
        class="icon-btn"
        class:is-on={prefs?.show_system_sounds}
        aria-pressed={prefs?.show_system_sounds ?? false}
        onclick={() => setPref("show_system_sounds", !(prefs?.show_system_sounds ?? true))}
        title={$t("mixer.showSystemSounds")}
        aria-label={$t("mixer.showSystemSounds")}
      >
        <Bell size={15} />
      </button>
      <div class="seg" role="group" aria-label={$t("settings.density")}>
        <button
          class="seg-btn"
          class:active={!compact}
          aria-pressed={!compact}
          onclick={() => setPref("density", "comfy")}
          title={$t("settings.densityComfy")}
        >
          <Rows2 size={13} />
        </button>
        <button
          class="seg-btn"
          class:active={compact}
          aria-pressed={compact}
          onclick={() => setPref("density", "compact")}
          title={$t("settings.densityCompact")}
        >
          <Rows3 size={13} />
        </button>
      </div>
    </div>
  </div>

  {#if !$audioAvailable}
    <EmptyState
      icon={VolumeOff}
      tone="danger"
      title={$t("mixer.unavailable.title")}
      body={$t("mixer.unavailable.body")}
    >
      {#snippet action()}
        <button class="btn btn-primary" onclick={() => void refreshMixer()}>
          {$t("common.retry")}
        </button>
      {/snippet}
    </EmptyState>
  {:else}
    {#if $master}
      <MasterStrip
        master={$master}
        peak={$masterPeak}
        onInput={previewMasterVolume}
        onCommit={(value) => void commitMasterVolume(value)}
        onToggleMute={() => void toggleMasterMute()}
      />
    {/if}

    {#if $sortedSessions.length === 0}
      {#if $searchQuery.trim()}
        <EmptyState
          icon={SearchX}
          title={$t("mixer.emptySearch.title")}
          body={$t("mixer.emptySearch.body", { query: $searchQuery.trim() })}
        />
      {:else}
        <EmptyState
          icon={AudioLines}
          title={$t("mixer.empty.title")}
          body={$t("mixer.empty.body")}
        />
      {/if}
    {:else}
      <div class="section-title" id="session-list-label">
        {$t("mixer.count", { count: $sortedSessions.length })}
      </div>
      <ul class="session-list" aria-labelledby="session-list-label">
        {#each $sortedSessions as session (session.live_id)}
          <li
            animate:flip={{ duration: motionDuration(220) }}
            in:fly={{ y: 10, duration: motionDuration(220) }}
            out:fly={{ y: -6, duration: motionDuration(140) }}
          >
            <AppRow
              {session}
              {compact}
              peak={$peaks[session.live_id] ?? 0}
              volume={$pendingVolumes[session.live_id] ?? session.volume}
              groupName={groupNameFor(session.group_id)}
              onInput={(value) => previewSessionVolume(session.live_id, value)}
              onCommit={(value) => void commitSessionVolume(session.live_id, value)}
              onToggleMute={() => void toggleSessionMute(session)}
            />
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .view {
    min-width: 0;
  }

  .session-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>
