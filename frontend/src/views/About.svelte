<script lang="ts">
  /**
   * About as product identity plus live runtime.
   *
   * A short facts list on a wide window left most of the viewport empty. This
   * pairs who NovaMixer is with what it is doing right now, which is what a
   * user actually checks here: is the audio engine connected, how many
   * applications am I managing, is efficiency mode really on.
   */
  import { onMount } from "svelte";
  import SlidersVertical from "@lucide/svelte/icons/sliders-vertical";
  import Check from "@lucide/svelte/icons/check";
  import { PRODUCT } from "../generated/product";
  import StatusRail from "../components/StatusRail.svelte";
  import { t } from "../lib/i18n/index";

  let version = $state("dev");
  let osVersion = $state<string | null>(null);

  /** Matches `[workspace.package].license` in the root Cargo.toml. */
  const LICENSE = "Apache-2.0";

  const CAPABILITIES = [
    "about.cap.policies",
    "about.cap.sessions",
    "about.cap.groups",
    "about.cap.scenes",
    "about.cap.hotkeys",
    "about.cap.tray",
    "about.cap.efficiency",
  ];

  onMount(async () => {
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      version = await getVersion();
    } catch {
      version = "dev";
    }
    try {
      const os = await import("@tauri-apps/plugin-os");
      osVersion = `${await os.type()} ${await os.version()}`;
    } catch {
      osVersion = null;
    }
  });
</script>

<div class="about">
  <div class="about-main">
    <section class="identity">
      <span class="identity-mark" aria-hidden="true">
        <SlidersVertical size={40} />
      </span>
      <div class="identity-copy">
        <h1 class="identity-name">{$t("app.name")}</h1>
        <p class="identity-tagline">{$t("app.tagline")}</p>
        <div class="identity-meta">
          <span class="chip chip-accent mono">v{version}</span>
          <span class="chip">{LICENSE}</span>
          <span class="identity-author">{$t("about.author", { author: PRODUCT.author })}</span>
        </div>
      </div>
    </section>

    <section class="card">
      <h2 class="card-title">{$t("about.capabilities")}</h2>
      <ul class="capabilities">
        {#each CAPABILITIES as key (key)}
          <li>
            <Check size={13} aria-hidden="true" />
            {$t(key)}
          </li>
        {/each}
      </ul>
    </section>

    <section class="card">
      <h2 class="card-title">{$t("about.runtime")}</h2>
      <dl class="facts">
        <div class="fact">
          <dt>{$t("about.audioEngine")}</dt>
          <dd>{$t("about.audioEngineValue")}</dd>
        </div>
        {#if osVersion}
          <div class="fact">
            <dt>{$t("about.windows")}</dt>
            <dd class="mono">{osVersion}</dd>
          </div>
        {/if}
        <div class="fact">
          <dt>{$t("about.repository")}</dt>
          <dd class="mono repo">{PRODUCT.repository}</dd>
        </div>
      </dl>
    </section>
  </div>

  <StatusRail />
</div>

<style>
  .about {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 272px;
    gap: var(--space-4);
    align-items: stretch;
    flex: 1;
    min-height: 0;
  }

  /* The capability list absorbs the leftover height: it is the block whose
     content breathes best when given more room, so the column reaches the frame
     without any card being padded out around empty space. */
  .about-main {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }
  .about-main > .card:first-of-type {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .about-main > .card:first-of-type .capabilities {
    flex: 1;
    align-content: space-evenly;
  }

  /* A large instrument mark rather than a small icon in a row: this is the one
     place the product gets to introduce itself. */
  .identity {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    padding: var(--space-6) var(--space-5);
    border-radius: var(--radius-2xl);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }
  .identity-mark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 88px;
    height: 88px;
    flex-shrink: 0;
    border-radius: var(--radius-2xl);
    background: linear-gradient(145deg, var(--bg-input), var(--bg-elevated));
    color: var(--text-primary);
    box-shadow:
      inset 0 0 0 1px color-mix(in oklab, var(--text-primary) 22%, transparent),
      var(--shadow-md);
  }
  .identity-copy {
    min-width: 0;
  }
  .identity-name {
    font-size: var(--fs-3xl);
    font-weight: 750;
    letter-spacing: var(--letter-tighter);
    line-height: 1.1;
    color: var(--text-primary);
  }
  .identity-tagline {
    font-size: var(--fs-md);
    color: var(--text-secondary);
    margin-top: 4px;
  }
  .identity-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
    margin-top: var(--space-4);
  }
  .identity-author {
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }

  .card {
    padding: var(--space-4) var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }
  .card-title {
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    margin-bottom: var(--space-3);
  }

  .capabilities {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: var(--space-2) var(--space-5);
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .capabilities li {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    line-height: var(--lh-snug);
  }
  .capabilities :global(svg) {
    color: var(--success);
    flex-shrink: 0;
    margin-top: 2px;
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
    gap: var(--space-4);
    min-width: 0;
  }
  .fact dt {
    font-size: var(--fs-sm);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .fact dd {
    margin: 0;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    text-align: right;
    min-width: 0;
  }
  .repo {
    font-size: var(--fs-xs);
    overflow-wrap: anywhere;
  }

  @media (max-width: 1000px) {
    .about {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  @media (max-width: 700px) {
    .identity {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-4);
      padding: var(--space-5) var(--space-4);
    }
  }
</style>
