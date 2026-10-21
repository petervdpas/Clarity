// What a window holds: records, lists of records and plain values
// (`data shown: decimal`). A part binds a plain value by its name, as a
// field named after it.

import type { DataDecl } from "./generated/DataDecl";
import type { Field } from "./generated/Field";
import type { FieldType } from "./generated/FieldType";
import type { Literal } from "./generated/Literal";
import type { TypeRefKind } from "./generated/TypeRefKind";
import type { JsonValue } from "./generated/serde_json/JsonValue";

/** The types a plain value can have besides an enum, as the language writes them. */
export const PLAIN_TYPES = ["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration"] as const;

/** The entity of a record or a list; null for a plain value. */
export function dataEntity(decl: DataDecl): string | null {
  return decl.of.kind === "value" ? null : decl.of.entity.name;
}

export function isList(decl: DataDecl): boolean {
  return decl.of.kind === "list";
}

/** A type as code writes it: `list<Line>`, `datetime`. */
export function typeText(ty: TypeRefKind): string {
  switch (ty.kind) {
    case "named":
      return ty.name;
    case "list":
      return `list<${ty.item}>`;
    case "dateTime":
      return "datetime";
    default:
      return ty.kind;
  }
}

function literalText(value: Literal): string {
  return value.kind === "str" ? JSON.stringify(value.value) : String(value.value);
}

/** The declaration as written after its name: `list<Order>`, `bool, default true`. */
export function dataType(decl: DataDecl): string {
  switch (decl.of.kind) {
    case "record":
      return decl.of.entity.name;
    case "list":
      return `list<${decl.of.entity.name}>`;
    case "value": {
      const ty = typeText(decl.of.ty.kind);
      return decl.of.default ? `${ty}, default ${literalText(decl.of.default)}` : ty;
    }
  }
}

/** The field type a part shows a value of `ty` as; null for a type no part shows. */
function shownType(ty: TypeRefKind, span: DataDecl["span"]): FieldType | null {
  switch (ty.kind) {
    case "text":
      return { kind: "text", maxLength: null };
    // A plain decimal keeps the digits it has. Precision 0 is no decimal
    // the language writes, so it says the value has no scale of its own.
    case "decimal":
      return { kind: "decimal", precision: 0, scale: 0 };
    case "named":
      return { kind: "enum", name: { name: ty.name, span } };
    case "file":
    case "bytes":
    case "list":
      return null;
    default:
      return { kind: ty.kind };
  }
}

/** Whether a decimal field is a plain value's, which keeps the digits it has. */
export function keepsItsDigits(type: FieldType): boolean {
  return type.kind === "decimal" && type.precision === 0;
}

/** The field a part shows a plain value as: named after it, of its type. */
export function valueField(decl: DataDecl): Field | null {
  if (decl.of.kind !== "value") return null;
  const type = shownType(decl.of.ty.kind, decl.span);
  if (!type) return null;
  const modifiers = {
    required: false,
    unique: false,
    owned: false,
    computed: false,
    min: null,
    max: null,
    pattern: null,
    default: decl.of.default,
    label: null,
    help: null,
    format: null,
    column: null,
    store: null,
  };
  return { name: decl.name, type, typeText: typeText(decl.of.ty.kind), modifiers, label: decl.name.name, span: decl.span };
}

/** The value a plain value starts at: its default, or empty. `firstValue`
 *  is the first value of its enum, for an enum without a default. */
export function startValue(decl: DataDecl, firstValue: string | null): JsonValue {
  if (decl.of.kind !== "value") return null;
  const ty = decl.of.ty.kind.kind;
  const given = decl.of.default;
  if (given) {
    // Integers travel as numbers; decimals and money as text, exactly.
    if (given.kind === "number") return ty === "integer" || ty === "duration" ? Number(given.value) : given.value;
    return given.value;
  }
  switch (ty) {
    case "text":
      return "";
    case "integer":
    case "duration":
      return 0;
    case "decimal":
    case "money":
      return "0";
    case "bool":
      return false;
    case "named":
      return firstValue;
    default:
      return null;
  }
}

/**
 * A window's data as it opens, with each text value that is still at its
 * default in the person's language (`tr`): a default is a text of the
 * project, translated as a label is.
 */
export function translatedDefaults(decls: DataDecl[], data: Record<string, unknown>, tr: (text: string) => string): Record<string, unknown> {
  const out = { ...data };
  for (const decl of decls) {
    const start = decl.of.kind === "value" ? decl.of.default : null;
    if (start?.kind === "str" && out[decl.name.name] === start.value) out[decl.name.name] = tr(start.value);
  }
  return out;
}
