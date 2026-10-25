// Settings (DESIGN 5.17) apart from the screen (conformance/settings.json):
// which kept values a start takes back. A value is taken only when it still
// fits its setting; one that does not is dropped, and the setting keeps the
// default the backend gives it.

import type { Entity } from "./generated/Entity";
import type { Preview } from "./generated/Preview";

/** A setting as the cases describe it: its type, and what it allows. */
export type Setting = {
  type: string;
  min?: string | null;
  max?: string | null;
  /** An enum's values. */
  values?: string[];
  /** The project's languages, for `language`. */
  languages?: string[];
  /** The offered themes, for `theme`. */
  themes?: string[];
};

const NUMBER = /^-?\d+(\.\d+)?$/;
const DATE = /^\d{4}-\d{2}-\d{2}$/;
/** A color as it is kept (conformance/colors.json), or none. */
const KEPT_COLOR = /^(#[0-9a-f]{6})?$/;
const MOMENT = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?$/;

function within(n: number, setting: Setting): boolean {
  return (setting.min == null || n >= Number(setting.min)) && (setting.max == null || n <= Number(setting.max));
}

/** Whether a kept value fits its setting. */
export function fits(setting: Setting, value: unknown): boolean {
  switch (setting.type) {
    case "integer":
    case "duration":
      return typeof value === "number" && Number.isInteger(value) && within(value, setting);
    case "decimal":
    case "money":
      return typeof value === "string" && NUMBER.test(value) && within(Number(value), setting);
    case "bool":
      return typeof value === "boolean";
    case "date":
      return typeof value === "string" && DATE.test(value);
    case "datetime":
      return typeof value === "string" && MOMENT.test(value);
    case "enum":
      return typeof value === "string" && (setting.values ?? []).includes(value);
    case "language":
      return typeof value === "string" && (setting.languages ?? []).includes(value);
    case "theme":
      return typeof value === "string" && (setting.themes ?? []).includes(value);
    case "color":
      return typeof value === "string" && KEPT_COLOR.test(value);
    default:
      return typeof value === "string";
  }
}

/** The kept values a start takes back: only of settings there are, and only those that fit. */
export function keptSettings(settings: Record<string, Setting>, kept: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [name, setting] of Object.entries(settings)) {
    if (name in kept && fits(setting, kept[name])) out[name] = kept[name];
  }
  return out;
}

/** The project's settings record type, if it has one. */
export function settingsEntity(preview: Preview): Entity | undefined {
  return preview.project.files.flatMap((f) => f.entities).find((e) => e.settings);
}

/** The settings of a project as `fits` reads them. */
/** The settings kept in the system's protected store, by name (DESIGN 5.27): the shell keeps them out of the file. */
export function secretSettings(preview: Preview): string[] {
  return (settingsEntity(preview)?.fields ?? []).filter((f) => f.type.kind === "secret").map((f) => f.name.name);
}

export function settingsOf(preview: Preview, languages: string[], themes: string[]): Record<string, Setting> {
  const out: Record<string, Setting> = {};
  const enums = preview.project.files.flatMap((f) => f.enums);
  for (const field of settingsEntity(preview)?.fields ?? []) {
    const t = field.type;
    const bound = (l: typeof field.modifiers.min) => (l?.kind === "number" ? l.value : null);
    const setting: Setting = { type: t.kind === "dateTime" ? "datetime" : t.kind, min: bound(field.modifiers.min), max: bound(field.modifiers.max) };
    if (t.kind === "enum") setting.values = enums.find((e) => e.name.name === t.name.name)?.variants.map((v) => v.name.name) ?? [];
    if (t.kind === "language") setting.languages = languages;
    if (t.kind === "theme") setting.themes = themes;
    out[field.name.name] = setting;
  }
  return out;
}

/**
 * The settings before anything was kept, as every backend makes them: each
 * setting's `default`, else the empty value of its type (an enum's first
 * value); the language and the theme shown. A record has an id; the
 * settings' is 0.
 */
export function settingsDefaults(preview: Preview, language: string, theme: string): Record<string, unknown> {
  const out: Record<string, unknown> = { id: 0 };
  const enums = preview.project.files.flatMap((f) => f.enums);
  for (const field of settingsEntity(preview)?.fields ?? []) {
    const t = field.type;
    const given = field.modifiers.default;
    let value: unknown;
    switch (t.kind) {
      case "language":
        value = language;
        break;
      case "theme":
        value = theme;
        break;
      case "integer":
      case "duration":
        value = given?.kind === "number" ? Number(given.value) : 0;
        break;
      case "decimal":
      case "money":
        value = given?.kind === "number" ? given.value : "0";
        break;
      case "bool":
        value = given?.kind === "bool" ? given.value : false;
        break;
      case "enum":
        value = given?.kind === "ident" ? given.value : (enums.find((e) => e.name.name === t.name.name)?.variants[0]?.name.name ?? null);
        break;
      case "date":
      case "dateTime":
        value = given?.kind === "str" ? given.value : null;
        break;
      default:
        value = given?.kind === "str" ? given.value : "";
    }
    out[field.name.name] = value;
  }
  return out;
}
