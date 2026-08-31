import { derived, writable, type Readable, type Writable } from "svelte/store";
import en from "./locales/en.json";
import es from "./locales/es.json";

export type Messages = typeof en;
export type Locale = "en" | "es";
export type TranslationVars = Record<string, string | number>;

export const LOCALES: readonly Locale[] = ["en", "es"];
export const DEFAULT_LOCALE: Locale = "en";
export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  es: "Español",
};

const CATALOGS: Record<Locale, Messages> = { en, es };

export const locale: Writable<Locale> = writable(DEFAULT_LOCALE);

export function isLocale(value: string | null | undefined): value is Locale {
  return value != null && (LOCALES as readonly string[]).includes(value);
}

export function localeFromNavigator(): Locale {
  const tag = typeof navigator === "undefined" ? "" : navigator.language.toLowerCase();
  return tag.startsWith("es") ? "es" : DEFAULT_LOCALE;
}

export function setLocale(next: Locale): void {
  locale.set(next);
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("lang", next);
  }
}

/** Replaces `{name}` placeholders, leaving an unknown name visible so a missing
 *  variable shows up in review instead of rendering as an empty gap. */
export function interpolate(template: string, vars?: TranslationVars): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_match, name: string) =>
    name in vars ? String(vars[name]) : `{${name}}`,
  );
}

function lookup(catalog: Messages, key: string): string | undefined {
  const value = (catalog as Record<string, unknown>)[key];
  return typeof value === "string" ? value : undefined;
}

/**
 * Resolves a dotted key against the active catalog.
 *
 * A key with a count variable first tries `key_one` / `key_other` using
 * `Intl.PluralRules`, so a translator controls plural shape per language
 * without any code change. Missing keys fall back to English, then to the key
 * itself — which is deliberately ugly, so an untranslated string is obvious.
 */
export function translate(
  activeLocale: Locale,
  key: string,
  vars?: TranslationVars,
): string {
  const catalog = CATALOGS[activeLocale];
  const fallback = CATALOGS[DEFAULT_LOCALE];

  if (vars && typeof vars.count === "number") {
    const category = new Intl.PluralRules(activeLocale).select(vars.count);
    const pluralKey = `${key}_${category}`;
    const plural = lookup(catalog, pluralKey) ?? lookup(fallback, pluralKey);
    if (plural !== undefined) return interpolate(plural, vars);
    const other = lookup(catalog, `${key}_other`) ?? lookup(fallback, `${key}_other`);
    if (other !== undefined) return interpolate(other, vars);
  }

  const message = lookup(catalog, key) ?? lookup(fallback, key);
  return message === undefined ? key : interpolate(message, vars);
}

export const t: Readable<(key: string, vars?: TranslationVars) => string> = derived(
  locale,
  ($locale) =>
    (key: string, vars?: TranslationVars): string =>
      translate($locale, key, vars),
);

/** Exposed for the parity test; not used at runtime. */
export const CATALOG_KEYS: Record<Locale, string[]> = {
  en: Object.keys(en),
  es: Object.keys(es),
};
