<script lang="ts">
  /**
   * Scenes: a whole set of levels applied at once.
   *
   * The capture button is the important half. Dialling levels by ear and then
   * saving what you hear is far easier than typing a percentage per application,
   * so creating a scene costs one click and a name.
   */
  import Camera from "@lucide/svelte/icons/camera";
  import Play from "@lucide/svelte/icons/play";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Layers from "@lucide/svelte/icons/layers";
  import Dialog from "./Dialog.svelte";
  import {
    captureCurrentAsScene,
    pushToast,
    removeScene,
    runScene,
    scenes,
  } from "../lib/stores";
  import { t } from "../lib/i18n/index";

  let captureOpen = $state(false);
  let nameDraft = $state("");
  let deleteTarget = $state<{ id: string; name: string } | null>(null);

  async function confirmCapture(): Promise<void> {
    const name = nameDraft.trim();
    if (!name) return;
    const created = await captureCurrentAsScene(name);
    if (created) pushToast($t("scene.captured", { name: created.name }), "success");
    captureOpen = false;
    nameDraft = "";
  }

  async function confirmDelete(): Promise<void> {
    const target = deleteTarget;
    if (!target) return;
    if (await removeScene(target.id)) {
      pushToast($t("scene.deleted", { name: target.name }), "success");
    }
    deleteTarget = null;
  }

  async function apply(id: string, name: string): Promise<void> {
    await runScene(id);
    pushToast($t("scene.applied", { name }), "success");
  }
</script>

<div class="scene-bar">
  <span class="scene-label">
    <Layers size={13} aria-hidden="true" />
    {$t("scene.label")}
  </span>

  {#if $scenes.length === 0}
    <span class="scene-hint">{$t("scene.emptyHint")}</span>
  {:else}
    <ul class="scene-list">
      {#each $scenes as scene (scene.id)}
        <li class="scene-chip">
          <button
            class="scene-apply"
            onclick={() => void apply(scene.id, scene.name)}
            title={$t("scene.applyTitle", { name: scene.name })}
          >
            <Play size={11} aria-hidden="true" />
            <span class="truncate">{scene.name}</span>
          </button>
          <button
            class="scene-delete"
            onclick={() => (deleteTarget = { id: scene.id, name: scene.name })}
            aria-label={$t("scene.deleteTitle", { name: scene.name })}
            title={$t("scene.deleteTitle", { name: scene.name })}
          >
            <Trash2 size={11} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <button
    class="btn btn-sm scene-capture"
    onclick={() => {
      nameDraft = "";
      captureOpen = true;
    }}
  >
    <Camera size={13} />
    {$t("scene.capture")}
  </button>
</div>

{#if captureOpen}
  <Dialog
    title={$t("scene.captureTitle")}
    description={$t("scene.captureBody")}
    width="400px"
    onClose={() => (captureOpen = false)}
  >
    <label class="field">
      <span class="field-label">{$t("scene.name")}</span>
      <input
        type="text"
        bind:value={nameDraft}
        placeholder={$t("scene.namePlaceholder")}
        onkeydown={(event) => {
          if (event.key === "Enter") void confirmCapture();
        }}
      />
    </label>
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (captureOpen = false)}>
        {$t("common.cancel")}
      </button>
      <button class="btn btn-primary" disabled={!nameDraft.trim()} onclick={() => void confirmCapture()}>
        {$t("scene.save")}
      </button>
    {/snippet}
  </Dialog>
{/if}

{#if deleteTarget}
  {@const target = deleteTarget}
  <Dialog
    title={$t("scene.deleteConfirmTitle", { name: target.name })}
    description={$t("scene.deleteConfirmBody")}
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

<style>
  /* A transport belt across the console, not a card: scenes are operational
     controls that sit between the master and the channels. */
  .scene-bar {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-3);
    min-height: var(--scene-belt-height);
    padding: 0 var(--space-4);
    background: var(--bg-cap);
    border-bottom: 1px solid var(--deck-line);
    overflow: hidden;
  }

  .scene-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-2xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .scene-hint {
    font-size: var(--fs-xs);
    color: var(--text-placeholder);
    flex: 1;
    min-width: 0;
  }

  /* Scenes overflow horizontally rather than wrapping into a second belt,
     which would push the channels down as the list grows. */
  .scene-list {
    display: flex;
    gap: var(--space-1);
    list-style: none;
    margin: 0;
    padding: 0;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-mask-image: linear-gradient(to right, transparent 0, #000 8px, #000 calc(100% - 8px), transparent 100%);
    mask-image: linear-gradient(to right, transparent 0, #000 8px, #000 calc(100% - 8px), transparent 100%);
  }
  .scene-list::-webkit-scrollbar { display: none; }

  /* Apply and delete are one visual pill but two targets, so a mis-click
     cannot destroy a scene the user meant to run. */
  .scene-chip {
    display: flex;
    align-items: center;
    height: 28px;
    border-radius: var(--radius-full);
    background: var(--bg-card);
    border: 1px solid var(--border);
    overflow: hidden;
    max-width: 200px;
    transition: border-color var(--dur-fast) var(--ease);
  }
  .scene-chip:hover {
    border-color: var(--border-hover);
  }

  .scene-apply {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 100%;
    padding: 0 8px 0 11px;
    font-size: var(--fs-xs);
    font-weight: 500;
    color: var(--text-secondary);
    min-width: 0;
  }
  .scene-apply:hover {
    color: var(--accent);
  }
  .scene-apply:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  .scene-delete {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 100%;
    color: var(--text-placeholder);
    border-left: 1px solid var(--border);
  }
  .scene-delete:hover {
    color: var(--danger);
    background: var(--danger-dim);
  }
  .scene-delete:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  .scene-capture {
    flex-shrink: 0;
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
</style>
