/**
 * Preview entry point, loaded only by `preview.html`.
 *
 * Installs the IPC stub, then mounts the real `App` so the interface can be
 * reviewed and screenshotted in a plain browser while running the same code
 * path as the shipped application. `src/main.ts` is the real entry and never
 * imports anything from this directory.
 *
 * `?view=groups&theme=light&lang=es` drives the screenshot matrix.
 */
import { installPreviewBackend, startPreviewMetering } from "./mockBackend";

const params = new URLSearchParams(window.location.search);
const theme = params.get("theme") === "light" ? "light" : "dark";
const language = params.get("lang") === "es" ? "es" : "en";
const view = params.get("view") ?? "applications";

// The transport must exist before any Tauri module initializes, and the theme
// must be set before the first paint, so both happen ahead of the dynamic
// imports below.
installPreviewBackend({ theme, language });
document.documentElement.setAttribute("data-theme", theme);
localStorage.setItem("novamixer-theme", theme);

async function boot(): Promise<void> {
  const { mount } = await import("svelte");
  const { default: App } = await import("../App.svelte");
  await import("../styles/global.css");
  const { currentView } = await import("../lib/stores");
  const { isViewId } = await import("../lib/ux");

  if (isViewId(view)) currentView.set(view);

  mount(App, { target: document.getElementById("app")! });
  startPreviewMetering();
}

void boot();
