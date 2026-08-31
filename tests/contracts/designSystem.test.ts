import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

// Vitest runs with the frontend package as its working directory.
const SRC = path.resolve(process.cwd(), "src");
const GLOBAL_CSS = `${SRC}/styles/global.css`;

function walk(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = `${dir}/${entry}`;
    if (statSync(full).isDirectory()) {
      walk(full, out);
    } else {
      out.push(full);
    }
  }
  return out;
}

const css = readFileSync(GLOBAL_CSS, "utf8");
const svelteFiles = walk(SRC).filter((file) => file.endsWith(".svelte"));

/**
 * The design system is only a system if the tokens are declared in one place and
 * every component resolves through them. These tests fail the build when a
 * component starts hard-coding values, which is how a light theme silently rots.
 */
describe("token declarations", () => {
  it("declares the shared scales exactly once, in :root", () => {
    for (const token of [
      "--space-1",
      "--space-8",
      "--radius-md",
      "--radius-full",
      "--fs-base",
      "--fs-display",
      "--dur-fast",
      "--dur-normal",
      "--ease",
      "--spring",
    ]) {
      const declarations = css.match(new RegExp(`^\\s*${token}:`, "gm")) ?? [];
      expect(declarations.length, `${token} should be declared once`).toBe(1);
    }
  });

  it("keeps the 4px spacing scale", () => {
    const expected: Record<string, string> = {
      "--space-1": "4px",
      "--space-2": "8px",
      "--space-3": "12px",
      "--space-4": "16px",
      "--space-5": "24px",
      "--space-6": "32px",
      "--space-7": "48px",
      "--space-8": "64px",
    };
    for (const [token, value] of Object.entries(expected)) {
      expect(css).toContain(`${token}: ${value};`);
    }
  });

  it("declares the mixer-specific tokens the meter and slider depend on", () => {
    for (const token of [
      "--meter-low",
      "--meter-mid",
      "--meter-hot",
      "--meter-idle",
      "--meter-hold",
      "--slider-track",
      "--slider-thumb",
      "--slider-hit-area",
      "--row-height-comfy",
      "--row-height-compact",
    ]) {
      expect(css, `${token} must exist`).toContain(`${token}:`);
    }
  });

  it("overrides every themed token in light mode", () => {
    const lightBlock = css.slice(css.indexOf('[data-theme="light"]'));
    for (const token of [
      "--bg-app",
      "--bg-card",
      "--text-primary",
      "--text-muted",
      "--accent",
      "--border",
      "--success",
      "--danger",
      "--meter-low",
      "--meter-hot",
      "--slider-track",
      "--glass-strong",
      "--shadow-md",
    ]) {
      expect(lightBlock, `light theme must override ${token}`).toContain(`${token}:`);
    }
  });

  it("switches row metrics through the density attribute", () => {
    expect(css).toContain('[data-density="compact"]');
    expect(css).toContain('[data-density="comfy"]');
  });
});

describe("shared primitives", () => {
  it("centralizes the controls components rely on", () => {
    for (const selector of [
      ".btn",
      ".btn.btn-primary",
      ".icon-btn",
      ".chip",
      ".card",
      ".surface",
      ".toggle",
      ".toggle-slider",
      ".setting-row",
      ".seg",
      ".pill",
      ".kbd",
      ".glass-dialog",
      ".dialog-close",
      ".edge-accent",
      ".sr-only",
    ]) {
      expect(css, `${selector} must be defined globally`).toContain(selector);
    }
  });

  it("provides one focus ring for every interactive element", () => {
    expect(css).toContain("button:focus-visible");
    expect(css).toContain("--shadow-ring");
  });

  it("honours reduced motion globally rather than per component", () => {
    expect(css).toContain("@media (prefers-reduced-motion: reduce)");
  });

  it("keeps high-contrast mode readable in the scrollbar", () => {
    expect(css).toContain("@media (forced-colors: active)");
  });
});

describe("component discipline", () => {
  it("uses no raw hex colour outside the token file", () => {
    // White and black are allowed inside a shadow or an overlay because they are
    // opacity carriers, not palette entries.
    const allowed = /#(fff|ffffff|000|000000)\b/i;
    for (const file of svelteFiles) {
      const style = readFileSync(file, "utf8").match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? "";
      const hexes = (style.match(/#[0-9a-f]{3,8}\b/gi) ?? []).filter(
        (hex) => !allowed.test(hex),
      );
      expect(hexes, `${file} should use tokens, not raw hex`).toEqual([]);
    }
  });

  it("uses no hard-coded transition duration in milliseconds", () => {
    for (const file of svelteFiles) {
      const style = readFileSync(file, "utf8").match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? "";
      const durations = style.match(/transition:[^;]*\b\d+m?s\b/g) ?? [];
      expect(durations, `${file} should use --dur-* tokens`).toEqual([]);
    }
  });

  it("does not redeclare a shared token inside a component", () => {
    const shared = ["--space-4", "--radius-md", "--fs-base", "--dur-normal", "--accent:"];
    for (const file of svelteFiles) {
      const style = readFileSync(file, "utf8").match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? "";
      for (const token of shared) {
        const declaration = token.endsWith(":") ? token : `${token}:`;
        expect(style.includes(declaration), `${file} must not redeclare ${token}`).toBe(false);
      }
    }
  });
});
