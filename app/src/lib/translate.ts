// Translations in the renderer (design section 1.5, I1c). The project
// carries a table per language of each text as written and its
// translation; the renderer picks the table for the person's locale and
// looks texts up in it. The rules are pure, so the shared cases in
// conformance/translations.json check them.

import type { ShownText } from "./format";

/** One language's translations: each text as written, and how it reads. */
export type Table = Record<string, string>;

const primary = (tag: string) => tag.split("-")[0].toLowerCase();

/** The language whose translations to show for `locale`, among those the
 *  project has files for; null to show the texts as written. The exact
 *  tag wins, then the base language, then the language without its region,
 *  then any region of the same language. */
export function pickLanguage(locale: string, available: readonly string[], base: string | null): string | null {
  if (available.includes(locale)) return locale;
  const language = primary(locale);
  if (language === primary(base ?? "en")) return null;
  return available.find((a) => a === language) ?? available.find((a) => primary(a) === language) ?? null;
}

/** A text as the table gives it, or as written when it has no translation. */
export function translate(text: string, table: Table | undefined): string {
  return table?.[text] || text;
}

/** Text from the backend, translated by its text as written; the values
 *  of its slots stay as they are. */
export function translateText(text: ShownText, table: Table | undefined): ShownText {
  if (typeof text === "string") return translate(text, table);
  return { text: translate(text.text, table), values: text.values };
}
