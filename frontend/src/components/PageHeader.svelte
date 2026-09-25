<script lang="ts">
  /**
   * The toolbar every view opens with: the view's name on the leading edge and
   * its actions on the trailing edge, in one row.
   *
   * Each view used to invent its own top: the mixer had none, Groups had a
   * large title with a paragraph, Settings put its title inside the section
   * index, and About used an identity card. One row makes the views read as one
   * application and gives the height back to the content.
   */
  import type { Snippet } from "svelte";

  let { title, actions }: { title: string; actions?: Snippet } = $props();
</script>

<header class="view-header">
  <h1 class="view-title">{title}</h1>
  {#if actions}
    <div class="header-actions">{@render actions()}</div>
  {/if}
</header>
