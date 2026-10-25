// Texts by key in the renderer (design section 1.5, I1c). The project
// carries each language's value of every key, the default language's too;
// the renderer picks the language for the person's locale and looks keys up
// in it, taking the default's value where that language leaves one empty.
// A text of the design that is no key (`7`) shows as written. The rules
// are pure, so the shared cases in conformance/translations.json check them.

import type { ShownText } from "./format";

/** Every language's value of each key. */
export type Values = Record<string, Record<string, string>>;

/** The values to look keys up in: the language picked (null for the
 *  default) and the default language. */
export type Texts = { values: Values; picked: string | null; fallback: string };

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

/** The value of `key` in the language picked, or the default language's
 *  where that one leaves it empty; undefined when no language has it. */
export function keyValue(key: string, texts: Texts): string | undefined {
  const of = (language: string | null) => (language ? texts.values[language]?.[key] || undefined : undefined);
  return of(texts.picked) ?? of(texts.fallback);
}

/** A text of the project as people read it: a key's value (`@saveCaption`),
 *  or the text as written. A key no language has shows as the key. */
export function translate(text: string, texts: Texts): string {
  if (!text.startsWith("@")) return text;
  const key = text.slice(1);
  return keyValue(key, texts) ?? key;
}

/** Clarity's own words the backend sends by key, in English, as they read
 *  until the project gives them a value (translations UNCONNECTED_KEYS). */
const OWN_TEXTS: Record<string, string> = {
  unconnectedNotKept: "The database connection is not set yet",
  unconnectedFailed: "The database did not open: {problem}",
  // The keys window (translations KEYS_WINDOW_KEYS).
  keysTitle: "Keys",
  keysEverywhere: "Everywhere",
  keysChange: "Change",
  keysReset: "Reset",
  keysResetAll: "Reset all",
  keysClose: "Close",
  keysPress: "Press the new key; Escape stops",
  keysNone: "None",
  keysTaken: "{key} is the key of {command}. Swap them?",
  keysSwap: "Swap",
  keysAlso: "{key} is also the key of {command}, elsewhere; here {mine} goes first",
  keysTyping: "{key} cannot be used: a key without Ctrl or Alt is typing",
  keysTab: "{key} cannot be used: Tab moves between fields",
  keysEnter: "{key} cannot be used: Enter confirms",
  keysEscape: "{key} cannot be used: Escape cancels",
};

/** Text from the backend: a key takes its value, keeping its slot values;
 *  other text is as the backend wrote it. */
export function translateText(text: ShownText, texts: Texts): ShownText {
  if (typeof text === "string" || "text" in text) return text;
  const value = keyValue(text.key, texts) ?? OWN_TEXTS[text.key] ?? text.key.replaceAll("{", "{{").replaceAll("}", "}}");
  return { text: value, values: text.values ?? {} };
}

/** The languages an application can switch between: the one its texts are
 *  written in first, then those it has translations for. */
export function languagesOf(base: string | null, available: readonly string[]): string[] {
  const written = base ?? "en";
  return [written, ...available.filter((a) => a !== written).sort()];
}

/** A language named in itself, as people look for it: `nl` is Nederlands. */
export function languageName(tag: string): string {
  try {
    const name = new Intl.DisplayNames([tag], { type: "language" }).of(tag) ?? tag;
    return name.charAt(0).toLocaleUpperCase(tag) + name.slice(1);
  } catch {
    return tag;
  }
}

/** A language chosen in an earlier run, as the choice to show: null for
 *  the texts as written, undefined to follow the locale when the project
 *  no longer has that language. */
export function rememberedChoice(stored: string | null, base: string | null, available: readonly string[]): string | null | undefined {
  if (!stored) return undefined;
  if (stored === (base ?? "en")) return null;
  return available.includes(stored) ? stored : undefined;
}
