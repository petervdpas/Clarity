// Lookups and display helpers over the generated model types. The model
// itself comes from the Rust backend; nothing here redefines it.

import type { Entity } from "./generated/Entity";
import type { EnumDecl } from "./generated/EnumDecl";
import type { Field } from "./generated/Field";
import type { Preview } from "./generated/Preview";
import type { JsonValue } from "./generated/serde_json/JsonValue";

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

/** The text a table cell or list item shows for a value. */
export function displayValue(preview: Preview, field: Field, value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "";
  switch (field.type.kind) {
    case "bool":
      return value ? "Yes" : "No";
    case "enum": {
      const decl = findEnum(preview, field.type.name.name);
      const variant = decl?.variants.find((v) => v.name.name === value);
      return variant?.label ?? String(value);
    }
    case "list":
      return `${value} item(s)`;
    default:
      return String(value);
  }
}

export function isNumeric(field: Field): boolean {
  return ["integer", "decimal", "money", "id"].includes(field.type.kind);
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
