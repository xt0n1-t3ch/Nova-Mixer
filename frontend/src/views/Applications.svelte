<script lang="ts">
  /**
   * The mixer desk.
   *
   * The mixer desk.
   *
   * One toolbar row (title, scenes, view tools), the output row, then every
   * application as a channel row, banked by state: Pinned, Active, Saved.
   *
   * Rows, not vertical strips. Strips spent the whole window height on each
   * source and scrolled sideways past about seven; rows cost ~56px each, so a
   * normal window shows every source, and each fader is long enough to set
   * precisely. Every row shares `--grid-channel`, so all faders, readouts and
   * mute buttons stand in the same columns.
   *
   * One row is one *application*, not one Windows audio session. Windows hands
   * an application several sessions whenever it likes, which is why an earlier
   * version showed Discord twice. Sessions live behind a disclosure.
   */
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import Plus from "@lucide/svelte/icons/plus";
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
  import PageHeader from "../components/PageHeader.svelte";
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
      <!-- One toolbar row: the view, the scenes that act on it, and the tools
           that filter it. Scenes sit in the header because they act on the
           whole desk, not on any one strip. -->
      <PageHeader title={$t("view.applications.title")}>
        {#snippet actions()}
          <div class="head-scenes"><SceneBar /></div>
          <div class="head-tools">
            <div class="tools-filters" role="group" aria-label={$t("desk.channels")}>
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
              class="btn btn-sm btn-primary"
              onclick={() => void openAdd()}
              title={$t("view.applications.add")}
              aria-label={$t("view.applications.add")}
            >
              <Plus size={13} />
              <span class="add-label">{$t("view.applications.add")}</span>
            </button>
          </div>
        {/snippet}
      </PageHeader>

      <!-- The output leads, because it is the level every channel below feeds
           into; the channels follow as one list of rows on a shared grid. -->
      {#if $master}
        <MasterDeck
          master={$master}
          peak={$masterPeak}
          onInput={previewMasterVolume}
          onCommit={(value) => void commitMasterVolume(value)}
          onToggleMute={() => void toggleMasterMute()}
        />
      {/if}

      <div class="channels surface">
        {#if $sortedApplications.length === 0}
          <div class="desk-empty">
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
          </div>
        {:else}
          {#each $applicationSections as section (section.id)}
            <section class="bank" aria-labelledby="bank-{section.id}">
              <h2 class="bank-head" id="bank-{section.id}">
                <span class="panel-label">{$t("section." + section.id)}</span>
                <span class="bank-count mono">{section.apps.length}</span>
              </h2>
              <ul class="bank-list">
                {#each section.apps as app (app.app_key)}
                  <li
                    animate:flip={{ duration: motionDuration(200) }}
                    in:fly={{ y: 6, duration: motionDuration(200) }}
                    out:fly={{ y: -4, duration: motionDuration(130) }}
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
                      onTogglePin={() => void patchApplication(app.app_key, { pinned: !app.pinned })}
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
      </div>
    {/if}
  </div>
  {#if $inspectedApp}
    {@const target = $inspectedApp}
    <!-- Docked beside the desk when there is room; below that it overlays, so
         it never squeezes the channel list. -->
    <button
      class="inspector-scrim"
      aria-label={$t("common.close")}
      onclick={() => inspectedAppKey.set(null)}
    ></button>
    <aside class="inspector-slot" transition:fly={{ x: 24, duration: motionDuration(200) }}>
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
    column-gap: var(--space-4);
    height: 100%;
    min-height: 0;
  }

  .desk-main {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: 100%;
    min-width: 0;
    min-height: 0;
  }

  .head-tools {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .tools-filters {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }

  /* Scenes take the header's free width and give it back first. */
  .head-scenes {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    justify-content: flex-end;
  }
  .head-scenes :global(.scene-bar) {
    min-width: 0;
  }

  /* The channel list takes the rest of the height and scrolls on its own, so
     the output above never scrolls away. */
  .channels {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px var(--space-2) var(--space-2);
    scrollbar-gutter: stable;
  }

  .desk-empty {
    display: flex;
    justify-content: center;
    padding: var(--space-6) var(--space-5);
  }

  .bank + .bank {
    margin-top: 4px;
    padding-top: 4px;
    border-top: 1px solid var(--deck-line);
  }
  .bank-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px var(--space-4) 2px;
    margin: 0;
    font-size: inherit;
  }
  .bank-count {
    font-size: var(--fs-2xs);
    font-weight: 650;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }
  .bank-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  /* Docked, the inspector is a panel like its neighbours, not a slab glued to
     the frame edge. */
  .inspector-slot {
    min-height: 0;
    overflow: hidden;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .inspector-scrim {
    display: none;
  }

  /* Docked only when the channel list keeps enough width to stay useful. */
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
      width: min(var(--inspector-overlay-width), calc(100vw - 24px));
      z-index: 80;
      border-radius: 0;
      border-width: 0 0 0 1px;
      box-shadow: var(--shadow-lg);
    }
    .inspector-scrim {
      display: block;
      position: fixed;
      top: var(--chrome-height);
      left: 0;
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
      border-width: 1px 0 0;
      border-radius: var(--radius-xl) var(--radius-xl) 0 0;
    }
  }

  /* At the window's floor the add button keeps its icon and drops its label,
     so the header stays one row. */
  @media (max-width: 1000px) {
    .add-label {
      display: none;
    }
  }
</style>
