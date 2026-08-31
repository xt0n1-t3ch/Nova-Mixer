<script lang="ts">
  /**
   * Groups: one volume across several applications, plus what happens when a
   * member launches.
   *
   * A group holds `app_key` strings now rather than its own copy of each
   * application's details — the application registry already owns those, and
   * duplicating them is how the two drifted apart in the previous version.
   */
  import Plus from "@lucide/svelte/icons/plus";
  import Layers from "@lucide/svelte/icons/layers";
  import X from "@lucide/svelte/icons/x";
  import Check from "@lucide/svelte/icons/check";
  import type { Group } from "../lib/api";
  import AppIcon from "../components/AppIcon.svelte";
  import Dialog from "../components/Dialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import GroupCard from "../components/GroupCard.svelte";
  import Range from "../components/Range.svelte";
  import {
    activateGroup,
    activeGroupId,
    applications,
    commitGroupVolume,
    groups,
    pushToast,
    removeGroup,
    saveGroup,
    selectedGroupId,
  } from "../lib/stores";
  import { formatPercent, volumeValueText } from "../lib/volume";
  import { matchesQuery } from "../lib/ux";
  import { t } from "../lib/i18n/index";

  let nameDialog = $state<{ mode: "create" | "rename"; value: string; groupId?: string } | null>(
    null,
  );
  let deleteTarget = $state<Group | null>(null);
  let memberPickerOpen = $state(false);
  let memberQuery = $state("");

  let selected = $derived(
    $groups.find((group) => group.id === $selectedGroupId) ?? $groups[0] ?? null,
  );
  let canDelete = $derived($groups.length > 1);

  /** Draft volume while dragging, so the slider is not fighting store updates. */
  let draftVolume = $state<number | null>(null);
  let shownVolume = $derived(draftVolume ?? selected?.volume ?? 0);

  let members = $derived(
    selected
      ? selected.app_keys
          .map((key) => $applications.find((app) => app.app_key === key))
          .filter((app): app is NonNullable<typeof app> => app !== undefined)
      : [],
  );

  let candidates = $derived(
    selected
      ? $applications
          .filter((app) => !selected.app_keys.includes(app.app_key) && !app.is_system_sounds)
          .filter((app) => matchesQuery(memberQuery, [app.display_name, app.executable_name]))
      : [],
  );

  function newGroupId(): string {
    return crypto.randomUUID();
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
        app_keys: [],
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

  async function addMember(appKey: string): Promise<void> {
    if (!selected) return;
    await patchSelected({ app_keys: [...selected.app_keys, appKey] });
  }

  async function removeMember(appKey: string): Promise<void> {
    if (!selected) return;
    await patchSelected({ app_keys: selected.app_keys.filter((key) => key !== appKey) });
  }
</script>

<div class="view">
  <div class="view-header">
    <div>
      <h1 class="view-title">{$t("view.groups.title")}</h1>
      <p class="view-subtitle">{$t("view.groups.subtitle")}</p>
    </div>
    <!-- Creating a group lives at the end of the bank, where the list makes it
         obvious what is being added to. -->
  </div>

  {#if $groups.length === 0}
    <EmptyState icon={Layers} title={$t("groups.empty.title")} body={$t("groups.empty.body")}>
      {#snippet action()}
        <button class="btn btn-primary" onclick={() => (nameDialog = { mode: "create", value: "" })}>
          <Plus size={14} />
          {$t("groups.new")}
        </button>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="groups-layout">
      <!-- A bank of groups, labelled like the mixer's own column legend, so
           this screen belongs to the same console as the desk. -->
      <div class="groups-bank">
        <div class="bank-head">
          <span class="bank-label">{$t("groups.bank")}</span>
          <span class="bank-rule" aria-hidden="true"></span>
          <span class="bank-count mono">{$groups.length}</span>
        </div>
        <div class="groups-list">
          {#each $groups as group (group.id)}
            <GroupCard
              {group}
              memberCount={group.app_keys.length}
              isActive={group.id === $activeGroupId}
              isSelected={group.id === selected?.id}
              {canDelete}
              onSelect={() => {
                selectedGroupId.set(group.id);
                draftVolume = null;
              }}
              onRename={() =>
                (nameDialog = { mode: "rename", value: group.name, groupId: group.id })}
              onDelete={() => (deleteTarget = group)}
            />
          {/each}
        </div>
        <button
          class="bank-add"
          onclick={() => (nameDialog = { mode: "create", value: "" })}
        >
          <Plus size={14} />
          {$t("groups.new")}
        </button>
      </div>

      {#if selected}
        {@const group = selected}
        <div class="group-detail">
          <section class="surface">
            <div class="detail-head">
              <div>
                <h2 class="section-heading">{group.name}</h2>
                <p class="section-sub">{$t("groups.apps", { count: group.app_keys.length })}</p>
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
                ariaValueText={volumeValueText(shownVolume, false, $t("app.muted"))}
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
              <h3 class="section-heading">{$t("groups.members")}</h3>
              <button
                class="btn btn-sm"
                onclick={() => {
                  memberQuery = "";
                  memberPickerOpen = true;
                }}
              >
                <Plus size={13} />
                {$t("groups.addMember")}
              </button>
            </div>

            {#if members.length === 0}
              <p class="section-sub empty-line">{$t("groups.noMembers")}</p>
            {:else}
              <ul class="member-chips">
                {#each members as app (app.app_key)}
                  <li class="member-chip">
                    <AppIcon
                      src={app.icon}
                      name={app.display_name}
                      appKey={app.app_key}
                      size={20}
                      dimmed={!app.running}
                    />
                    <span class="truncate">{app.display_name}</span>
                    <button
                      class="icon-btn icon-btn-sm"
                      onclick={() => void removeMember(app.app_key)}
                      aria-label={$t("groups.removeMember", { app: app.display_name })}
                    >
                      <X size={12} />
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </section>

          <section class="surface">
            <h3 class="section-heading">{$t("groups.behaviour", { group: group.name })}</h3>

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
                  ariaValueText={volumeValueText(group.startup_volume, false, $t("app.muted"))}
                  onInput={() => {}}
                  onCommit={(value) => void patchSelected({ startup_volume: value })}
                />
                <output class="detail-value mono" for="">
                  {formatPercent(group.startup_volume)}
                </output>
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
      <button class="btn btn-ghost" onclick={() => (nameDialog = null)}>
        {$t("common.cancel")}
      </button>
      <button
        class="btn btn-primary"
        disabled={!dialog.value.trim()}
        onclick={() => void confirmName()}
      >
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
      <button class="btn btn-ghost" onclick={() => (deleteTarget = null)}>
        {$t("common.cancel")}
      </button>
      <button class="btn btn-danger" onclick={() => void confirmDelete()}>
        {$t("common.delete")}
      </button>
    {/snippet}
  </Dialog>
{/if}

{#if memberPickerOpen && selected}
  <Dialog
    title={$t("groups.addMember")}
    description={$t("groups.addMemberHint")}
    width="440px"
    onClose={() => (memberPickerOpen = false)}
  >
    <input
      type="search"
      bind:value={memberQuery}
      placeholder={$t("groups.searchApps")}
      aria-label={$t("groups.searchApps")}
      class="member-search"
    />
    {#if candidates.length === 0}
      <p class="section-sub empty-line">{$t("groups.noCandidates")}</p>
    {:else}
      <ul class="candidate-list">
        {#each candidates as app (app.app_key)}
          <li>
            <button class="candidate" onclick={() => void addMember(app.app_key)}>
              <AppIcon
                src={app.icon}
                name={app.display_name}
                appKey={app.app_key}
                size={26}
                dimmed={!app.running}
              />
              <span class="truncate">{app.display_name}</span>
              <Check size={13} class="candidate-add" />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    {#snippet footer()}
      <button class="btn btn-primary" onclick={() => (memberPickerOpen = false)}>
        {$t("common.done")}
      </button>
    {/snippet}
  </Dialog>
{/if}

<style>
  /* The view is a flex column so the layout below it can claim the remaining
     height; without this the grid measures against an auto-sized parent and
     collapses to its content. */
  .view {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .groups-layout {
    display: grid;
    grid-template-columns: minmax(220px, 284px) minmax(0, 1fr);
    gap: var(--space-4);
    align-items: stretch;
    flex: 1;
    min-height: 0;
  }

  /* The bank fills its column and keeps its own header and footer, so a short
     list no longer leaves most of the left side blank. */
  .groups-bank {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-right: var(--space-4);
    border-right: 1px solid var(--border);
  }
  .bank-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .bank-label {
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .bank-rule {
    flex: 1;
    height: 1px;
    background: var(--border);
  }
  .bank-count {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--text-placeholder);
    font-variant-numeric: tabular-nums;
  }

  /* The add slot follows the last card immediately rather than being pushed to
     the bottom of the column, so a short list reads as a list with room after
     it instead of two clusters with a void between them. */
  .groups-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* A dashed slot at the end of the bank reads as "there is room for another
     one" far better than a button in a page header. */
  .bank-add {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    height: 44px;
    border-radius: var(--radius-lg);
    border: 1px dashed var(--border-strong);
    color: var(--text-muted);
    font-size: var(--fs-sm);
    font-weight: 500;
    transition:
      color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .bank-add:hover {
    color: var(--text-primary);
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .bank-add:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  .group-detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }
  /* The behaviour panel takes the remaining height so the detail column reaches
     the frame rather than ending two thirds down. */
  .group-detail > .surface:last-child {
    flex: 1;
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

  .member-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .member-chip {
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

  .member-search {
    margin-bottom: var(--space-3);
  }

  .candidate-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 320px;
    overflow-y: auto;
  }
  .candidate {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    text-align: left;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }
  .candidate:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }
  .candidate:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .candidate :global(.candidate-add) {
    margin-left: auto;
    color: var(--accent);
    opacity: 0;
    flex-shrink: 0;
  }
  .candidate:hover :global(.candidate-add),
  .candidate:focus-visible :global(.candidate-add) {
    opacity: 1;
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
