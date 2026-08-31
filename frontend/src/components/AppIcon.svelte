<script lang="ts">
  /**
   * App icon with a deterministic lettered fallback.
   *
   * Icon extraction fails for packaged apps, protected processes and anything
   * whose executable path is unreadable, which is common enough that the
   * fallback is a first-class state rather than an error. The tint is derived
   * from the app key so the same application keeps the same colour forever.
   */
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import { initialFor, tintForKey } from "../lib/ux";

  let {
    src,
    name,
    appKey,
    size = 36,
    isSystem = false,
    dimmed = false,
  }: {
    src: string | null;
    name: string;
    appKey: string;
    size?: number;
    isSystem?: boolean;
    /** Desaturates the icon for an application that is not currently running. */
    dimmed?: boolean;
  } = $props();

  let failed = $state(false);
  let showImage = $derived(!!src && !failed);

  // A new source deserves a fresh attempt; otherwise one failure would poison
  // the slot for every later session that reuses this component instance.
  $effect(() => {
    void src;
    failed = false;
  });
</script>

{#if showImage}
  <img
    class="app-icon"
    class:is-dimmed={dimmed}
    style:--icon-size="{size}px"
    src={src}
    alt=""
    width={size}
    height={size}
    loading="lazy"
    decoding="async"
    onerror={() => (failed = true)}
  />
{:else}
  <span
    class="app-icon app-icon-fallback aura-badge"
    class:is-dimmed={dimmed}
    style:--icon-size="{size}px"
    data-tint={isSystem ? undefined : tintForKey(appKey)}
    aria-hidden="true"
  >
    {#if isSystem}
      <Volume2 size={Math.round(size * 0.5)} />
    {:else}
      <span class="app-icon-initial">{initialFor(name)}</span>
    {/if}
  </span>
{/if}

<style>
  .app-icon {
    width: var(--icon-size);
    height: var(--icon-size);
    flex-shrink: 0;
    border-radius: var(--radius-md);
    object-fit: contain;
  }

  .app-icon-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
  }

  /* An offline application stays listed so its settings remain reachable; the
     desaturated icon says "not running" without hiding the row. */
  .app-icon.is-dimmed {
    filter: grayscale(0.85);
    opacity: 0.75;
  }

  .app-icon-initial {
    font-size: calc(var(--icon-size) * 0.42);
    font-weight: 650;
    line-height: 1;
    letter-spacing: var(--letter-tight);
  }
</style>
