/**
 * Renders the design preview to PNG for visual review.
 *
 * Starts Vite against `frontend/preview.html`, walks the view / theme matrix,
 * and writes one screenshot per combination. This is a review aid, not part of
 * the shipped application or of CI.
 *
 * Usage: node scripts/capture-preview.mjs [outputDir]
 */
import { spawn } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
// Playwright is a frontend dev dependency, so it is not resolvable from the
// repo root; import it through the frontend package.
import { chromium } from "../frontend/node_modules/@playwright/test/index.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const frontendDir = path.join(repoRoot, "frontend");
const outDir = path.resolve(repoRoot, process.argv[2] ?? ".preview");

const PORT = 1425;
const BASE = `http://localhost:${PORT}/preview.html`;
const VIEWPORT = { width: 1280, height: 800 };

const MATRIX = [
  { view: "applications", theme: "dark", lang: "en" },
  { view: "applications", theme: "light", lang: "en" },
  { view: "groups", theme: "dark", lang: "en" },
  { view: "groups", theme: "light", lang: "en" },
  { view: "settings", theme: "dark", lang: "en" },
  { view: "settings", theme: "light", lang: "en" },
  { view: "about", theme: "dark", lang: "en" },
  { view: "applications", theme: "dark", lang: "es" },
];

function startVite() {
  const command = process.platform === "win32" ? "npx.cmd" : "npx";
  const child = spawn(command, ["vite", "--port", String(PORT), "--strictPort"], {
    cwd: frontendDir,
    stdio: ["ignore", "pipe", "pipe"],
    shell: process.platform === "win32",
  });

  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Vite did not start in time")), 60_000);
    let settled = false;
    // Vite decorates its banner with ANSI colour codes, so the host and port are
    // not adjacent in the raw stream. Matching the port alone is reliable.
    const onData = (chunk) => {
      if (!settled && chunk.toString().includes(String(PORT))) {
        settled = true;
        clearTimeout(timer);
        resolve(child);
      }
    };
    child.stdout.on("data", onData);
    child.stderr.on("data", onData);
    child.on("exit", (code) => {
      if (settled) return;
      clearTimeout(timer);
      reject(new Error(`Vite exited with code ${code}`));
    });
  });
}

async function main() {
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });

  const vite = await startVite();
  const browser = await chromium.launch();
  const errors = [];

  try {
    const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: 2 });
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    page.on("pageerror", (error) => errors.push(error.message));

    for (const entry of MATRIX) {
      const query = new URLSearchParams(entry).toString();
      await page.goto(`${BASE}?${query}`, { waitUntil: "networkidle" });
      // Let fonts settle and the entry transitions finish before capturing.
      await page.waitForTimeout(700);

      const name = `${entry.view}-${entry.theme}${entry.lang === "es" ? "-es" : ""}.png`;
      await page.screenshot({ path: path.join(outDir, name) });
      process.stdout.write(`captured ${name}\n`);
    }
  } finally {
    await browser.close();
    vite.kill();
  }

  if (errors.length > 0) {
    process.stderr.write(`\nConsole errors during capture:\n${errors.join("\n")}\n`);
    process.exitCode = 1;
    return;
  }
  process.stdout.write(`\nNo console errors. Screenshots in ${outDir}\n`);
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exitCode = 1;
});
