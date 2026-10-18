// Lookups and display helpers over the generated model types. The model
// itself comes from the Rust backend; nothing here redefines it.

import type { Entity } from "./generated/Entity";
import type { EnumDecl } from "./generated/EnumDecl";
import type { Field } from "./generated/Field";
import type { Preview } from "./generated/Preview";
import type { JsonValue } from "./generated/serde_json/JsonValue";
import { fillText, formatValue, userLocale, type FieldFormat, type Formats, type ShownText } from "./format";
import { languageChoice } from "./i18n.svelte";
import { pickLanguage, translate, translateText } from "./translate";
import { isDark, pickTheme } from "./theme";
import type { ThemeDecl } from "./generated/ThemeDecl";
import { scheme, themeChoice } from "./scheme.svelte";

export type Row = { [field: string]: JsonValue };

export function findEntity(preview: Preview, name: string): Entity | undefined {
  return preview.project.files.flatMap((f) => f.entities).find((e) => e.name.name === name);
}

export function findEnum(preview: Preview, name: string): EnumDecl | undefined {
  return preview.project.files.flatMap((f) => f.enums).find((e) => e.name.name === name);
}

export function findField(entity: Entity, name: string): Field | undefined {
  return entity.fields.find((f) => f.name.name === name);
}

/** Where values are shown: the locale of the person looking, and the
 *  project's currency. */
export function formatsOf(preview: Preview): Formats {
  const { language, currency } = preview.project.locale;
  // A language chosen in the designer shows its formats too.
  if (languageChoice.language) return { locale: languageChoice.language, currency };
  const system = typeof navigator === "undefined" ? undefined : navigator.languages;
  return { locale: userLocale(system, language), currency };
}

/** The translations to show texts in: the designer's choice, or those for
 *  the person's locale; undefined shows the texts as written. */
export function translationsOf(preview: Preview): Record<string, string> | undefined {
  const all = preview.project.translations;
  const chosen = languageChoice.language;
  const language =
    chosen !== undefined ? chosen : pickLanguage(formatsOf(preview).locale, Object.keys(all), preview.project.locale.language);
  return language ? all[language] : undefined;
}

/** A text of the project as the person reads it: translated, or as written. */
export function tr(preview: Preview, text: string): string {
  return translate(text, translationsOf(preview));
}

/** How a field's values are formatted, for the types that have formats. */
export function fieldFormat(field: Field): FieldFormat | null {
  const t = field.type;
  const format = field.modifiers.format;
  switch (t.kind) {
    case "integer":
    case "money":
    case "date":
    case "dateTime":
      return { kind: t.kind, format };
    case "duration":
      return { kind: "duration" };
    case "decimal":
      return { kind: "decimal", scale: t.scale, format };
    default:
      return null;
  }
}

/** The text a table cell or list item shows for a value. */
export function displayValue(preview: Preview, field: Field, value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "";
  switch (field.type.kind) {
    case "bool":
      return tr(preview, value ? "Yes" : "No");
    case "enum": {
      const decl = findEnum(preview, field.type.name.name);
      const variant = decl?.variants.find((v) => v.name.name === value);
      return variant ? tr(preview, variant.label) : String(value);
    }
    case "list":
      return `${value} item(s)`;
    default: {
      const format = fieldFormat(field);
      if (format && (typeof value === "string" || typeof value === "number") && value !== "") {
        return formatValue(format, value, formatsOf(preview));
      }
      return String(value);
    }
  }
}

export function isNumeric(field: Field): boolean {
  return ["integer", "decimal", "money", "id", "duration"].includes(field.type.kind);
}

/** Compares two cell values for sorting: numbers numerically, the rest as text. */
export function describeType(field: Field): string {
  const t = field.type;
  switch (t.kind) {
    case "text":
      return t.maxLength === null ? "text" : `text(${t.maxLength})`;
    case "decimal":
      return `decimal(${t.precision}, ${t.scale})`;
    case "dateTime":
      return "datetime";
    case "enum":
      return `enum ${t.name.name}`;
    case "ref":
      return `ref ${t.entity.name}`;
    case "list":
      return `list<${t.item.name}>`;
    default:
      return t.kind;
  }
}

/** Text from the backend as shown: its slots filled in the person's
 *  format, an enum value by its label. */
export function shownText(preview: Preview, text: ShownText): string {
  const table = translationsOf(preview);
  const label = (enumName: string, value: string) => {
    const variant = findEnum(preview, enumName)?.variants.find((v) => v.name.name === value);
    return variant ? translate(variant.label, table) : value;
  };
  return fillText(translateText(text, table), formatsOf(preview), label, (word) => translate(word, table));
}

/** What went wrong in the backend, as text to show: a refusal arrives as
 *  text with slots, anything else as a plain message. */
export function errorText(err: unknown): ShownText {
  if (typeof err === "object" && err !== null && "text" in err && "values" in err) return err as ShownText;
  return String(err);
}

/** The theme shown: the one chosen among those the application offers, or
 *  the one it starts with; null when the project file names none. */
export function themeOf(preview: Preview): ThemeDecl | null {
  const name = pickTheme(preview.project.themes, themeChoice.theme);
  return (name && preview.project.files.flatMap((f) => f.themes).find((t) => t.name.name === name)) || null;
}

/** The themes the application offers, as declared, in its order. */
export function offeredThemes(preview: Preview): ThemeDecl[] {
  const all = preview.project.files.flatMap((f) => f.themes);
  return preview.project.themes.map((name) => all.find((t) => t.name.name === name)).filter((t): t is ThemeDecl => !!t);
}

/** What puts a theme on an area (a running window, the designer's canvas):
 *  the theme's class, whose CSS the generator writes, and light or dark. */
export function themeArea(theme: ThemeDecl | null): { scheme: string; class: string } {
  // Without a project theme, Clarity's own follows the system.
  const dark = isDark(theme?.mode ?? "system", scheme.dark);
  return { scheme: dark ? "dark" : "light", class: ["clarity-app", theme?.class].filter(Boolean).join(" ") };
}

/** The style names of the project's theme, for the designer. */
export function themeStyles(preview: Preview): string[] {
  return themeOf(preview)?.styles.map((s) => s.name.name) ?? [];
}
