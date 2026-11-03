// Custom parts in the designer (DESIGN 5.19): what is typed for a property
// as the value written on the part's line, and how its tells read. A
// running custom part draws on Clarity.UI's canvas, not on this screen.

import type { Literal } from "../generated/Literal";
import type { Param } from "../generated/Param";
import type { TypeRefKind } from "../generated/TypeRefKind";

/** What is typed for a property of `type` as the value written on the part's line; null for none or one that does not fit. */
export function writtenValue(type: TypeRefKind, typed: string): Literal | null {
  const text = typed.trim();
  if (text === "") return null;
  switch (type.kind) {
    case "integer":
    case "duration":
      return /^-?\d+$/.test(text) ? { kind: "number", value: text } : null;
    case "decimal":
    case "money":
      return /^-?\d+(\.\d+)?$/.test(text) ? { kind: "number", value: text } : null;
    case "bool":
      return text === "true" || text === "false" ? { kind: "bool", value: text === "true" } : null;
    case "named":
      return /^[A-Za-z_]\w*$/.test(text) ? { kind: "ident", value: text } : null;
    default:
      return { kind: "str", value: text };
  }
}

/** A value written on the part's line, as the properties panel shows it. */
export function literalShown(value: Literal | undefined): string {
  if (!value) return "";
  return String(value.value);
}

/** The values a tell gives, as its handler takes them: `points: integer, by: text`. */
export function paramsText(params: Param[]): string {
  return params.map((p) => `${p.name.name}: ${typeText(p.ty.kind)}`).join(", ");
}

function typeText(type: TypeRefKind): string {
  switch (type.kind) {
    case "named":
      return type.name;
    case "list":
      return `list<${type.item}>`;
    case "dateTime":
      return "datetime";
    default:
      return type.kind;
  }
}
