// What several kinds of part share: the choices of a field (a radio group, a
// dropdown, a table's cells), a table's columns, what a table, tree or chart
// asks the runtime to fill in (DESIGN 1.7), and moving rows (DESIGN 5.30).

import type { CellKind, TableColumn } from "../parts/Table.svelte";
import type { Move } from "../parts/rowDrag.ts";
import type { Field } from "../generated/Field.ts";
import { displayValue, fieldFormat, findEnum, findField, formatsOf, offeredThemes, tr } from "../screen/model.ts";
import { numberEditing } from "../screen/format.ts";
import { languageName, languagesOf } from "../screen/translate.ts";
import { wiredCall } from "../screen/calls.ts";
import type { PartContext } from "./context.ts";

/** What a cell of a field's type holds. */
export function cellKind(kind: string): CellKind {
  switch (kind) {
    case "integer":
    case "decimal":
    case "money":
      return "number";
    case "bool":
      return "bool";
    case "enum":
    case "ref":
      return "choice";
    case "date":
      return "date";
    case "dateTime":
      return "moment";
    case "color":
      return "color";
    default:
      return "text";
  }
}

/** An enum's values (never empty), or the records a reference can point at after an empty choice. */
export function choicesOf(ctx: PartContext, field: Field): NonNullable<TableColumn["choices"]> {
  const { preview, live } = ctx;
  const type = field.type;
  // The settings' own: the project's languages by their own names, the offered themes by their titles.
  if (type.kind === "language") return languagesOf(preview.project.locale.language, Object.keys(preview.project.translations)).map((l) => ({ value: l, label: languageName(l) }));
  if (type.kind === "theme") return offeredThemes(preview).map((t) => ({ value: t.name.name, label: tr(preview, t.label) }));
  if (type.kind === "enum") return (findEnum(preview, type.name.name)?.variants ?? []).map((v) => ({ value: v.name.name, label: tr(preview, v.label) }));
  if (type.kind !== "ref") return [];
  // Running, a reference holds the id of its record; in the designer, the sample's text.
  const records = live
    ? live.refOptions(type.entity.name).map((o) => ({ value: o.id, label: o.text }))
    : (preview.displays[type.entity.name] ?? []).map((text) => ({ value: text, label: text }));
  return [{ value: null, label: "" }, ...records];
}

/** A table's or tree's columns for the library's table: how each field shows and is edited. */
export function tableColumnsOf(ctx: PartContext): TableColumn[] {
  const { p, preview, collection } = ctx;
  if (!collection) return [];
  const entity = collection.entity;
  // A tree shows the fields of `show`, else its record's text.
  const display = entity.fields.find((f) => f.type.kind === "text" || f.type.kind === "email")?.name.name ?? "id";
  const names = p.kind === "tree" ? (p.show.length ? p.show : [display]) : p.columns;
  return names.map((name) => {
    const field = findField(entity, name);
    const label = field ? tr(preview, field.label) : name;
    if (!field) return { field: name, label, kind: "text", edit: false };
    const kind = cellKind(field.type.kind);
    const format = fieldFormat(field);
    const choices = kind === "choice" ? choicesOf(ctx, field) : undefined;
    return {
      field: name,
      label,
      kind,
      edit: p.edit.includes(name),
      switch: p.switches.includes(name),
      width: p.widths[name],
      // A choice (an enum, a reference) shows its label; a reference holds only its record's id.
      shown: choices ? (value) => choices.find((c) => c.value === (value ?? null))?.label ?? "" : (value) => displayValue(preview, field, value),
      choices,
      number: kind === "number" && format ? numberEditing(format, formatsOf(preview)) : undefined,
    };
  });
}

/** Declares what the part shows of its list for the runtime to fill in, in a running application. */
export function declareView(ctx: PartContext, view: Record<string, unknown> | null) {
  const { live, design, p } = ctx;
  if (view && live && !design && p.name && p.bind) live.declareView?.(p.name, view);
}

/** What the runtime filled in of the part's view; none in the designer or before the first answer. */
export function viewFilled(ctx: PartContext): Record<string, unknown> | null {
  const { live, p } = ctx;
  return live && p.name ? ((live.views?.[p.name] as Record<string, unknown> | undefined) ?? null) : null;
}

/** Tells the runtime what the person did to the part's view (a sort, a page, a row opened). */
export function viewAction(ctx: PartContext): ((action: Record<string, unknown>) => void) | undefined {
  const { live, p } = ctx;
  return live?.view && p.name ? (action) => void live.view!(p.name!, action) : undefined;
}

/** Whether the person moves this part's rows now (`reorder by`, DESIGN 5.30). */
export function reorders(ctx: PartContext): boolean {
  return !!ctx.p.reorder && !!ctx.live && !ctx.design && !ctx.disabled;
}

/**
 * A row moved by the person (DESIGN 5.30): the runtime gives the list its
 * new order, each row its number in the `reorder by` field (and in a tree
 * its parent), chooses the moved row and runs `on reorder`, as one edit.
 */
export function reordered(ctx: PartContext, move: Move) {
  const { live, design, p, collection } = ctx;
  if (!live || design || !p.name || !p.bind || !p.reorder || !collection) return;
  const tree = p.kind === "tree" || p.kind === "treeTable";
  const own = collection.entity;
  const parent = tree ? (own.fields.find((f) => f.type.kind === "ref" && f.type.entity.name === own.name.name)?.name.name ?? null) : null;
  void live.reorder?.(p.name, { list: p.bind, by: p.reorder, parent, move }, wiredCall(p.handlers, "reorder"));
}

/** The entity of the record the part binds into: what its pictures and files are kept under. */
export function boundEntity(ctx: PartContext): string | undefined {
  const { p, data } = ctx;
  return p.bind ? data[p.bind.split(".")[0]]?.entity.name.name : undefined;
}
