<script lang="ts" generics="V extends string | null">
  /**
   * Listbox select. A native `<select>` cannot be styled to match the rest of
   * the design system on Windows, so this reimplements the listbox pattern with
   * full keyboard support rather than dropping it.
   */
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import Check from "@lucide/svelte/icons/check";

  type Option = { value: V; label: string; disabled?: boolean };

  let {
    value = $bindable(),
    options,
    placeholder = "",
    disabled = false,
    ariaLabel,
    onSelect,
  }: {
    value: V;
    options: Option[];
    placeholder?: string;
    disabled?: boolean;
    ariaLabel: string;
    /** Fires only on a user choice, so callers can persist without an effect
     *  that would also run for externally driven value changes. */
    onSelect?: (value: V) => void;
  } = $props();

  let open = $state(false);
  let activeIndex = $state(-1);
  let root = $state<HTMLDivElement>();

  let selected = $derived(options.find((option) => option.value === value) ?? null);

  function openMenu(): void {
    if (disabled) return;
    open = true;
    activeIndex = options.findIndex((option) => option.value === value && !option.disabled);
  }

  function close(): void {
    open = false;
    activeIndex = -1;
  }

  function choose(option: Option): void {
    if (option.disabled) return;
    value = option.value;
    close();
    onSelect?.(option.value);
  }

  function move(delta: number): void {
    const count = options.length;
    if (count === 0) return;
    let index = activeIndex;
    for (let step = 0; step < count; step++) {
      index = (index + delta + count) % count;
      if (!options[index].disabled) {
        activeIndex = index;
        break;
      }
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      close();
      return;
    }
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
        event.preventDefault();
        openMenu();
      }
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      move(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      move(-1);
    } else if (event.key === "Home") {
      event.preventDefault();
      activeIndex = -1;
      move(1);
    } else if (event.key === "End") {
      event.preventDefault();
      activeIndex = options.length;
      move(-1);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (activeIndex >= 0) choose(options[activeIndex]);
    }
  }

  $effect(() => {
    if (!open) return;
    function onPointerDown(event: PointerEvent): void {
      if (root && !root.contains(event.target as Node)) close();
    }
    document.addEventListener("pointerdown", onPointerDown, true);
    return () => document.removeEventListener("pointerdown", onPointerDown, true);
  });
</script>

<div class="sel" class:open bind:this={root}>
  <button
    type="button"
    class="sel-trigger"
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={ariaLabel}
    onclick={() => (open ? close() : openMenu())}
    onkeydown={onKeydown}
  >
    <span class="sel-value" class:is-placeholder={!selected}>
      {selected ? selected.label : placeholder}
    </span>
    <ChevronDown class="sel-chev" size={14} />
  </button>

  {#if open}
    <div class="sel-menu glass-dialog" role="listbox" aria-label={ariaLabel}>
      {#each options as option, index (String(option.value))}
        <button
          type="button"
          role="option"
          aria-selected={option.value === value}
          class="sel-opt"
          class:active={index === activeIndex}
          class:chosen={option.value === value}
          disabled={option.disabled}
          onclick={() => choose(option)}
          onpointerenter={() => !option.disabled && (activeIndex = index)}
        >
          <span class="truncate">{option.label}</span>
          {#if option.value === value}
            <Check size={13} />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    display: block;
    width: 100%;
    min-width: 0;
  }

  .sel-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: var(--fs-sm);
    transition:
      border-color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .sel-trigger:hover:not(:disabled) {
    border-color: var(--border-hover);
  }
  .sel.open .sel-trigger {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .sel-trigger:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .sel-trigger:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .sel-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sel-value.is-placeholder {
    color: var(--text-muted);
  }

  .sel :global(.sel-chev) {
    flex: 0 0 auto;
    color: var(--text-muted);
    transition: transform var(--dur-fast) var(--ease);
  }
  .sel.open :global(.sel-chev) {
    transform: rotate(180deg);
    color: var(--accent);
  }

  .sel-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 240;
    padding: 4px 4px 4px 6px;
    max-height: 280px;
    overflow-y: auto;
    border-radius: var(--radius-md);
  }

  .sel-opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 8px 9px;
    border-radius: var(--radius-sm);
    font-size: var(--fs-sm);
    text-align: left;
    color: var(--text-secondary);
  }
  .sel-opt.active {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }
  .sel-opt.chosen {
    color: var(--accent);
    font-weight: 600;
  }
  .sel-opt:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
