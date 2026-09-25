<script lang="ts">
  /**
   * About: who made NovaMixer, what it does, and what it is doing now.
   *
   * A short facts list on a wide window left most of the viewport empty. This
   * pairs who NovaMixer is with what it is doing right now, which is what a
   * user actually checks here: is the audio engine connected, how many
   * applications am I managing, is efficiency mode really on.
   */
  import { onMount } from "svelte";
  import Check from "@lucide/svelte/icons/check";
  import { PRODUCT } from "../generated/product";
  import logo from "../assets/novamixer-logo.png";
  import PageHeader from "../components/PageHeader.svelte";
  import StatusRail from "../components/StatusRail.svelte";
  import { t } from "../lib/i18n/index";

  let version = $state("dev");
  let osVersion = $state<string | null>(null);

  /** Matches `[workspace.package].license` in the root Cargo.toml. */
  const LICENSE = "Apache-2.0";

  /** The author's handle, derived from the author URL in product.toml. */
  const authorHandle = "@" + PRODUCT.authorUrl.replace(/\/+$/, "").split("/").pop();

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

<PageHeader title={$t("view.about.title")} />

<div class="about">
  <div class="about-main">
    <!-- Who made it comes first and plainly: the product, its author, and the
         three facts a user checks — version, licence, source. -->
    <section class="hero surface">
      <img class="hero-logo" src={logo} alt="" width="72" height="72" />
      <div class="hero-copy">
        <h2 class="hero-name">{$t("app.name")}</h2>
        <p class="hero-tagline">{$t("app.tagline")}</p>
        <p class="hero-author">
          {$t("about.createdBy")}
          <strong>{PRODUCT.author}</strong>
          <span class="hero-handle mono">{authorHandle}</span>
        </p>
      </div>
      <dl class="hero-facts">
        <div>
          <dt>{$t("about.version")}</dt>
          <dd class="mono">v{version}</dd>
        </div>
        <div>
          <dt>{$t("about.license")}</dt>
          <dd class="mono">{LICENSE}</dd>
        </div>
      </dl>
    </section>

    <section class="surface">
      <h3 class="panel-label card-title">{$t("about.capabilities")}</h3>
      <ul class="capabilities">
        {#each CAPABILITIES as key (key)}
          <li>
            <Check size={13} aria-hidden="true" />
            {$t(key)}
          </li>
        {/each}
      </ul>
    </section>

    <section class="surface">
      <h3 class="panel-label card-title">{$t("about.links")}</h3>
      <dl class="facts">
        <div class="fact">
          <dt>{$t("about.author")}</dt>
          <dd class="mono selectable">{PRODUCT.authorUrl}</dd>
        </div>
        <div class="fact">
          <dt>{$t("about.repository")}</dt>
          <dd class="mono selectable">{PRODUCT.repository}</dd>
        </div>
        <div class="fact">
          <dt>{$t("about.releases")}</dt>
          <dd class="mono selectable">{PRODUCT.releases}</dd>
        </div>
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
      </dl>
    </section>
  </div>

  <StatusRail />
</div>

<style>
  /* Same silhouette as Groups and Settings, mirrored: the product reads first,
     and live status sits in the side column where a glance finds it. */
  .about {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--side-panel-width);
    gap: var(--space-4);
    align-items: start;
  }
  .about-main {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }

  .hero {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    padding: var(--space-5);
    flex-wrap: wrap;
  }
  .hero-logo {
    width: 72px;
    height: 72px;
    flex-shrink: 0;
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-md);
  }
  .hero-copy {
    flex: 1 1 240px;
    min-width: 0;
  }
  .hero-name {
    font-size: var(--fs-2xl);
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    line-height: var(--lh-tight);
    color: var(--text-primary);
  }
  .hero-tagline {
    margin-top: 4px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }
  .hero-author {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: var(--space-3);
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .hero-author strong {
    font-weight: 650;
    color: var(--text-primary);
  }
  .hero-handle {
    font-size: var(--fs-xs);
    color: var(--text-faint);
  }
  .hero-facts {
    display: flex;
    gap: var(--space-5);
    margin: 0;
  }
  .hero-facts dt {
    font-size: var(--fs-2xs);
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .hero-facts dd {
    margin: 4px 0 0;
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-primary);
  }

  .card-title {
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
    overflow-wrap: anywhere;
  }
  .fact dd.mono {
    font-size: var(--fs-xs);
  }
  /* Links are shown for copying: the app makes no network requests and opens
     no browser, so a URL is text the user can select. */
  .selectable {
    user-select: text;
  }

  @media (max-width: 960px) {
    .about {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>