// Lookups and display helpers over the generated model types. The model
// itself comes from the Rust backend; nothing here redefines it.

import type { Entity } from "./generated/Entity";
import type { EnumDecl } from "./generated/EnumDecl";
import type { Field } from "./generated/Field";
import type { Preview } from "./generated/Preview";
import type { JsonValue } from "./generated/serde_json/JsonValue";
import { fillText, formatValue, userLocale, type FieldFormat, type Formats, type ShownText } from "./format";
import { languageChoice } from "./i18n.svelte";
import { keyValue, pickLanguage, translate, translateText, type Texts } from "./translate";
import { isDark, pickTheme } from "./theme";
import type { ThemeDecl } from "./generated/ThemeDecl";
import { shownList, type Item } from "./listFields";
import { scheme, themeChoice } from "./scheme.svelte";
import { keepsItsDigits } from "./windowData";
import { keyResource } from "./resources";

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

/** The language texts show in, as the backend is told it: the designer's
 *  choice, or the one for the person's locale; null for the default. */
export function languageOf(preview: Preview): string | null {
  const chosen = languageChoice.language;
  const all = Object.keys(preview.project.translations);
  return (chosen !== undefined ? chosen : pickLanguage(formatsOf(preview).locale, all, preview.project.locale.language)) ?? null;
}

/** The values to look keys up in, in the language texts show in. */
export function textsOf(preview: Preview): Texts {
  return { values: preview.project.translations, picked: languageOf(preview), fallback: preview.project.locale.language ?? "en" };
}

/** The text of Markdown file `name` of the project's resources. */
export function markdownOf(preview: Preview, name: string): string {
  return preview.project.resources?.markdown[name] ?? "";
}

/** The resource file a key (`@gameRules`) points at in the language texts show in, if it points at one. */
export function keyFile(preview: Preview, caption: string): string | undefined {
  const texts = textsOf(preview);
  return keyResource(preview.project.resource_keys, caption, texts.picked ?? texts.fallback, texts.fallback);
}

/** A text of the project as the person reads it: a key's value, or as written. */
export function tr(preview: Preview, text: string): string {
  return translate(text, textsOf(preview));
}

/** Clarity's own words by key, with how they read when the project gives them no value. */
const OWN_WORDS: Record<string, { key: string; written: string }> = {
  Yes: { key: "yes", written: "Yes" },
  No: { key: "no", written: "No" },
  OK: { key: "ok", written: "OK" },
  Cancel: { key: "cancel", written: "Cancel" },
  Dismiss: { key: "dismiss", written: "Dismiss" },
  Back: { key: "back", written: "Back" },
  Next: { key: "next", written: "Next" },
  Finish: { key: "finish", written: "Finish" },
  "No picture": { key: "noPicture", written: "No picture" },
  "Load a picture": { key: "loadPicture", written: "Load a picture" },
  "Remove the picture": { key: "removePicture", written: "Remove the picture" },
  // A file part's words.
  "No file": { key: "noFile", written: "No file" },
  "Choose a file": { key: "chooseFile", written: "Choose a file" },
  Open: { key: "openFile", written: "Open" },
  "Save a copy": { key: "saveFileCopy", written: "Save a copy" },
  "Remove the file": { key: "removeFile", written: "Remove the file" },
  "A copy is in your Downloads folder": { key: "fileCopySaved", written: "A copy is in your Downloads folder" },
  // The page bar of a table shown a page at a time.
  "First page": { key: "firstPage", written: "First page" },
  "Previous page": { key: "previousPage", written: "Previous page" },
  "Next page": { key: "nextPage", written: "Next page" },
  "Last page": { key: "lastPage", written: "Last page" },
  // A tree's toggle.
  Expand: { key: "expandRow", written: "Expand" },
  Collapse: { key: "collapseRow", written: "Collapse" },
  // A tags part's words.
  Add: { key: "addTag", written: "Add" },
  Remove: { key: "removeTag", written: "Remove" },
  // A password input's buttons.
  Show: { key: "showSecret", written: "Show" },
  Hide: { key: "hideSecret", written: "Hide" },
  Copy: { key: "copySecret", written: "Copy" },
  // The headings of the pickers of the click targets `language` and `theme`.
  Language: { key: "languagePicker", written: "Language" },
  Theme: { key: "themePicker", written: "Theme" },
};

/** One of Clarity's own words (`Yes`, `No`, a dialog's buttons, a picture's) in the language texts show in. */
export function ownWord(preview: Preview, word: string): string {
  const own = OWN_WORDS[word];
  return own ? (keyValue(own.key, textsOf(preview)) ?? own.written) : word;
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
      return { kind: "decimal", scale: keepsItsDigits(t) ? undefined : t.scale, format };
    default:
      return null;
  }
}

/** The text a table cell or list item shows for a value. */
export function displayValue(preview: Preview, field: Field, value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "";
  switch (field.type.kind) {
    case "bool":
      return ownWord(preview, value ? "Yes" : "No");
    case "enum": {
      const decl = findEnum(preview, field.type.name.name);
      const variant = decl?.variants.find((v) => v.name.name === value);
      return variant ? tr(preview, variant.label) : String(value);
    }
    case "list":
      return `${value} item(s)`;
    // A list of values: its items joined (conformance/list-fields.json).
    case "values":
      return shownList(Array.isArray(value) ? (value as Item[]) : [], (item) => itemText(preview, field, item));
    default: {
      const format = fieldFormat(field);
      if (format && (typeof value === "string" || typeof value === "number") && value !== "") {
        return formatValue(format, value, formatsOf(preview));
      }
      return String(value);
    }
  }
}

/** One item of a list field as people read it: a date in the person's format, an enum's value by its label. */
export function itemText(preview: Preview, field: Field, item: Item): string {
  if (field.type.kind !== "values") return String(item);
  const listed = field.type.item;
  if (listed.kind === "enum") return displayValue(preview, { ...field, type: { kind: "enum", name: listed.name } }, item);
  if (listed.kind === "date") return displayValue(preview, { ...field, type: { kind: "date" }, modifiers: { ...field.modifiers, format: null } }, item);
  return String(item);
}

/** The field types a table cell can edit, as the language allows them in `edit`. */
const CELL_EDIT_KINDS = ["text", "email", "url", "color", "integer", "decimal", "money", "bool", "date", "dateTime", "enum", "ref"];

export function cellEdits(field: Field): boolean {
  return CELL_EDIT_KINDS.includes(field.type.kind);
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
      return keepsItsDigits(t) ? "decimal" : `decimal(${t.precision}, ${t.scale})`;
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
  const texts = textsOf(preview);
  const label = (enumName: string, value: string) => {
    const variant = findEnum(preview, enumName)?.variants.find((v) => v.name.name === value);
    return variant ? translate(variant.label, texts) : value;
  };
  return fillText(translateText(text, texts), formatsOf(preview), label, (word) => ownWord(preview, word));
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
