// A toast's declaration as lines (`toast saved` with `text`, `lifespan`,
// `level`, `icon`, `corner`): what the toast's form writes, one line per
// change (Edit setDeclLine), and the declaration + New toast adds. A value
// that is Clarity's default takes its line away.

import type { ToastCorner } from "@clarity/ui/generated/ToastCorner.ts";
import type { ToastLevel } from "@clarity/ui/generated/ToastLevel.ts";

/** One line to set (its key) or take away (`line` null). */
export type ToastLine = { key: "text" | "lifespan" | "level" | "icon" | "corner"; line: string | null };

export const TOAST_LEVELS: { value: ToastLevel; label: string }[] = [
  { value: "info", label: "Info" },
  { value: "success", label: "Success" },
  { value: "warning", label: "Warning" },
  { value: "error", label: "Error" },
];

export const TOAST_CORNERS: { value: ToastCorner; label: string }[] = [
  { value: "topLeft", label: "Top left" },
  { value: "topRight", label: "Top right" },
  { value: "bottomLeft", label: "Bottom left" },
  { value: "bottomRight", label: "Bottom right" },
];

/** How long a toast may show, in milliseconds (clarity_spec::code::TOAST_LIFESPANS). */
export const TOAST_LIFESPAN = { least: 500, most: 120000, default: 5000 } as const;

/** A toast's name: camelCase, as every lower-case name in Clarity. */
export function isToastName(name: string): boolean {
  return /^[a-z][A-Za-z0-9]*$/.test(name);
}

/** The text's key line: `text @savedToast`. */
export function textLine(key: string): ToastLine {
  return { key: "text", line: `text @${key.replace(/^@/, "").trim()}` };
}

/** The lifespan line, or why it is refused: a whole number of milliseconds Clarity can show. */
export function lifespanLine(ms: number): ToastLine | string {
  if (!Number.isInteger(ms) || ms < TOAST_LIFESPAN.least || ms > TOAST_LIFESPAN.most) {
    return `A toast shows from ${TOAST_LIFESPAN.least} to ${TOAST_LIFESPAN.most} milliseconds.`;
  }
  return { key: "lifespan", line: `lifespan ${ms}` };
}

export function levelLine(level: ToastLevel): ToastLine {
  return { key: "level", line: level === "info" ? null : `level ${level}` };
}

export function iconLine(icon: string | null): ToastLine {
  return { key: "icon", line: icon ? `icon "${icon}"` : null };
}

export function cornerLine(corner: ToastCorner): ToastLine {
  return { key: "corner", line: corner === "topRight" ? null : `corner ${corner}` };
}

/** The key a new toast's text gets: its name and `Toast`. */
export function textKeyOf(name: string): string {
  return `${name}Toast`;
}

/** A new toast's declaration, as + New toast adds it. */
export function newToastText(name: string): string {
  return `toast ${name}\n    text @${textKeyOf(name)}\n    lifespan ${TOAST_LIFESPAN.default}\n`;
}

/** The theme color of a level, as the screen draws the toast's edge. */
export function levelColor(level: ToastLevel): string {
  return level === "info" ? "var(--accent)" : `var(--${level})`;
}
