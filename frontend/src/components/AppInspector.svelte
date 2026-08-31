<script lang="ts">
  /**
   * The per-application settings panel.
   *
   * This is what the previous version was missing entirely: a place to rename,
   * pin, hide, remember a level, assign a group, and forget an application. It
   * is a right-hand rail rather than a modal so the user can keep adjusting the
   * mixer while it is open and see the effect immediately.
   */
  import X from "@lucide/svelte/icons/x";
  import Pin from "@lucide/svelte/icons/pin";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import Bookmark from "@lucide/svelte/icons/bookmark";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import type { Application, ApplicationPatch, Group } from "../lib/api";
  import { formatPercent } from "../lib/volume";
  import AppIcon from "./AppIcon.svelte";
  import Select from "./Select.svelte";
  import { t } from "../lib/i18n/index";

  let {
    app,
    groups,
    onClose,
    onPatch,
    onForget,
  }: {
    app: Application;
    groups: Group[];
    onClose: () => void;
    onPatch: (patch: ApplicationPatch) => void;
    onForget: () => void;
  } = $props();

  let nameDraft = $state("");

  // Seeded from the app, and re-seeded when the panel switches to a different
  // one; otherwise the previous application's half-typed name would leak in.
  $effect(() => {
    void app.app_key;
    nameDraft = app.custom_name ?? "";
  });

  function commitName(): void {
    const trimmed = nameDraft.trim();
    const next = trimmed === "" ? null : trimmed;
    if (next !== app.custom_name) onPatch({ custom_name: next });
  }

  let groupOptions = $derived([
    { value: "", label: $t("common.none") },
    ...groups.map((group) => ({ value: group.id, label: group.name })),
  ]);

  let identityNote = $derived(
    app.identity_kind === "filename" ? $t("inspector.identityWeak") : null,
  );
</script>

<aside class="inspector" aria-label={$t("inspector.title", { app: app.display_name })}>
  <header class="inspector-head">
    <AppIcon
      src={app.icon}
      name={app.display_name}
      appKey={app.app_key}
      size={40}
      isSystem={app.is_system_sounds}
      dimmed={!app.running}
    />
    <div class="head-text">
      <h2 class="head-name truncate">{app.display_name}</h2>
      <p class="head-status">
        {#if app.running}
          <span class="status-dot is-live" aria-hidden="true"></span>
          {$t("app.sessionCount", { count: app.sessions.length })}
        {:else}
          <span class="status-dot" aria-hidden="true"></span>
          {$t("app.offline")}
        {/if}
      </p>
    </div>
    <button class="icon-btn" onclick={onClose} aria-label={$t("common.close")}>
      <X size={15} />
    </button>
  </header>

  <div class="inspector-body">
    <section class="block">
      <label class="field">
        <span class="field-label">{$t("inspector.name")}</span>
        <input
          type="text"
          bind:value={nameDraft}
          placeholder={app.executable_name ?? app.display_name}
          onblur={commitName}
          onkeydown={(event) => {
            if (event.key === "Enter") event.currentTarget.blur();
            if (event.key === "Escape") {
              nameDraft = app.custom_name ?? "";
              event.currentTarget.blur();
            }
          }}
        />
        <span class="field-hint">{$t("inspector.nameHint")}</span>
      </label>
    </section>

    <section class="block">
      <div class="setting-row">
        <div class="setting-copy">
          <div class="setting-label">
            <Bookmark size={13} aria-hidden="true" />
            {$t("inspector.remember")}
          </div>
          <div class="setting-hint">
            {$t("inspector.rememberHint", { level: formatPercent(app.volume) })}
          </div>
        </div>
        <label class="toggle">
          <input
            type="checkbox"
            checked={app.remembered}
            aria-label={$t("inspector.remember")}
            onchange={(event) => onPatch({ remembered: event.currentTarget.checked })}
          />
          <span class="toggle-slider"></span>
        </label>
      </div>

      <div class="setting-row">
        <div class="setting-copy">
          <div class="setting-label">
            <Pin size={13} aria-hidden="true" />
            {$t("inspector.pin")}
          </div>
          <div class="setting-hint">{$t("inspector.pinHint")}</div>
        </div>
        <label class="toggle">
          <input
            type="checkbox"
            checked={app.pinned}
            aria-label={$t("inspector.pin")}
            onchange={(event) => onPatch({ pinned: event.currentTarget.checked })}
          />
          <span class="toggle-slider"></span>
        </label>
      </div>

      <div class="setting-row">
        <div class="setting-copy">
          <div class="setting-label">
            <EyeOff size={13} aria-hidden="true" />
            {$t("inspector.hide")}
          </div>
          <div class="setting-hint">{$t("inspector.hideHint")}</div>
        </div>
        <label class="toggle">
          <input
            type="checkbox"
            checked={app.hidden}
            aria-label={$t("inspector.hide")}
            onchange={(event) => onPatch({ hidden: event.currentTarget.checked })}
          />
          <span class="toggle-slider"></span>
        </label>
      </div>
    </section>

    <section class="block">
      <div class="field">
        <span class="field-label">{$t("inspector.group")}</span>
        <Select
          value={app.group_id ?? ""}
          options={groupOptions}
          ariaLabel={$t("inspector.group")}
          onSelect={(next) => onPatch({ group_id: next === "" ? null : next })}
        />
        <span class="field-hint">{$t("inspector.groupHint")}</span>
      </div>
    </section>

    <section class="block">
      <h3 class="block-title">{$t("inspector.details")}</h3>
      <dl class="facts">
        {#if app.executable_name}
          <div class="fact">
            <dt>{$t("inspector.executable")}</dt>
            <dd class="mono truncate" title={app.executable_path ?? app.executable_name}>
              {app.executable_name}
            </dd>
          </div>
        {/if}
        <div class="fact">
          <dt>{$t("inspector.identity")}</dt>
          <dd class="mono">{$t("inspector.identity." + app.identity_kind)}</dd>
        </div>
        {#if app.running}
          <div class="fact">
            <dt>{$t("inspector.level")}</dt>
            <dd class="mono">
              {app.mixed ? $t("app.mixedShort") : formatPercent(app.volume)}
            </dd>
          </div>
        {/if}
      </dl>

      {#if identityNote}
        <p class="note">
          <TriangleAlert size={13} aria-hidden="true" />
          {identityNote}
        </p>
      {/if}
    </section>

    <section class="block block-danger">
      <h3 class="block-title">{$t("inspector.forgetTitle")}</h3>
      <p class="block-copy">{$t("inspector.forgetBody")}</p>
      <div class="danger-actions">
        {#if app.custom_name !== null || app.pinned || app.hidden || app.remembered}
          <button
            class="btn btn-sm"
            onclick={() =>
              onPatch({ custom_name: null, pinned: false, hidden: false, remembered: false })}
          >
            <RotateCcw size={13} />
            {$t("inspector.resetSettings")}
          </button>
        {/if}
        <button class="btn btn-sm btn-danger" onclick={onForget}>
          <Trash2 size={13} />
          {$t("inspector.forget")}
        </button>
      </div>
    </section>
  </div>
</aside>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    overflow: hidden;
  }

  .inspector-head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4);
    border-bottom: 1px solid var(--border);
    background: var(--bg-cap);
  }
  .head-text {
    flex: 1;
    min-width: 0;
  }
  .head-name {
    font-size: var(--fs-md);
    font-weight: 650;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
  }
  .head-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-muted);
    margin-top: 3px;
  }
  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-placeholder);
    flex-shrink: 0;
  }
  .status-dot.is-live {
    background: var(--success);
  }

  .inspector-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .block-title {
    font-size: var(--fs-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .block-copy {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    line-height: var(--lh-snug);
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
  .field-hint {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    line-height: var(--lh-snug);
  }

  .setting-row {
    padding: var(--space-2) 0;
  }
  .setting-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-sm);
  }

  .facts {
    display: grid;
    gap: var(--space-2);
    margin: 0;
  }
  .fact {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    min-width: 0;
  }
  .fact dt {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .fact dd {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    text-align: right;
    min-width: 0;
  }

  .note {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--warning);
    line-height: var(--lh-snug);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--warning-dim);
  }

  .block-danger {
    padding-top: var(--space-4);
    border-top: 1px solid var(--border);
  }
  .danger-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }
</style>
