<script lang="ts">
  /**
   * The Applications view.
   *
   * One row per application, not per Windows audio session — that distinction is
   * the whole point of this screen. An application stays listed while it is
   * closed so its settings remain reachable, its sessions collapse behind a
   * disclosure, and every management action lives on the row or in the
   * inspector rail rather than being unavailable.
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
  import MasterStrip from "../components/MasterStrip.svelte";
  import SceneBar from "../components/SceneBar.svelte";
  import {
    addApplicationFromPath,
    appPeaks,
    applicationSections,
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
    if (added > 0) {
      pushToast($t("add.addedToast", { count: added }), "success");
    }
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

  let sectionLabel = (id: string): string => $t("section." + id);
</script>

<div class="view" class:has-rail={!!$inspectedApp}>
  <div class="view-main">
    <div class="view-header">
      <div>
        <h1 class="view-title">{$t("view.applications.title")}</h1>
        <p class="view-subtitle">{$t("view.applications.subtitle")}</p>
      </div>
      <div class="header-actions">
        <button
          class="icon-btn"
          class:is-on={prefs?.show_offline}
          aria-pressed={prefs?.show_offline ?? false}
          onclick={() => setPref("show_offline", !(prefs?.show_offline ?? true))}
          title={$t("view.applications.showOffline")}
          aria-label={$t("view.applications.showOffline")}
        >
          <PowerOff size={15} />
        </button>
        <button
          class="icon-btn"
          class:is-on={prefs?.show_hidden}
          aria-pressed={prefs?.show_hidden ?? false}
          onclick={() => setPref("show_hidden", !(prefs?.show_hidden ?? false))}
          title={$t("view.applications.showHidden")}
          aria-label={$t("view.applications.showHidden")}
        >
          <EyeOff size={15} />
        </button>
        <button
          class="icon-btn"
          class:is-on={prefs?.show_system_sounds}
          aria-pressed={prefs?.show_system_sounds ?? false}
          onclick={() => setPref("show_system_sounds", !(prefs?.show_system_sounds ?? true))}
          title={$t("view.applications.showSystemSounds")}
          aria-label={$t("view.applications.showSystemSounds")}
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

        <button class="btn btn-primary" onclick={() => void openAdd()}>
          <Plus size={14} />
          {$t("view.applications.add")}
        </button>
      </div>
    </div>

    {#if !$audioAvailable}
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

      <SceneBar />

      {#if $sortedApplications.length === 0}
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
      {:else}
        {#each $applicationSections as section (section.id)}
          <section class="app-section">
            <h2 class="section-title">
              {sectionLabel(section.id)}
              <span class="section-count mono">{section.apps.length}</span>
            </h2>
            <ul class="app-list">
              {#each section.apps as app (app.app_key)}
                <li
                  animate:flip={{ duration: motionDuration(220) }}
                  in:fly={{ y: 10, duration: motionDuration(220) }}
                  out:fly={{ y: -6, duration: motionDuration(140) }}
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
                      inspectedAppKey.set($inspectedAppKey === app.app_key ? null : app.app_key)}
                    onTogglePin={() =>
                      void patchApplication(app.app_key, { pinned: !app.pinned })}
                    onSessionInput={previewSessionVolume}
                    onSessionCommit={(liveId, value) => void commitSessionVolume(liveId, value)}
                    onSessionMute={(liveId, muted) => void toggleSessionMute(liveId, muted)}
                  />
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      {/if}
    {/if}
  </div>

  {#if $inspectedApp}
    {@const target = $inspectedApp}
    <div class="view-rail" transition:fly={{ x: 20, duration: motionDuration(200) }}>
      <AppInspector
        app={target}
        groups={$groups}
        onClose={() => inspectedAppKey.set(null)}
        onPatch={(patch) => void patchApplication(target.app_key, patch)}
        onForget={() => (forgetTarget = target)}
      />
    </div>
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
  /* The inspector is a column rather than an overlay, so the user can keep
     mixing while it is open and watch a change land. */
  .view {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-4);
    align-items: start;
    min-width: 0;
  }
  .view.has-rail {
    grid-template-columns: minmax(0, 1fr) minmax(280px, 340px);
  }

  .view-main {
    min-width: 0;
  }

  .view-rail {
    position: sticky;
    top: 0;
    max-height: calc(100vh - var(--topbar-height) - var(--space-6));
    min-height: 0;
  }

  .app-section + .app-section {
    margin-top: var(--space-5);
  }

  .section-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--fs-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    margin-bottom: var(--space-3);
  }
  .section-count {
    font-size: var(--fs-2xs);
    font-weight: 700;
    padding: 1px 6px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .app-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    list-style: none;
    margin: 0;
    padding: 0;
  }

  /* Below this width the rail would crush the faders, so it stacks instead. */
  @media (max-width: 1100px) {
    .view.has-rail {
      grid-template-columns: minmax(0, 1fr);
    }
    .view-rail {
      position: static;
      max-height: none;
    }
  }
</style>
