<script lang="ts">
  import Dialog from "./Dialog.svelte";
  import { shortcutOverlayOpen } from "../lib/stores";
  import { SHORTCUTS } from "../lib/ux";
  import { t } from "../lib/i18n/index";

  function keyLabel(key: string): string {
    return key === "esc" ? "Esc" : key.length === 1 ? key.toUpperCase() : key;
  }
</script>

{#if $shortcutOverlayOpen}
  <Dialog
    title={$t("shortcut.title")}
    width="440px"
    onClose={() => shortcutOverlayOpen.set(false)}
  >
    <dl class="shortcuts">
      {#each SHORTCUTS as shortcut (shortcut.descriptionKey)}
        <div class="shortcut">
          <dt>{$t(shortcut.descriptionKey)}</dt>
          <dd>
            {#each shortcut.keys as key, index (index)}
              {#if index > 0}<span class="sep" aria-hidden="true">then</span>{/if}
              <span class="kbd">{keyLabel(key)}</span>
            {/each}
          </dd>
        </div>
      {/each}
    </dl>
  </Dialog>
{/if}

<style>
  .shortcuts {
    display: grid;
    gap: var(--space-2);
    margin: 0;
  }
  .shortcut {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: 6px 0;
  }
  .shortcut + .shortcut {
    border-top: 1px solid var(--border);
  }
  .shortcut dt {
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }
  .shortcut dd {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .sep {
    font-size: var(--fs-2xs);
    color: var(--text-faint);
  }
</style>
