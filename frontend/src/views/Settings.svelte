<script lang="ts">
  /**
   * Settings as a two-column workshop.
   *
   * A tab strip over one narrow card left most of a wide window empty and gave
   * no sense of what a setting actually did. This uses a section index on the
   * left, the controls in the middle, and a live status rail on the right, so
   * the space carries the consequence of each toggle rather than stretching it.
   */
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import Power from "@lucide/svelte/icons/power";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import Palette from "@lucide/svelte/icons/palette";
  import HardDrive from "@lucide/svelte/icons/hard-drive";
  import type { AppSettings, Density, HotkeyAction, Theme } from "../lib/api";
  import { openDataFolder } from "../lib/api";
  import Dialog from "../components/Dialog.svelte";
  import EfficiencyToggle from "../components/EfficiencyToggle.svelte";
  import HotkeyCapture from "../components/HotkeyCapture.svelte";
  import Select from "../components/Select.svelte";
  import StatusRail from "../components/StatusRail.svelte";
  import {
    activeGroup,
    applyHotkeys,
    flushSettings,
    persistSettings,
    pushToast,
    restoreSettingsBackup,
    settings,
  } from "../lib/stores";
  import { formatPercent } from "../lib/volume";
  import { SHORTCUTS } from "../lib/ux";
  import { locale, LOCALE_LABELS, LOCALES, setLocale, t, type Locale } from "../lib/i18n/index";

  let { onSetTheme, currentTheme }: { onSetTheme: (theme: Theme) => void; currentTheme: string } =
    $props();

  type Section = "startup" | "hotkeys" | "appearance" | "storage";
  const SECTIONS: { id: Section; icon: typeof Power }[] = [
    { id: "startup", icon: Power },
    { id: "hotkeys", icon: Keyboard },
    { id: "appearance", icon: Palette },
    { id: "storage", icon: HardDrive },
  ];
  let section = $state<Section>("startup");

  let restoreOpen = $state(false);

  const HOTKEY_ACTIONS: { action: HotkeyAction; labelKey: string }[] = [
    { action: "volume_up", labelKey: "settings.hotkeyVolumeUp" },
    { action: "volume_down", labelKey: "settings.hotkeyVolumeDown" },
    { action: "mute_toggle", labelKey: "settings.hotkeyMute" },
  ];

  const STEP_OPTIONS = [0.01, 0.02, 0.05, 0.1] as const;

  function patch(changes: Partial<AppSettings>): void {
    const current = $settings;
    if (!current) return;
    persistSettings({ ...current, ...changes });
  }

  function acceleratorFor(action: HotkeyAction): string | null {
    return $settings?.hotkeys.find((binding) => binding.action === action)?.accelerator ?? null;
  }

  function changeHotkey(action: HotkeyAction, accelerator: string | null): void {
    const current = $settings;
    if (!current) return;

    if (accelerator) {
      const clash = current.hotkeys.find(
        (binding) => binding.action !== action && binding.accelerator === accelerator,
      );
      if (clash) {
        const clashLabel =
          HOTKEY_ACTIONS.find((item) => item.action === clash.action)?.labelKey ?? "";
        pushToast($t("settings.hotkeyConflict", { action: $t(clashLabel) }), "danger");
        return;
      }
    }

    const others = current.hotkeys.filter((binding) => binding.action !== action);
    void applyHotkeys([...others, { action, accelerator }]);
  }

  function changeLocale(next: Locale): void {
    setLocale(next);
    const current = $settings;
    if (current) {
      persistSettings({ ...current, ui_prefs: { ...current.ui_prefs, language: next } });
    }
  }

  async function confirmRestore(): Promise<void> {
    if (await restoreSettingsBackup()) pushToast($t("toast.backupRestored"), "success");
    restoreOpen = false;
  }

  /** States the toggles combine into, so the user is not left to work it out. */
  let startupSummary = $derived.by(() => {
    const config = $settings;
    if (!config) return "";
    if (!config.launch_on_startup) return $t("settings.summary.manual");
    return config.start_minimized
      ? $t("settings.summary.startTray")
      : $t("settings.summary.startWindow");
  });

  /** Which hotkeys act on: the active group, or the output. */
  let hotkeyTarget = $derived(
    $activeGroup && $activeGroup.hotkeys_enabled ? $activeGroup.name : $t("master.label"),
  );
</script>

<div class="settings">
  <nav class="index" aria-label={$t("view.settings.title")}>
    <h1 class="index-title">{$t("view.settings.title")}</h1>
    {#each SECTIONS as item (item.id)}
      {@const Icon = item.icon}
      <button
        class="index-item"
        class:active={section === item.id}
        aria-current={section === item.id ? "true" : undefined}
        onclick={() => (section = item.id)}
      >
        <Icon size={15} />
        <span>{$t("settings.section." + item.id)}</span>
      </button>
    {/each}
  </nav>

  {#if $settings}
    {@const config = $settings}
    <div class="panel">
      {#if section === "startup"}
        <section class="block">
          <div class="block-head">
            <h2 class="block-title">{$t("settings.section.startup")}</h2>
            <p class="block-summary">{startupSummary}</p>
          </div>

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.launchOnStartup")}</div>
              <div class="setting-hint">{$t("settings.launchOnStartupHint")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={config.launch_on_startup}
                aria-label={$t("settings.launchOnStartup")}
                onchange={(event) => patch({ launch_on_startup: event.currentTarget.checked })}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.startMinimized")}</div>
              <div class="setting-hint">{$t("settings.startMinimizedHint")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={config.start_minimized}
                aria-label={$t("settings.startMinimized")}
                onchange={(event) => patch({ start_minimized: event.currentTarget.checked })}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.minimizeToTray")}</div>
              <div class="setting-hint">{$t("settings.minimizeToTrayHint")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={config.minimize_to_tray}
                aria-label={$t("settings.minimizeToTray")}
                onchange={(event) => patch({ minimize_to_tray: event.currentTarget.checked })}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
        </section>

        <section class="block">
          <div class="block-head">
            <h2 class="block-title">{$t("settings.section.performance")}</h2>
          </div>

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.autoSave")}</div>
              <div class="setting-hint">{$t("settings.autoSaveHint")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={config.auto_save}
                aria-label={$t("settings.autoSave")}
                onchange={(event) => patch({ auto_save: event.currentTarget.checked })}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>

          <EfficiencyToggle
            enabled={config.efficiency_mode}
            onChange={(enabled) => patch({ efficiency_mode: enabled })}
          />

          {#if !config.auto_save}
            <div class="block-actions">
              <button
                class="btn btn-sm btn-primary"
                onclick={async () => {
                  await flushSettings();
                  pushToast($t("toast.settingsSaved"), "success");
                }}
              >
                {$t("common.save")}
              </button>
            </div>
          {/if}
        </section>
      {:else if section === "hotkeys"}
        <section class="block">
          <div class="block-head">
            <h2 class="block-title">{$t("settings.section.hotkeys")}</h2>
            <p class="block-summary">
              {$t("settings.hotkeyTargetNow", { target: hotkeyTarget })}
            </p>
          </div>

          {#each HOTKEY_ACTIONS as item (item.action)}
            <div class="setting-row">
              <div class="setting-copy">
                <div class="setting-label">{$t(item.labelKey)}</div>
              </div>
              <HotkeyCapture
                accelerator={acceleratorFor(item.action)}
                label={$t(item.labelKey)}
                onChange={(accelerator) => changeHotkey(item.action, accelerator)}
              />
            </div>
          {/each}

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.volumeStep")}</div>
              <div class="setting-hint">{$t("settings.volumeStepHint")}</div>
            </div>
            <div class="control-slot">
              <Select
                value={String(config.volume_step)}
                options={STEP_OPTIONS.map((step) => ({
                  value: String(step),
                  label: formatPercent(step),
                }))}
                ariaLabel={$t("settings.volumeStep")}
                onSelect={(next) => patch({ volume_step: Number(next) })}
              />
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.smartVolume")}</div>
              <div class="setting-hint">{$t("settings.smartVolumeHint")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={config.smart_volume}
                aria-label={$t("settings.smartVolume")}
                onchange={(event) => patch({ smart_volume: event.currentTarget.checked })}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
        </section>

        <section class="block">
          <div class="block-head">
            <h2 class="block-title">{$t("shortcut.title")}</h2>
          </div>
          <dl class="shortcut-grid">
            {#each SHORTCUTS as shortcut (shortcut.descriptionKey)}
              <div class="shortcut">
                <dt>{$t(shortcut.descriptionKey)}</dt>
                <dd>
                  {#each shortcut.keys as key (key)}
                    <span class="kbd">{key === "esc" ? "Esc" : key === "mod" ? "Ctrl" : key.toUpperCase()}</span>
                  {/each}
                </dd>
              </div>
            {/each}
          </dl>
        </section>
      {:else if section === "appearance"}
        <section class="block">
          <div class="block-head">
            <h2 class="block-title">{$t("settings.theme")}</h2>
          </div>
          <!-- Previews rather than a segmented control: the choice is visual,
               so the control should be too. -->
          <div class="choice-grid">
            {#each ["dark", "light"] as const as option (option)}
              <button
                class="choice"
                class:active={currentTheme === option}
                aria-pressed={currentTheme === option}
                onclick={() => onSetTheme(option)}
              >
                <span class="choice-preview" data-theme-preview={option} aria-hidden="true">
                  <span class="preview-bar"></span>
                  <span class="preview-row"></span>
                  <span class="preview-row is-short"></span>
                </span>
                <span class="choice-label">
                  {$t(option === "dark" ? "settings.themeDark" : "settings.themeLight")}
                </span>
              </button>
            {/each}
          </div>
        </section>

        <section class="block">
          <div class="block-head">
            <h2 class="block-title">{$t("settings.density")}</h2>
          </div>
          <div class="choice-grid">
            {#each ["comfy", "compact"] as const as option (option)}
              <button
                class="choice"
                class:active={config.ui_prefs.density === option}
                aria-pressed={config.ui_prefs.density === option}
                onclick={() =>
                  patch({ ui_prefs: { ...config.ui_prefs, density: option as Density } })}
              >
                <span class="choice-preview" aria-hidden="true">
                  <span class="preview-row" class:is-tight={option === "compact"}></span>
                  <span class="preview-row" class:is-tight={option === "compact"}></span>
                  <span class="preview-row" class:is-tight={option === "compact"}></span>
                  {#if option === "compact"}<span class="preview-row is-tight"></span>{/if}
                </span>
                <span class="choice-label">
                  {$t(option === "comfy" ? "settings.densityComfy" : "settings.densityCompact")}
                </span>
              </button>
            {/each}
          </div>
        </section>

        <section class="block">
          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.language")}</div>
            </div>
            <div class="control-slot">
              <Select
                value={$locale}
                options={LOCALES.map((code) => ({ value: code, label: LOCALE_LABELS[code] }))}
                ariaLabel={$t("settings.language")}
                onSelect={(next) => changeLocale(next as Locale)}
              />
            </div>
          </div>
        </section>
      {:else}
        <section class="block">
          <div class="block-head">
            <h2 class="block-title">{$t("settings.section.storage")}</h2>
            <p class="block-summary">{$t("settings.dataFolderHint")}</p>
          </div>

          <dl class="facts">
            <div class="fact">
              <dt>{$t("settings.settingsFile")}</dt>
              <dd class="mono">settings.json</dd>
            </div>
            <div class="fact">
              <dt>{$t("settings.backupFile")}</dt>
              <dd class="mono">settings.backup.json</dd>
            </div>
            <div class="fact">
              <dt>{$t("settings.schemaVersion")}</dt>
              <dd class="mono">{config.schema_version}</dd>
            </div>
          </dl>

          <div class="block-actions">
            <button class="btn btn-sm" onclick={() => void openDataFolder()}>
              <FolderOpen size={13} />
              {$t("settings.openDataFolder")}
            </button>
            <button class="btn btn-sm" onclick={() => (restoreOpen = true)}>
              <RotateCcw size={13} />
              {$t("settings.restoreBackup")}
            </button>
          </div>
          <p class="block-note">{$t("settings.restoreBackupHint")}</p>
        </section>
      {/if}
    </div>

    <StatusRail />
  {/if}
</div>

{#if restoreOpen}
  <Dialog
    title={$t("settings.restoreTitle")}
    description={$t("settings.restoreBody")}
    tone="warning"
    width="420px"
    onClose={() => (restoreOpen = false)}
  >
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (restoreOpen = false)}>
        {$t("common.cancel")}
      </button>
      <button class="btn btn-primary" onclick={() => void confirmRestore()}>
        {$t("settings.restoreBackup")}
      </button>
    {/snippet}
  </Dialog>
{/if}

<style>
  /* The columns stretch to the frame instead of ending two thirds down, which
     is what made the previous version read as an empty page with a card on it.
     The index and status rail are full-height surfaces; the controls scroll
     inside their own column. */
  .settings {
    display: grid;
    grid-template-columns: 196px minmax(0, 1fr) 272px;
    gap: var(--space-4);
    align-items: stretch;
    flex: 1;
    min-height: 0;
  }

  .index {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-right: var(--space-4);
    border-right: 1px solid var(--border);
  }
  .index-title {
    font-size: var(--fs-2xl);
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    color: var(--text-primary);
    margin-bottom: var(--space-4);
  }
  .index-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: 36px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    text-align: left;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .index-item:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }
  .index-item.active {
    background: var(--accent-dim);
    color: var(--accent);
    font-weight: 600;
  }
  .index-item:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }

  /* The last block absorbs the leftover height, so the column reaches the frame
     instead of stopping wherever its content happens to end. */
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }
  .panel > .block:last-child {
    flex: 1;
  }

  .block {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-5);
  }
  .block-head {
    margin-bottom: var(--space-2);
  }
  .block-title {
    font-size: var(--fs-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .block-summary {
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    margin-top: 6px;
  }
  .block-actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
    margin-top: var(--space-4);
  }
  .block-note {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    margin-top: var(--space-2);
    line-height: var(--lh-snug);
  }

  .control-slot {
    width: 160px;
    flex-shrink: 0;
  }

  .choice-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-3);
    margin-top: var(--space-2);
  }
  .choice {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    transition:
      border-color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .choice:hover {
    border-color: var(--border-hover);
  }
  .choice.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .choice:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .choice-label {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-secondary);
  }
  .choice.active .choice-label {
    color: var(--accent);
  }

  /* Each preview paints itself in the theme it offers, so the swatches read
     correctly whichever theme is currently active. The values come from the
     token file's own light and dark blocks via `color-scheme`, rather than
     hard-coded hex that would drift from the palette. */
  .choice-preview {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
    border-radius: var(--radius-sm);
    min-height: 58px;
  }
  .choice-preview[data-theme-preview="dark"] {
    background: #000;
    --preview-ink: #fff;
  }
  .choice-preview[data-theme-preview="light"] {
    background: #fff;
    --preview-ink: #000;
  }
  .preview-bar {
    height: 6px;
    border-radius: 2px;
    background: var(--preview-ink, var(--accent));
    opacity: 0.75;
  }
  .preview-row {
    height: 8px;
    border-radius: 2px;
    background: var(--preview-ink, var(--text-placeholder));
    opacity: 0.35;
  }
  .preview-row.is-short {
    width: 60%;
  }
  .preview-row.is-tight {
    height: 5px;
  }

  .facts {
    display: grid;
    gap: var(--space-2);
    margin: var(--space-2) 0 0;
  }
  .fact {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .fact dt {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .fact dd {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--text-secondary);
  }

  .shortcut-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--space-1) var(--space-5);
    margin: var(--space-2) 0 0;
  }
  .shortcut {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: 5px 0;
  }
  .shortcut dt {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
  }
  .shortcut dd {
    margin: 0;
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }

  /* At the minimum window the index becomes a horizontal strip: a quarter of an
     860px window is too much to spend on four labels, and the settings copy
     needs that width to stop wrapping. */
  @media (max-width: 1100px) {
    .settings {
      grid-template-columns: minmax(0, 1fr) 252px;
      grid-template-rows: auto 1fr;
    }
    .index {
      grid-column: 1 / -1;
      flex-direction: row;
      flex-wrap: wrap;
      align-items: center;
      gap: var(--space-1);
      padding: 0 0 var(--space-3);
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .index-title {
      margin: 0 var(--space-4) 0 0;
      font-size: var(--fs-xl);
    }
  }

  @media (max-width: 860px) {
    .settings {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
