<script lang="ts">
  import Plus from "@lucide/svelte/icons/plus";
  import Layers from "@lucide/svelte/icons/layers";
  import X from "@lucide/svelte/icons/x";
  import type { AppBinding, Group } from "../lib/api";
  import AppIcon from "../components/AppIcon.svelte";
  import AppPicker from "../components/AppPicker.svelte";
  import Dialog from "../components/Dialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import GroupCard from "../components/GroupCard.svelte";
  import Range from "../components/Range.svelte";
  import {
    activateGroup,
    activeGroupId,
    commitGroupVolume,
    groups,
    pushToast,
    removeGroup,
    saveGroup,
    selectedGroupId,
    sessions,
  } from "../lib/stores";
  import { formatPercent, volumeValueText } from "../lib/volume";
  import { t } from "../lib/i18n/index";

  let nameDialog = $state<{ mode: "create" | "rename"; value: string; groupId?: string } | null>(
    null,
  );
  let deleteTarget = $state<Group | null>(null);
  let pickerOpen = $state(false);

  let selected = $derived($groups.find((group) => group.id === $selectedGroupId) ?? $groups[0] ?? null);
  let canDelete = $derived($groups.length > 1);

  // Draft volume while dragging, so the slider is not fighting store updates.
  let draftVolume = $state<number | null>(null);
  let shownVolume = $derived(draftVolume ?? selected?.volume ?? 0);

  function newGroupId(): string {
    return crypto.randomUUID();
  }

  function openCreate(): void {
    nameDialog = { mode: "create", value: "" };
  }

  function openRename(group: Group): void {
    nameDialog = { mode: "rename", value: group.name, groupId: group.id };
  }

  async function confirmName(): Promise<void> {
    const dialog = nameDialog;
    if (!dialog) return;
    const name = dialog.value.trim();
    if (!name) return;

    if (dialog.mode === "create") {
      const created = await saveGroup({
        id: newGroupId(),
        name,
        is_default: $groups.length === 0,
        volume: 1,
        apps: [],
        startup_volume: null,
        auto_mute_on_launch: false,
        hotkeys_enabled: true,
      });
      if (created) {
        selectedGroupId.set(created.id);
        pushToast($t("toast.groupCreated", { group: created.name }), "success");
      }
    } else {
      const group = $groups.find((item) => item.id === dialog.groupId);
      if (group) await saveGroup({ ...group, name });
    }
    nameDialog = null;
  }

  async function confirmDelete(): Promise<void> {
    const group = deleteTarget;
    if (!group) return;
    if (await removeGroup(group.id)) {
      pushToast($t("toast.groupDeleted", { group: group.name }), "success");
    }
    deleteTarget = null;
  }

  async function patchSelected(patch: Partial<Group>): Promise<void> {
    if (!selected) return;
    await saveGroup({ ...selected, ...patch });
  }

  async function addApps(bindings: AppBinding[]): Promise<void> {
    if (!selected) return;
    await patchSelected({ apps: [...selected.apps, ...bindings] });
    pickerOpen = false;
  }

  async function removeApp(appKey: string): Promise<void> {
    if (!selected) return;
    await patchSelected({ apps: selected.apps.filter((app) => app.app_key !== appKey) });
  }
</script>

<div class="view">
  <div class="view-header">
    <div>
      <h1 class="view-title">{$t("view.groups.title")}</h1>
      <p class="view-subtitle">{$t("view.groups.subtitle")}</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-primary" onclick={openCreate}>
        <Plus size={14} />
        {$t("groups.new")}
      </button>
    </div>
  </div>

  {#if $groups.length === 0}
    <EmptyState icon={Layers} title={$t("groups.empty.title")} body={$t("groups.empty.body")}>
      {#snippet action()}
        <button class="btn btn-primary" onclick={openCreate}>
          <Plus size={14} />
          {$t("groups.new")}
        </button>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="groups-layout">
      <div class="groups-list">
        {#each $groups as group (group.id)}
          <GroupCard
            {group}
            isActive={group.id === $activeGroupId}
            isSelected={group.id === selected?.id}
            {canDelete}
            onSelect={() => {
              selectedGroupId.set(group.id);
              draftVolume = null;
            }}
            onRename={() => openRename(group)}
            onDelete={() => (deleteTarget = group)}
          />
        {/each}
      </div>

      {#if selected}
        {@const group = selected}
        <div class="group-detail">
          <section class="surface">
            <div class="detail-head">
              <div>
                <h2 class="section-heading">{group.name}</h2>
                <p class="section-sub">{$t("groups.apps", { count: group.apps.length })}</p>
              </div>
              {#if group.id !== $activeGroupId}
                <button class="btn btn-sm" onclick={() => void activateGroup(group.id)}>
                  {$t("groups.setActive")}
                </button>
              {/if}
            </div>

            <div class="detail-fader">
              <Range
                value={shownVolume}
                size="lg"
                ariaLabel={$t("groups.groupVolume", { group: group.name })}
                ariaValueText={volumeValueText(shownVolume, false, $t("mixer.muted"))}
                onInput={(value) => (draftVolume = value)}
                onCommit={(value) => {
                  draftVolume = null;
                  void commitGroupVolume(group.id, value);
                }}
              />
              <output class="detail-value mono" for="">{formatPercent(shownVolume)}</output>
            </div>
          </section>

          <section class="surface">
            <div class="detail-head">
              <h3 class="section-heading">{$t("groups.linkedApps")}</h3>
              <button class="btn btn-sm" onclick={() => (pickerOpen = true)}>
                <Plus size={13} />
                {$t("groups.addApp")}
              </button>
            </div>

            {#if group.apps.length === 0}
              <p class="section-sub empty-line">{$t("groups.noApps.body")}</p>
            {:else}
              <ul class="app-chips">
                {#each group.apps as app (app.app_key)}
                  <li class="app-chip">
                    <AppIcon src={null} name={app.display_name} appKey={app.app_key} size={22} />
                    <span class="truncate">{app.display_name}</span>
                    <button
                      class="icon-btn icon-btn-sm"
                      onclick={() => void removeApp(app.app_key)}
                      aria-label={$t("groups.removeApp", { app: app.display_name })}
                    >
                      <X size={12} />
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </section>

          <section class="surface">
            <h3 class="section-heading">{$t("view.groups.title")}</h3>

            <div class="setting-row">
              <div class="setting-copy">
                <div class="setting-label">{$t("groups.autoMute")}</div>
                <div class="setting-hint">{$t("groups.autoMuteHint")}</div>
              </div>
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={group.auto_mute_on_launch}
                  aria-label={$t("groups.autoMute")}
                  onchange={(event) =>
                    void patchSelected({ auto_mute_on_launch: event.currentTarget.checked })}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>

            <div class="setting-row">
              <div class="setting-copy">
                <div class="setting-label">{$t("groups.startupVolume")}</div>
                <div class="setting-hint">{$t("groups.startupVolumeHint")}</div>
              </div>
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={group.startup_volume !== null}
                  disabled={group.auto_mute_on_launch}
                  aria-label={$t("groups.startupVolume")}
                  onchange={(event) =>
                    void patchSelected({
                      startup_volume: event.currentTarget.checked ? group.volume : null,
                    })}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>

            {#if group.startup_volume !== null && !group.auto_mute_on_launch}
              <div class="detail-fader startup-fader">
                <Range
                  value={group.startup_volume}
                  ariaLabel={$t("groups.startupVolume")}
                  ariaValueText={volumeValueText(group.startup_volume, false, $t("mixer.muted"))}
                  onInput={() => {}}
                  onCommit={(value) => void patchSelected({ startup_volume: value })}
                />
                <output class="detail-value mono" for="">{formatPercent(group.startup_volume)}</output>
              </div>
            {/if}

            <div class="setting-row">
              <div class="setting-copy">
                <div class="setting-label">{$t("groups.hotkeysEnabled")}</div>
                <div class="setting-hint">{$t("groups.hotkeysHint")}</div>
              </div>
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={group.hotkeys_enabled}
                  aria-label={$t("groups.hotkeysEnabled")}
                  onchange={(event) =>
                    void patchSelected({ hotkeys_enabled: event.currentTarget.checked })}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </section>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if nameDialog}
  {@const dialog = nameDialog}
  <Dialog
    title={dialog.mode === "create" ? $t("groups.newTitle") : $t("groups.renameTitle")}
    onClose={() => (nameDialog = null)}
    width="400px"
  >
    <label class="field">
      <span class="field-label">{$t("groups.nameLabel")}</span>
      <input
        type="text"
        bind:value={dialog.value}
        placeholder={$t("groups.namePlaceholder")}
        onkeydown={(event) => {
          if (event.key === "Enter") void confirmName();
        }}
      />
    </label>
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (nameDialog = null)}>{$t("common.cancel")}</button>
      <button class="btn btn-primary" disabled={!dialog.value.trim()} onclick={() => void confirmName()}>
        {dialog.mode === "create" ? $t("common.add") : $t("common.save")}
      </button>
    {/snippet}
  </Dialog>
{/if}

{#if deleteTarget}
  {@const target = deleteTarget}
  <Dialog
    title={$t("groups.deleteTitle", { group: target.name })}
    description={$t("groups.deleteBody")}
    tone="danger"
    width="400px"
    onClose={() => (deleteTarget = null)}
  >
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (deleteTarget = null)}>{$t("common.cancel")}</button>
      <button class="btn btn-danger" onclick={() => void confirmDelete()}>{$t("common.delete")}</button>
    {/snippet}
  </Dialog>
{/if}

{#if pickerOpen && selected}
  <AppPicker
    sessions={$sessions}
    excludeKeys={selected.apps.map((app) => app.app_key)}
    onClose={() => (pickerOpen = false)}
    onConfirm={(bindings) => void addApps(bindings)}
  />
{/if}

<style>
  .groups-layout {
    display: grid;
    grid-template-columns: minmax(240px, 320px) minmax(0, 1fr);
    gap: var(--space-4);
    align-items: start;
  }

  .groups-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .group-detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }

  .detail-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .detail-head :global(.section-heading) {
    margin-bottom: 0;
  }

  .detail-fader {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  .detail-value {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    min-width: 46px;
    text-align: right;
  }
  .startup-fader {
    padding-bottom: var(--space-3);
  }

  .empty-line {
    color: var(--text-muted);
  }

  .app-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .app-chip {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px 4px 4px 8px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    max-width: 240px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field-label {
    font-size: var(--fs-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }

  @media (max-width: 1000px) {
    .groups-layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
