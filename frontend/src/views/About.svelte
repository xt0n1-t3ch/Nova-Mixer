<script lang="ts">
  import { onMount } from "svelte";
  import SlidersVertical from "@lucide/svelte/icons/sliders-vertical";
  import { PRODUCT } from "../generated/product";
  import { t } from "../lib/i18n/index";

  let version = $state("dev");
  let osVersion = $state<string | null>(null);

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

  /** Matches `[workspace.package].license` in the root Cargo.toml. */
  const LICENSE = "Apache-2.0";
</script>

<div class="view">
  <div class="view-header">
    <div>
      <h1 class="view-title">{$t("view.about.title")}</h1>
      <p class="view-subtitle">{$t("view.about.subtitle")}</p>
    </div>
  </div>

  <section class="surface hero">
    <span class="hero-mark" aria-hidden="true"><SlidersVertical size={26} /></span>
    <div class="hero-copy">
      <h2 class="hero-name">{$t("app.name")}</h2>
      <p class="hero-tagline">{$t("app.tagline")}</p>
      <p class="hero-author">{$t("about.author", { author: PRODUCT.author })}</p>
    </div>
    <span class="chip chip-accent mono">v{version}</span>
  </section>

  <section class="surface">
    <dl class="facts">
      <div class="fact">
        <dt>{$t("about.version")}</dt>
        <dd class="mono">{version}</dd>
      </div>
      <div class="fact">
        <dt>{$t("about.license")}</dt>
        <dd>{LICENSE}</dd>
      </div>
      <div class="fact">
        <dt>{$t("about.audioEngine")}</dt>
        <dd>{$t("about.audioEngineValue")}</dd>
      </div>
      {#if osVersion}
        <div class="fact">
          <dt>Windows</dt>
          <dd class="mono">{osVersion}</dd>
        </div>
      {/if}
    </dl>

    <div class="divider"></div>

    <p class="repo-line">
      <span class="repo-label">{$t("about.repository")}</span>
      <span class="mono repo-url">{PRODUCT.repository}</span>
    </p>
  </section>
</div>

<style>
  .view {
    max-width: 680px;
  }

  .hero {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }
  .hero-mark {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-xl);
    background: linear-gradient(145deg, var(--bg-input), var(--bg-elevated));
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow:
      inset 0 0 0 1px color-mix(in oklab, var(--text-primary) 24%, transparent),
      var(--shadow-sm);
  }
  .hero-copy {
    flex: 1;
    min-width: 0;
  }
  .hero-name {
    font-size: var(--fs-xl);
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    color: var(--text-primary);
  }
  .hero-tagline {
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    margin-top: 2px;
  }
  .hero-author {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    margin-top: 6px;
  }

  .facts {
    display: grid;
    gap: var(--space-3);
    margin: 0;
  }
  .fact {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .fact dt {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .fact dd {
    margin: 0;
    font-size: var(--fs-sm);
    color: var(--text-primary);
    text-align: right;
  }

  .repo-line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .repo-label {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .repo-url {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    overflow-wrap: anywhere;
    text-align: right;
  }
</style>
