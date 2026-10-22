// The ERD on the designer's canvas: entities drawn as boxes of fields,
// relations as lines with how many at each end, and the edits the canvas
// makes to the ERD. The backend reads and writes the Mermaid text
// (read_erd, write_erd); the drawing is designCanvas.ts.

import type { Erd } from "./generated/Erd";
import type { ErdEntity } from "./generated/ErdEntity";
import { freeName, pinned, type Box, type CanvasEdit, type CanvasPick, type Link, type PaletteItem, type Shape } from "./designCanvas.ts";

/** What the palette offers on an ERD. */
export const ERD_PALETTE: PaletteItem[] = [
  { id: "entity", label: "Entity", glyph: "entity", tool: false, title: "Drag onto the canvas to add an entity; a click puts it in the middle" },
  { id: "relation", label: "Relation", glyph: "relation", tool: true, title: "Drag from one entity to another to relate them: many of the first refer to one of the second" },
];

/** A field's row: its type, its name, and its keys (FK for a ref, UK for unique, * for required). */
export function rowsOf(entity: ErdEntity): string[][] {
  return entity.fields.map((f) => {
    const keys = [f.typeText.startsWith("ref ") ? "FK" : "", f.unique ? "UK" : "", f.required ? "*" : ""].filter(Boolean).join(" ");
    return [f.typeText, f.name, keys];
  });
}

export function erdShapes(erd: Erd): Shape[] {
  return erd.entities.map((e) => ({ name: e.name, x: e.x, y: e.y, rows: rowsOf(e), tone: "entity", cells: ["type", "field", "keys"] }));
}

/** The relations as lines: how many at each end, dashed unless the many are owned. */
export function erdLinks(erd: Erd): Link[] {
  return erd.relations.map((r) => ({
    left: r.left,
    right: r.right,
    leftEnd: { kind: "count", count: r.leftCount },
    rightEnd: { kind: "count", count: r.rightCount },
    dashed: !r.identifying,
    label: r.label,
  }));
}

function rename(erd: Erd, from: string, to: string): Erd {
  return {
    entities: erd.entities.map((e) => (e.name === from ? { ...e, name: to } : e)),
    relations: erd.relations.map((r) => ({ ...r, left: r.left === from ? to : r.left, right: r.right === from ? to : r.right })),
  };
}

/** Adds an entity at (x, y), named Entity1, Entity2... as the first free name. */
export function addEntity(erd: Erd, x: number, y: number): Erd {
  const name = freeName("Entity", erd.entities.map((e) => e.name));
  return { ...erd, entities: [...erd.entities, { name, x: Math.round(x), y: Math.round(y), fields: [] }] };
}

/** Puts an entity's box at (x, y). */
export function moveEntity(erd: Erd, name: string, x: number, y: number): Erd {
  return { ...erd, entities: erd.entities.map((e) => (e.name === name ? { ...e, x: Math.round(x), y: Math.round(y) } : e)) };
}

/** Renames an entity and its relations; the same ERD when the name is empty or in use. */
export function renameEntity(erd: Erd, from: string, to: string): Erd {
  const name = to.trim();
  if (!name || name === from || erd.entities.some((e) => e.name === name)) return erd;
  return rename(erd, from, name);
}

/** Takes an entity away, with its relations. */
export function removeEntity(erd: Erd, name: string): Erd {
  return { entities: erd.entities.filter((e) => e.name !== name), relations: erd.relations.filter((r) => r.left !== name && r.right !== name) };
}

/** Draws a relation from `from` to `to`: many of `from` refer to one of `to`. */
export function connect(erd: Erd, from: string, to: string): Erd {
  if (from === to) return erd;
  if (erd.relations.some((r) => (r.left === from && r.right === to) || (r.left === to && r.right === from))) return erd;
  return { ...erd, relations: [...erd.relations, { left: from, right: to, leftCount: "zeroOrMore", rightCount: "exactlyOne", identifying: false, label: null }] };
}

/** Makes an edit of the canvas in the ERD, every other box pinned where it is shown; `pick` is what to pick after it. */
export function erdEdit(erd: Erd, edit: CanvasEdit, shown: Box[]): { next: Erd; pick?: CanvasPick } {
  const pin = (next: Erd) => ({ ...next, entities: pinned(next.entities, shown) });
  switch (edit.op) {
    case "add": {
      const next = addEntity(erd, edit.x, edit.y);
      return { next: pin(next), pick: { kind: "box", name: next.entities[next.entities.length - 1].name } };
    }
    case "move":
      return { next: pin(moveEntity(erd, edit.name, edit.x, edit.y)) };
    case "moveAll":
      return { next: pin(edit.moves.reduce((e, m) => moveEntity(e, m.name, m.x, m.y), erd)) };
    case "connect": {
      const next = connect(erd, edit.from, edit.to);
      return next === erd ? { next: pin(erd) } : { next: pin(next), pick: { kind: "link", index: next.relations.length - 1 } };
    }
  }
}
