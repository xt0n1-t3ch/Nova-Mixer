import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import en from "@/lib/i18n/locales/en.json";
import es from "@/lib/i18n/locales/es.json";
import { LOCALES, translate } from "@/lib/i18n/index";

// Vitest runs with the frontend package as its working directory.
const SRC = path.resolve(process.cwd(), "src");

function read(relative: string): string {
  return readFileSync(`${SRC}/${relative}`, "utf8");
}

describe("catalog parity", () => {
  it("declares the same keys in both locales", () => {
    const enKeys = Object.keys(en).sort();
    const esKeys = Object.keys(es).sort();

    expect(esKeys.filter((key) => !enKeys.includes(key))).toEqual([]);
    expect(enKeys.filter((key) => !esKeys.includes(key))).toEqual([]);
  });

  it("has no empty message in either locale", () => {
    for (const [locale, catalog] of [
      ["en", en],
      ["es", es],
    ] as const) {
      for (const [key, value] of Object.entries(catalog)) {
        expect(typeof value, `${locale}.${key} must be a string`).toBe("string");
        expect((value as string).trim(), `${locale}.${key} must not be empty`).not.toBe("");
      }
    }
  });

  it("uses the same placeholders in a translation as in the source", () => {
    const placeholders = (text: string): string[] =>
      [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();

    for (const [key, english] of Object.entries(en)) {
      const spanish = (es as Record<string, string>)[key];
      expect(placeholders(spanish), `placeholders differ for ${key}`).toEqual(
        placeholders(english as string),
      );
    }
  });

  it("provides both plural forms wherever one exists", () => {
    for (const catalog of [en, es]) {
      const keys = Object.keys(catalog);
      for (const key of keys) {
        if (key.endsWith("_one")) {
          expect(keys, `${key} needs a matching _other`).toContain(
            `${key.slice(0, -"_one".length)}_other`,
          );
        }
        if (key.endsWith("_other")) {
          expect(keys, `${key} needs a matching _one`).toContain(
            `${key.slice(0, -"_other".length)}_one`,
          );
        }
      }
    }
  });
});

describe("translate", () => {
  it("interpolates named variables", () => {
    expect(translate("en", "app.mute", { app: "Spotify" })).toBe("Mute Spotify");
    expect(translate("es", "app.mute", { app: "Spotify" })).toBe("Silenciar Spotify");
  });

  it("selects the plural form from the count", () => {
    expect(translate("en", "app.sessionCount", { count: 1 })).toBe("1 stream");
    expect(translate("en", "app.sessionCount", { count: 3 })).toBe("3 streams");
    expect(translate("es", "app.sessionCount", { count: 1 })).toBe("1 pista");
    expect(translate("es", "app.sessionCount", { count: 3 })).toBe("3 pistas");
  });

  it("returns the key itself when a message is missing, so gaps are visible", () => {
    expect(translate("en", "does.not.exist")).toBe("does.not.exist");
  });

  it("leaves an unknown placeholder visible rather than rendering a gap", () => {
    expect(translate("en", "app.mute", { wrong: "x" })).toContain("{app}");
  });
});

describe("source usage", () => {
  const sources = [
    "App.svelte",
    "components/AppRow.svelte",
    "components/MasterDeck.svelte",
    "components/StatusRail.svelte",
    "components/CommandRail.svelte",
    "components/ChromeBar.svelte",
    "components/Dialog.svelte",
    "components/AddAppDialog.svelte",
    "components/AppInspector.svelte",
    "components/SessionRow.svelte",
    "components/SceneBar.svelte",
    "components/CommandPalette.svelte",
    "components/EfficiencyToggle.svelte",
    "components/GroupCard.svelte",
    "components/HotkeyCapture.svelte",
    "views/Applications.svelte",
    "views/Groups.svelte",
    "views/Settings.svelte",
    "views/About.svelte",
  ];

  it("references only keys that exist in the English catalog", () => {
    const known = new Set(Object.keys(en));
    const pluralBases = new Set(
      Object.keys(en)
        .filter((key) => key.endsWith("_one") || key.endsWith("_other"))
        .map((key) => key.replace(/_(one|other)$/, "")),
    );

    for (const file of sources) {
      const text = read(file);
      // Skip `$t("nav." + id)`: the literal is a prefix, not a whole key, and
      // its completion is checked by the prefix test below.
      for (const match of text.matchAll(/\$t\(\s*"([a-z][\w.]*)"\s*[,)]/gi)) {
        const key = match[1];
        expect(known.has(key) || pluralBases.has(key), `${file} uses unknown key ${key}`).toBe(true);
      }
    }
  });

  it("resolves every key built by concatenating a prefix with an id", () => {
    // The sidebar and settings tabs compose keys from a view or tab id. Those
    // are invisible to a literal scan, so the composed results are listed here.
    const composed = [
      "nav.applications",
      "nav.groups",
      "nav.settings",
      "nav.about",
      "settings.tab.general",
      "settings.tab.hotkeys",
      "settings.tab.appearance",
      "settings.tab.advanced",
    ];
    for (const key of composed) {
      expect(Object.keys(en), `en is missing composed key ${key}`).toContain(key);
      expect(Object.keys(es), `es is missing composed key ${key}`).toContain(key);
    }
  });

  it("supports exactly the locales the settings UI offers", () => {
    expect([...LOCALES].sort()).toEqual(["en", "es"]);
  });
});
