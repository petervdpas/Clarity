// The lines of a theme the theme form writes, one per value, and the keys
// that name them (the spec's decl_lines): the form sends a key and its new
// line, or no line to take the value away so Clarity's own shows again.

import type { Look } from "@clarity/ui/generated/Look.ts";
import type { ThemeDecl } from "@clarity/ui/generated/ThemeDecl.ts";

/** One line of a theme set by its key; a null line takes it away. */
export type ThemeLine = { key: string; line: string | null };

/** Whether `text` is a color written out: `#rgb` or `#rrggbb`. */
export function isHex(text: string): boolean {
  return /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(text);
}

/** `#abc` as `#aabbcc`, which a color input takes; other text as it is. */
export function longHex(text: string): string {
  return /^#[0-9a-fA-F]{3}$/.test(text) ? "#" + [...text.slice(1)].map((c) => c + c).join("") : text;
}

/** A color in a look: a name of the theme as it is, a color written out in quotes. */
function colorWord(value: string): string {
  return isHex(value) ? JSON.stringify(value) : value;
}

/** What a look sets, as a theme writes it: `background raised, text text, weight bold`. */
export function lookText(look: Look): string {
  const parts: string[] = [];
  if (look.background) parts.push(`background ${colorWord(look.background)}`);
  if (look.text) parts.push(`text ${colorWord(look.text)}`);
  if (look.border) parts.push(`border ${colorWord(look.border)}`);
  if (look.radius !== null) parts.push(`radius ${look.radius}`);
  if (look.weight) parts.push(`weight ${look.weight}`);
  if (look.size !== null) parts.push(`size ${look.size}`);
  if (look.font) parts.push(`font ${look.font}`);
  if (look.align) parts.push(`align ${look.align}`);
  return parts.join(", ");
}

export const EMPTY_LOOK: Look = { background: null, text: null, border: null, radius: null, weight: null, size: null, font: null, align: null };

export function colorLine(name: string, light: string, dark: string | null): ThemeLine {
  const tail = dark && dark !== light ? ` dark ${JSON.stringify(dark)}` : "";
  return { key: `color ${name}`, line: `color ${name} ${JSON.stringify(light)}${tail}` };
}

/** A font by role; the text font is written without one. */
export function fontLine(role: string, value: string | null): ThemeLine {
  const key = `font ${role}`;
  if (!value?.trim()) return { key, line: null };
  return { key, line: role === "text" ? `font ${JSON.stringify(value.trim())}` : `font ${role} ${JSON.stringify(value.trim())}` };
}

/** A text size by role; the text size is written without one. */
export function sizeLine(role: string, value: number | null): ThemeLine {
  const key = `size ${role}`;
  if (value === null) return { key, line: null };
  return { key, line: role === "text" ? `size ${value}` : `size ${role} ${value}` };
}

/** A corner radius by kind of part; the radius of all is written without one. */
export function radiusLine(role: string, value: number | null): ThemeLine {
  const key = `radius ${role}`;
  if (value === null) return { key, line: null };
  return { key, line: role === "all" ? `radius ${value}` : `radius ${role} ${value}` };
}

/** `mode`, `padding`, `border`, `shadow`: one word and its value. */
export function settingLine(word: "mode" | "padding" | "border" | "shadow", value: string | number | null): ThemeLine {
  return { key: word, line: value === null || value === "" ? null : `${word} ${value}` };
}

export function titleLine(title: string | null): ThemeLine {
  return { key: "title", line: title?.trim() ? `title ${JSON.stringify(title.trim())}` : null };
}

export function partKey(kind: string, state: string | null): string {
  return state ? `part ${kind} ${state}` : `part ${kind}`;
}

/** A kind of part's look in a state; a look that sets nothing takes the line away. */
export function partLine(kind: string, state: string | null, look: Look): ThemeLine {
  const text = lookText(look);
  return { key: partKey(kind, state), line: text ? `${partKey(kind, state)} ${text}` : null };
}

/** A style; one that sets nothing is written with the look of none, so it stays. */
export function styleLine(name: string, look: Look): ThemeLine {
  const text = lookText(look);
  return { key: `style ${name}`, line: `style ${name} ${text || "border none"}` };
}

/** The look a theme gives a kind of part in a state; null where it gives none. */
export function partLook(theme: ThemeDecl, kind: string, state: string | null): Look | null {
  return theme.parts.find((p) => p.kind === kind && p.state === state)?.look ?? null;
}

/** The value a theme sets by role (fonts, sizes, radii); null where it sets none. */
export function roleValue(values: { role: string; value: string }[], role: string): string | null {
  return values.find((v) => v.role === role)?.value ?? null;
}

/** The colors a look can name: the roles, then the theme's own, then Clarity's own extra ones. */
export function colorNames(roles: string[], theme: ThemeDecl): string[] {
  const names = [...roles];
  for (const c of theme.colors) if (!names.includes(c.name.name)) names.push(c.name.name);
  return names;
}

/** The states a kind of part has: by default, then each state it can be in. */
export function statesOf(kind: string, states: string[], selectable: string[]): (string | null)[] {
  return [null, ...states.filter((s) => s !== "selected" || selectable.includes(kind))];
}
