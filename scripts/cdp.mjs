/**
 * Development driver for the running NovaMixer webview.
 *
 * Debug builds expose a Chrome DevTools Protocol endpoint on port 9333
 * (see `src-tauri/src/lib.rs`). This script turns that endpoint into the verbs a
 * visual-acceptance loop needs: doctor, eval, script, html, shot and drag.
 *
 * A single socket is held open for the whole run, so a sequence such as
 * "press, move, release, screenshot" is one uninterrupted interaction instead of
 * four disconnected ones.
 *
 * Usage:
 *   node scripts/cdp.mjs doctor
 *   node scripts/cdp.mjs shot out.png
 *   node scripts/cdp.mjs eval "document.title"
 *   node scripts/cdp.mjs script probe.mjs
 *   node scripts/cdp.mjs drag <x0> <y0> <x1> <y1>
 *   node scripts/cdp.mjs html
 */
import fs from "node:fs";

const PORT = Number(process.env.NOVAMIXER_CDP_PORT ?? 9333);
const ENDPOINT = `http://127.0.0.1:${PORT}/json`;

async function getWsUrl() {
  let list;
  try {
    const res = await fetch(ENDPOINT);
    list = await res.json();
  } catch (cause) {
    throw new Error(
      `No CDP endpoint on port ${PORT}. Start a debug build with \`pnpm dev\` first. (${cause.message})`,
    );
  }
  const target =
    list.find((t) => t.type === "page" && !String(t.url).startsWith("devtools://")) ?? list[0];
  if (!target) throw new Error(`No CDP targets found on port ${PORT}`);
  return target.webSocketDebuggerUrl;
}

/** One socket, many commands. Callers must `close()` when finished. */
export async function connect() {
  const ws = new WebSocket(await getWsUrl());
  const pending = new Map();
  let seq = 0;

  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = () => reject(new Error("Cannot open CDP socket"));
  });

  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    const entry = pending.get(msg.id);
    if (!entry) return;
    pending.delete(msg.id);
    clearTimeout(entry.timer);
    if (msg.error) entry.reject(new Error(JSON.stringify(msg.error)));
    else entry.resolve(msg.result);
  };

  const send = (method, params = {}, timeoutMs = 15000) =>
    new Promise((resolve, reject) => {
      const id = ++seq;
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`Timeout waiting for CDP response (${method})`));
      }, timeoutMs);
      pending.set(id, { resolve, reject, timer });
      ws.send(JSON.stringify({ id, method, params }));
    });

  /** Evaluates in the page and awaits promises, so backend probes return data. */
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
      userGesture: true,
    });
    if (result.exceptionDetails) {
      const detail =
        result.exceptionDetails.exception?.description ?? result.exceptionDetails.text;
      throw new Error(`Page threw: ${detail}`);
    }
    return result.result?.value;
  };

  const mouse = (type, x, y, extra = {}) =>
    send("Input.dispatchMouseEvent", {
      type,
      x,
      y,
      button: "left",
      buttons: type === "mouseReleased" ? 0 : 1,
      clickCount: 1,
      pointerType: "mouse",
      ...extra,
    });

  /** Presses, moves in steps and releases, which is a real drag to the page. */
  const drag = async (x0, y0, x1, y1, steps = 12) => {
    await mouse("mousePressed", x0, y0);
    for (let i = 1; i <= steps; i += 1) {
      await mouse("mouseMoved", x0 + ((x1 - x0) * i) / steps, y0 + ((y1 - y0) * i) / steps);
      await new Promise((r) => setTimeout(r, 16));
    }
    await mouse("mouseReleased", x1, y1);
  };

  const shot = async (file) => {
    const result = await send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(file, Buffer.from(result.data, "base64"));
    return file;
  };

  return { send, evaluate, mouse, drag, shot, close: () => ws.close() };
}

const [action, ...args] = process.argv.slice(2);
const cdp = await connect();
const print = (value) =>
  console.log(typeof value === "string" ? value : JSON.stringify(value, null, 2));

try {
  switch (action) {
    case "doctor":
      print(
        await cdp.evaluate(
          `JSON.stringify({ url: location.href, width: innerWidth, height: innerHeight,
            theme: document.documentElement.dataset.theme,
            density: document.documentElement.dataset.density,
            tauri: typeof window.__TAURI_INTERNALS__ !== "undefined" })`,
        ),
      );
      break;
    case "shot":
      console.log(`Screenshot saved to ${await cdp.shot(args[0] ?? "novamixer-screenshot.png")}`);
      break;
    case "eval":
      print(await cdp.evaluate(args[0]));
      break;
    case "script": {
      // The async IIFE makes top-level `await` legal inside a probe file.
      const source = fs.readFileSync(args[0], "utf8");
      print(await cdp.evaluate(`(async () => { ${source} })()`));
      break;
    }
    case "drag":
      await cdp.drag(Number(args[0]), Number(args[1]), Number(args[2]), Number(args[3]));
      console.log("drag done");
      break;
    case "html":
      print(await cdp.evaluate("document.documentElement.outerHTML"));
      break;
    default:
      console.error("Usage: node scripts/cdp.mjs <doctor|shot|eval|script|drag|html> [args]");
      process.exitCode = 2;
  }
} finally {
  cdp.close();
}
