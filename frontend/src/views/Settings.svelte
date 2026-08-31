<script lang="ts">
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import type { AppSettings, Density, HotkeyAction, Theme } from "../lib/api";
  import { openDataFolder } from "../lib/api";
  import Dialog from "../components/Dialog.svelte";
  import EfficiencyToggle from "../components/EfficiencyToggle.svelte";
  import HotkeyCapture from "../components/HotkeyCapture.svelte";
  import Select from "../components/Select.svelte";
  import {
    applyHotkeys,
    flushSettings,
    persistSettings,
    pushToast,
    restoreSettingsBackup,
    settings,
  } from "../lib/stores";
  import { formatPercent } from "../lib/volume";
  import { locale, LOCALE_LABELS, LOCALES, setLocale, t, type Locale } from "../lib/i18n/index";

  let { onSetTheme, currentTheme }: { onSetTheme: (theme: Theme) => void; currentTheme: string } =
    $props();

  type Tab = "general" | "hotkeys" | "appearance" | "advanced";
  const TABS: Tab[] = ["general", "hotkeys", "appearance", "advanced"];
  let tab = $state<Tab>("general");

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
    if (await restoreSettingsBackup()) {
      pushToast($t("toast.backupRestored"), "success");
    }
    restoreOpen = false;
  }
</script>

<div class="view">
  <div class="view-header">
    <div>
      <h1 class="view-title">{$t("view.settings.title")}</h1>
      <p class="view-subtitle">{$t("view.settings.subtitle")}</p>
    </div>
    {#if $settings && !$settings.auto_save}
      <div class="header-actions">
        <button
          class="btn btn-primary"
          onclick={async () => {
            await flushSettings();
            pushToast($t("toast.settingsSaved"), "success");
          }}
        >
          {$t("common.save")}
        </button>
      </div>
    {/if}
  </div>

  <div class="seg tabs" role="tablist" aria-label={$t("view.settings.title")}>
    {#each TABS as id (id)}
      <button
        class="seg-btn"
        class:active={tab === id}
        role="tab"
        aria-selected={tab === id}
        onclick={() => (tab = id)}
      >
        {$t("settings.tab." + id)}
      </button>
    {/each}
  </div>

  {#if $settings}
    {@const config = $settings}
    <div class="tab-panel" role="tabpanel">
      {#if tab === "general"}
        <section class="surface">
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
        </section>
      {:else if tab === "hotkeys"}
        <section class="surface">
          <p class="section-sub target-hint">{$t("settings.hotkeyTargetHint")}</p>

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
      {:else if tab === "appearance"}
        <section class="surface">
          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.theme")}</div>
            </div>
            <div class="seg" role="group" aria-label={$t("settings.theme")}>
              <button
                class="seg-btn"
                class:active={currentTheme === "dark"}
                aria-pressed={currentTheme === "dark"}
                onclick={() => onSetTheme("dark")}
              >
                {$t("settings.themeDark")}
              </button>
              <button
                class="seg-btn"
                class:active={currentTheme === "light"}
                aria-pressed={currentTheme === "light"}
                onclick={() => onSetTheme("light")}
              >
                {$t("settings.themeLight")}
              </button>
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.density")}</div>
            </div>
            <div class="seg" role="group" aria-label={$t("settings.density")}>
              {#each ["comfy", "compact"] as const as density (density)}
                <button
                  class="seg-btn"
                  class:active={config.ui_prefs.density === density}
                  aria-pressed={config.ui_prefs.density === density}
                  onclick={() =>
                    patch({ ui_prefs: { ...config.ui_prefs, density: density as Density } })}
                >
                  {$t(density === "comfy" ? "settings.densityComfy" : "settings.densityCompact")}
                </button>
              {/each}
            </div>
          </div>

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
        <section class="surface">
          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.dataFolder")}</div>
              <div class="setting-hint">{$t("settings.dataFolderHint")}</div>
            </div>
            <button class="btn btn-sm" onclick={() => void openDataFolder()}>
              <FolderOpen size={13} />
              {$t("settings.openDataFolder")}
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-copy">
              <div class="setting-label">{$t("settings.restoreBackup")}</div>
              <div class="setting-hint">{$t("settings.restoreBackupHint")}</div>
            </div>
            <button class="btn btn-sm" onclick={() => (restoreOpen = true)}>
              <RotateCcw size={13} />
              {$t("settings.restoreBackup")}
            </button>
          </div>
        </section>
      {/if}
    </div>
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
      <button class="btn btn-ghost" onclick={() => (restoreOpen = false)}>{$t("common.cancel")}</button>
      <button class="btn btn-primary" onclick={() => void confirmRestore()}>
        {$t("settings.restoreBackup")}
      </button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .tabs {
    margin-bottom: var(--space-4);
  }
  .tab-panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 760px;
  }
  .target-hint {
    color: var(--text-muted);
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--border);
  }
  .control-slot {
    width: 170px;
    flex-shrink: 0;
  }
</style>
