/**
 * Captures README showcase screenshots from the running NovaMixer app.
 *
 * Unlike `capture-preview.mjs`, which renders the design preview with seeded
 * data, this drives the real installed or debug app over the Chrome DevTools
 * Protocol, so the images show real applications, real icons and real Windows
 * meter levels. Start the app with
 * `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` first.
 *
 * Usage: node scripts/capture-showcase.mjs [outputDir]
 */
import fs from "node:fs";
import path from "node:path";

const PORT = Number(process.env.NOVAMIXER_CDP_PORT ?? 9333);
const outDir = path.resolve(process.argv[2] ?? "docs/media");
const WIDTH = 1180;
const HEIGHT = 760;

const targets = await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
const page = targets.find((t) => t.type === "page");
if (!page) throw new Error("No NovaMixer page on the CDP port");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});

let nextId = 0;
const pending = new Map();
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    pending.get(message.id)(message);
    pending.delete(message.id);
  }
};
function send(method, params = {}) {
  const id = ++nextId;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve) => pending.set(id, resolve));
}
async function evaluate(expression) {
  const result = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  return result.result?.result?.value;
}
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

await send("Emulation.setDeviceMetricsOverride", {
  width: WIDTH,
  height: HEIGHT,
  deviceScaleFactor: 2,
  mobile: false,
});

/** Clicks a top-bar tab by its test id and waits for the view to settle. */
async function openView(id) {
  await evaluate(`document.querySelector('[data-testid="nav-${id}"]')?.click()`);
  await wait(900);
}

/** Switches the theme through the app's own toggle when it differs. */
async function setTheme(theme) {
  const current = await evaluate(`document.documentElement.getAttribute("data-theme")`);
  if (current !== theme) {
    await evaluate(`document.querySelector('button[aria-label="Toggle light and dark theme"]')?.click()`);
    await wait(700);
  }
}

async function shot(name) {
  const result = await send("Page.captureScreenshot", { format: "png" });
  const file = path.join(outDir, `${name}.png`);
  fs.writeFileSync(file, Buffer.from(result.result.data, "base64"));
  console.log("captured", file);
}

fs.mkdirSync(outDir, { recursive: true });
const startTheme = await evaluate(`document.documentElement.getAttribute("data-theme")`);

await setTheme("dark");
await openView("applications");
await wait(1500);
await shot("showcase-mixer-dark");
await openView("groups");
await shot("showcase-groups");
await openView("settings");
await shot("showcase-settings");
await openView("about");
await shot("showcase-about");
await setTheme("light");
await openView("applications");
await wait(1500);
await shot("showcase-mixer-light");

// Leave the app as it was found.
await setTheme(startTheme ?? "dark");
await openView("applications");
await send("Emulation.clearDeviceMetricsOverride");
ws.close();
