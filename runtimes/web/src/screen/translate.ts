// Texts by key in the renderer (design section 1.5, I1c). The project
// carries each language's value of every key, the default language's too;
// the renderer picks the language for the person's locale and looks keys up
// in it, taking the default's value where that language leaves one empty.
// A text of the design that is no key (`7`) shows as written. The rules
// are pure, so the shared cases in conformance/translations.json check them.


/** Every language's value of each key. */
export type Values = Record<string, Record<string, string>>;

/** The values to look keys up in: the language picked (null for the
 *  default) and the default language; and the app's version, which fills
 *  `{app.version}`. */
export type Texts = { values: Values; picked: string | null; fallback: string; version?: string | null };

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
  const value = of(texts.picked) ?? of(texts.fallback);
  // `{app.version}` is the project's version, which Clarity fills itself.
  return value !== undefined && texts.version ? value.replaceAll("{app.version}", texts.version) : value;
}

/** A text of the project as people read it: a key's value (`@saveCaption`),
 *  or the text as written. A key no language has shows as the key. */
export function translate(text: string, texts: Texts): string {
  if (!text.startsWith("@")) return text;
  const key = text.slice(1);
  return keyValue(key, texts) ?? key;
}

/** Languages named in themselves, as a running app names them (Clarity.UI's choices.rs language_name). */
const LANGUAGE_NAMES: Record<string, string> = {
  ar: "العربية",
  cs: "Čeština",
  da: "Dansk",
  de: "Deutsch",
  el: "Ελληνικά",
  en: "English",
  es: "Español",
  fi: "Suomi",
  fr: "Français",
  fy: "Frysk",
  he: "עברית",
  hi: "हिन्दी",
  hu: "Magyar",
  id: "Indonesia",
  it: "Italiano",
  ja: "日本語",
  ko: "한국어",
  nb: "Norsk bokmål",
  nl: "Nederlands",
  no: "Norsk",
  pl: "Polski",
  pt: "Português",
  ro: "Română",
  ru: "Русский",
  sk: "Slovenčina",
  sv: "Svenska",
  th: "ไทย",
  tr: "Türkçe",
  uk: "Українська",
  vi: "Tiếng Việt",
  zh: "中文",
  ca: "Català",
};

/** A language named in itself, as people look for it and as the running app shows it: `nl` is Nederlands; one not known is its tag. */
export function languageName(tag: string): string {
  return LANGUAGE_NAMES[tag.split(/[-_]/)[0].toLowerCase()] ?? tag;
}

