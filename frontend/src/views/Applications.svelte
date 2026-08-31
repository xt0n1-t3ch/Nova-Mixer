<script lang="ts">
  /**
   * The mixer desk.
   *
   * Composition, top to bottom: master deck, scene belt, channel legend, then a
   * scrolling column of channels. The master and the belt are fixed equipment;
   * only the channels scroll, so the thing every channel feeds into never
   * leaves the frame.
   *
   * One row is one *application*, not one Windows audio session. Windows hands
   * an application several sessions whenever it likes, which is why an earlier
   * version showed Discord twice. Sessions live behind a disclosure.
   */
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import Plus from "@lucide/svelte/icons/plus";
  import SearchX from "@lucide/svelte/icons/search-x";
  import AudioLines from "@lucide/svelte/icons/audio-lines";
  import VolumeOff from "@lucide/svelte/icons/volume-off";
  import Rows3 from "@lucide/svelte/icons/rows-3";
  import Rows2 from "@lucide/svelte/icons/rows-2";
  import PowerOff from "@lucide/svelte/icons/power-off";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import Bell from "@lucide/svelte/icons/bell";
  import type { Application, AppCandidate } from "../lib/api";
  import { listAppCandidates } from "../lib/api";
  import AddAppDialog from "../components/AddAppDialog.svelte";
  import AppInspector from "../components/AppInspector.svelte";
  import AppRow from "../components/AppRow.svelte";
  import Dialog from "../components/Dialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import MasterDeck from "../components/MasterDeck.svelte";
  import SceneBar from "../components/SceneBar.svelte";
  import {
    addApplicationFromPath,
    appPeaks,
    applicationSections,
    applications,
    audioAvailable,
    commitAppVolume,
    commitMasterVolume,
    commitSessionVolume,
    expandedApps,
    forgetApplication,
    groups,
    inspectedApp,
    inspectedAppKey,
    master,
    masterPeak,
    patchApplication,
    pendingVolumes,
    persistSettings,
    previewAppVolume,
    previewMasterVolume,
    previewSessionVolume,
    pushToast,
    refreshMixer,
    searchQuery,
    sessionPeaks,
    settings,
    sortedApplications,
    toggleAppMute,
    toggleExpanded,
    toggleMasterMute,
    toggleSessionMute,
  } from "../lib/stores";
  import { motionDuration } from "../lib/ux";
  import { t } from "../lib/i18n/index";

  let prefs = $derived($settings?.ui_prefs);
  let compact = $derived(prefs?.density === "compact");
  let runningCount = $derived($applications.filter((app) => app.running).length);

  let addOpen = $state(false);
  let candidates = $state<AppCandidate[]>([]);
  let forgetTarget = $state<Application | null>(null);

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

  async function openAdd(): Promise<void> {
    addOpen = true;
    try {
      candidates = await listAppCandidates();
    } catch {
      candidates = [];
    }
  }

  async function addPaths(paths: string[]): Promise<void> {
    let added = 0;
    for (const path of paths) {
      if (await addApplicationFromPath(path)) added += 1;
    }
    addOpen = false;
    if (added > 0) pushToast($t("add.addedToast", { count: added }), "success");
  }

  /** Browsing for an executable is how a closed application gets added. */
  async function browseForExecutable(): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: true,
        filters: [{ name: $t("add.filterName"), extensions: ["exe"] }],
      });
      if (!selected) return;
      await addPaths(Array.isArray(selected) ? selected : [selected]);
    } catch {
      pushToast($t("add.browseFailed"), "danger");
    }
  }

  async function confirmForget(): Promise<void> {
    const target = forgetTarget;
    if (!target) return;
    if (await forgetApplication(target)) {
      pushToast($t("inspector.forgotten", { app: target.display_name }), "success");
    }
    forgetTarget = null;
  }
</script>

<div class="desk" class:has-inspector={!!$inspectedApp}>
  <div class="desk-main">
    {#if !$audioAvailable}
      <div class="desk-empty">
        <EmptyState
          icon={VolumeOff}
          tone="danger"
          title={$t("view.applications.unavailable.title")}
          body={$t("view.applications.unavailable.body")}
        >
          {#snippet action()}
            <button class="btn btn-primary" onclick={() => void refreshMixer()}>
              {$t("common.retry")}
            </button>
          {/snippet}
        </EmptyState>
      </div>
    {:else}
      {#if $master}
        <MasterDeck
          master={$master}
          peak={$masterPeak}
          {runningCount}
          onInput={previewMasterVolume}
          onCommit={(value) => void commitMasterVolume(value)}
          onToggleMute={() => void toggleMasterMute()}
        />
      {/if}

      <SceneBar />

      <!-- View controls own a real band. They are not forced into the legend's
           last three grid columns, which only offered 164px for ~240px of tools
           and caused the clipping in the real build. -->
      <div class="channel-tools" aria-label={$t("desk.channels")}>
        <span class="tools-label">{$t("desk.channels")}</span>
        <div class="tools-filters">
          <button
            class="icon-btn icon-btn-sm"
            class:is-on={prefs?.show_offline}
            aria-pressed={prefs?.show_offline ?? false}
            onclick={() => setPref("show_offline", !(prefs?.show_offline ?? true))}
            title={$t("view.applications.showOffline")}
            aria-label={$t("view.applications.showOffline")}
          ><PowerOff size={13} /></button>
          <button
            class="icon-btn icon-btn-sm"
            class:is-on={prefs?.show_hidden}
            aria-pressed={prefs?.show_hidden ?? false}
            onclick={() => setPref("show_hidden", !(prefs?.show_hidden ?? false))}
            title={$t("view.applications.showHidden")}
            aria-label={$t("view.applications.showHidden")}
          ><EyeOff size={13} /></button>
          <button
            class="icon-btn icon-btn-sm"
            class:is-on={prefs?.show_system_sounds}
            aria-pressed={prefs?.show_system_sounds ?? false}
            onclick={() => setPref("show_system_sounds", !(prefs?.show_system_sounds ?? true))}
            title={$t("view.applications.showSystemSounds")}
            aria-label={$t("view.applications.showSystemSounds")}
          ><Bell size={13} /></button>
          <button
            class="icon-btn icon-btn-sm"
            onclick={() => setPref("density", compact ? "comfy" : "compact")}
            title={compact ? $t("settings.densityComfy") : $t("settings.densityCompact")}
            aria-label={compact ? $t("settings.densityComfy") : $t("settings.densityCompact")}
          >{#if compact}<Rows2 size={13} />{:else}<Rows3 size={13} />{/if}</button>
        </div>
        <button
          class="btn btn-sm btn-primary add-btn"
          onclick={() => void openAdd()}
          title={$t("view.applications.add")}
          aria-label={$t("view.applications.add")}
        >
          <Plus size={13} />
          <span class="add-label">{$t("view.applications.add")}</span>
        </button>
      </div>

      <!-- The legend labels columns only. Its grid is identical to AppRow's and
           contains no controls, so it cannot overflow into SOURCE/SIGNAL/LEVEL. -->
      <div class="legend" class:is-compact={compact} aria-hidden="true">
        <span class="legend-cell">{$t("desk.source")}</span>
        <span class="legend-cell">{$t("desk.signal")}</span>
        <span class="legend-cell">{$t("desk.level")}</span>
        <span></span><span></span><span></span>
      </div>

      <div class="channels">
        {#if $sortedApplications.length === 0}
          <div class="desk-empty">
            {#if $searchQuery.trim()}
              <EmptyState
                icon={SearchX}
                title={$t("view.applications.noMatch.title")}
                body={$t("view.applications.noMatch.body", { query: $searchQuery.trim() })}
              />
            {:else}
              <EmptyState
                icon={AudioLines}
                title={$t("view.applications.empty.title")}
                body={$t("view.applications.empty.body")}
              >
                {#snippet action()}
                  <button class="btn btn-primary" onclick={() => void openAdd()}>
                    <Plus size={14} />
                    {$t("view.applications.add")}
                  </button>
                {/snippet}
              </EmptyState>
            {/if}
          </div>
        {:else}
          {#each $applicationSections as section (section.id)}
            <div class="bank">
              <div class="bank-head">
                <span class="bank-name">{$t("section." + section.id)}</span>
                <span class="bank-rule" aria-hidden="true"></span>
                <span class="bank-count mono">{section.apps.length}</span>
              </div>
              <ul class="bank-list">
                {#each section.apps as app (app.app_key)}
                  <li
                    animate:flip={{ duration: motionDuration(220) }}
                    in:fly={{ y: 8, duration: motionDuration(220) }}
                    out:fly={{ y: -4, duration: motionDuration(140) }}
                  >
                    <AppRow
                      {app}
                      {compact}
                      peak={$appPeaks[app.app_key] ?? 0}
                      sessionPeaks={$sessionPeaks}
                      volume={$pendingVolumes[app.app_key] ?? app.volume}
                      groupName={groupNameFor(app.group_id)}
                      expanded={$expandedApps.has(app.app_key)}
                      inspected={$inspectedAppKey === app.app_key}
                      onInput={(value) => previewAppVolume(app.app_key, value)}
                      onCommit={(value) => void commitAppVolume(app.app_key, value)}
                      onToggleMute={() => void toggleAppMute(app)}
                      onToggleExpanded={() => toggleExpanded(app.app_key)}
                      onInspect={() =>
                        inspectedAppKey.set(
                          $inspectedAppKey === app.app_key ? null : app.app_key,
                        )}
                      onTogglePin={() =>
                        void patchApplication(app.app_key, { pinned: !app.pinned })}
                      onSessionInput={previewSessionVolume}
                      onSessionCommit={(liveId, value) => void commitSessionVolume(liveId, value)}
                      onSessionMute={(liveId, muted) => void toggleSessionMute(liveId, muted)}
                    />
                  </li>
                {/each}
              </ul>
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>

  {#if $inspectedApp}
    {@const target = $inspectedApp}
    <!-- Docked beside the desk when there is room; below that it overlays, so
         it never squeezes the channels into initials. -->
    <button
      class="inspector-scrim"
      aria-label={$t("common.close")}
      onclick={() => inspectedAppKey.set(null)}
    ></button>
    <aside class="inspector-slot" transition:fly={{ x: 24, duration: motionDuration(220) }}>
      <AppInspector
        app={target}
        groups={$groups}
        onClose={() => inspectedAppKey.set(null)}
        onPatch={(patch) => void patchApplication(target.app_key, patch)}
        onForget={() => (forgetTarget = target)}
      />
    </aside>
  {/if}
</div>

{#if addOpen}
  <AddAppDialog
    {candidates}
    onClose={() => (addOpen = false)}
    onAddCandidates={(paths) => void addPaths(paths)}
    onBrowse={() => void browseForExecutable()}
  />
{/if}

{#if forgetTarget}
  {@const target = forgetTarget}
  <Dialog
    title={$t("inspector.forgetConfirmTitle", { app: target.display_name })}
    description={$t("inspector.forgetConfirmBody")}
    tone="danger"
    width="420px"
    onClose={() => (forgetTarget = null)}
  >
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (forgetTarget = null)}>
        {$t("common.cancel")}
      </button>
      <button class="btn btn-danger" onclick={() => void confirmForget()}>
        {$t("inspector.forget")}
      </button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .desk {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    height: 100%;
    min-height: 0;
    background: var(--deck-bg);
  }

  .desk-main {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 1600px;
    min-width: 0;
    min-height: 0;
    margin-inline: auto;
    border-inline: 1px solid var(--deck-line);
  }

  /* Only the channels scroll. The master and scene belt stay put, because on a
     console the output section does not scroll away from you. */
  .channels {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
  }

  .desk-empty {
    padding: var(--space-6) var(--space-5);
  }

  .channel-tools {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 36px;
    padding: 0 var(--space-4);
    background: var(--bg-cap);
    border-bottom: 1px solid var(--deck-line);
  }
  .tools-label {
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .tools-filters {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
  }
  .add-btn {
    height: 26px;
    margin-left: var(--space-2);
  }

  .legend {
    display: grid;
    grid-template-columns: minmax(168px, 1.1fr) 40px minmax(140px, 2fr) 48px 32px 60px;
    align-items: center;
    gap: var(--space-3);
    height: 28px;
    padding: 0 var(--space-4) 0 calc(var(--space-2) + 2px);
    border-bottom: 1px solid var(--deck-line);
    background: color-mix(in oklab, var(--bg-cap) 60%, transparent);
  }
  .legend.is-compact {
    grid-template-columns: minmax(150px, 1fr) 36px minmax(130px, 2fr) 44px 32px 60px;
    gap: var(--space-2);
  }
  .legend-cell {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-placeholder);
  }
  .bank {
    padding-bottom: var(--space-2);
  }
  .bank-head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4) var(--space-2) var(--space-4);
  }
  /* The first bank sits directly under the legend, which already separates it
     from the tools above; a second gap there only wastes channel height. */
  .bank:first-child .bank-head {
    padding-top: var(--space-2);
  }
  .bank-name {
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .bank-rule {
    flex: 1;
    height: 1px;
    background: var(--deck-line);
  }
  .bank-count {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--text-placeholder);
    font-variant-numeric: tabular-nums;
  }

  .bank-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .inspector-slot {
    min-height: 0;
    background: var(--bg-card);
    border-left: 1px solid var(--deck-line-strong);
  }
  .inspector-scrim {
    display: none;
  }

  /* Docked only when the channels keep enough width to stay legible. */
  @media (min-width: 1240px) {
    .desk.has-inspector {
      grid-template-columns: minmax(0, 1fr) var(--inspector-width);
    }
  }

  @media (max-width: 1239px) {
    .inspector-slot {
      position: fixed;
      top: var(--chrome-height);
      right: 0;
      bottom: 0;
      width: min(var(--inspector-overlay-width), calc(100vw - var(--rail-width) - 24px));
      z-index: 80;
      box-shadow: var(--shadow-lg);
    }
    .inspector-scrim {
      display: block;
      position: fixed;
      top: var(--chrome-height);
      left: var(--rail-width);
      right: 0;
      bottom: 0;
      z-index: 79;
      background: rgba(0, 0, 0, 0.45);
      border: none;
      cursor: pointer;
    }
  }

  /* Narrow enough that a side panel would cover the desk entirely, so it
     becomes a bottom sheet instead. */
  @media (max-width: 640px) {
    .inspector-slot {
      top: auto;
      left: 0;
      right: 0;
      bottom: 0;
      width: auto;
      max-height: 82vh;
      border-left: none;
      border-top: 1px solid var(--deck-line-strong);
      border-radius: var(--radius-xl) var(--radius-xl) 0 0;
      overflow: hidden;
    }
  }

  @media (max-width: 900px) {
    .legend,
    .legend.is-compact {
      grid-template-columns: minmax(150px, 1fr) 36px minmax(120px, 1.8fr) 44px 32px 30px;
    }
    .channel-tools {
      min-height: 40px;
      padding: 0 var(--space-3);
    }
    .tools-label {
      display: none;
    }
    .add-btn {
      height: 28px;
    }
  }

  /* On a short window every band above the channels is competing with the
     thing the user came for, so the legend and bank heads tighten. */
  @media (max-height: 700px) {
    .legend {
      height: 24px;
    }
    .bank-head {
      padding-top: var(--space-2);
      padding-bottom: var(--space-1);
    }
  }

  @media (max-width: 760px) {
    .legend,
    .legend.is-compact {
      grid-template-columns: minmax(120px, 1fr) minmax(110px, 1.8fr) 42px 32px 30px;
      padding-right: var(--space-3);
    }
    .legend-cell:nth-child(2) {
      display: none;
    }
  }
</style>
