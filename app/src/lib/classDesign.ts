// The class diagram on the designer's canvas: entities, enums, engines and
// modules drawn as boxes of their members, relations as UML lines (an
// arrow to what is referred to, a diamond at the whole), and the edits the
// canvas makes to the design. The backend reads and writes the Mermaid
// text (read_classes, write_classes); the drawing is designCanvas.ts.

import type { Cardinality } from "./generated/Cardinality";
import type { ClassBox } from "./generated/ClassBox";
import type { ClassBoxKind } from "./generated/ClassBoxKind";
import type { ClassDesign } from "./generated/ClassDesign";
import type { ClassLink } from "./generated/ClassLink";
import { freeName, pinned, type Box, type CanvasEdit, type CanvasPick, type Link, type PaletteItem, type Shape, wholePoints } from "./designCanvas.ts";

/** What the palette offers on a class diagram: four kinds of box and three kinds of line. */
export const CLASS_PALETTE: PaletteItem[] = [
  { id: "entity", label: "Entity", glyph: "entity", tool: false, title: "Drag onto the canvas to add an entity (a class)" },
  { id: "enum", label: "Enum", glyph: "enum", tool: false, title: "Drag onto the canvas to add an enum (an <<enumeration>>)" },
  { id: "engine", label: "Engine", glyph: "engine", tool: false, title: "Drag onto the canvas to add an engine with its functions" },
  { id: "module", label: "Module", glyph: "module", tool: false, title: "Drag onto the canvas to add a module's functions" },
  { id: "association", label: "Association", glyph: "association", tool: true, title: "Drag from one box to another: many of the first refer to one of the second" },
  { id: "composition", label: "Composition", glyph: "composition", tool: true, title: "Drag from the whole to its parts: the whole owns a list of them, and they go with it" },
  { id: "aggregation", label: "Aggregation", glyph: "aggregation", tool: true, title: "Drag from the whole to its parts: the whole lists them, and they stay on their own" },
];

/** The kinds of box the palette adds, by item. */
const BOX_KINDS: Record<string, ClassBoxKind> = { entity: "entity", enum: "enum", engine: "engine", module: "module" };

/** How a new box of each kind is named: Entity1, Enum1, Engine1, module1. */
const BASE_NAMES: Record<ClassBoxKind, string> = { entity: "Entity", enum: "Enum", engine: "Engine", module: "module" };

/** The stereotype a kind is written with. */
const STEREOTYPES: Record<ClassBoxKind, string | null> = { entity: null, enum: "enumeration", engine: "engine", module: "module" };

/** The kinds of line the palette draws, by tool. */
const LINKS: Record<string, ClassLink> = { association: "association", composition: "composition", aggregation: "aggregation" };

/** A count as UML writes it beside a line's end. */
export const MULTIPLICITY: Record<Cardinality, string> = { exactlyOne: "1", zeroOrOne: "0..1", zeroOrMore: "*", oneOrMore: "1..*" };

function rowsOf(b: ClassBox): string[][] {
  switch (b.kind) {
    case "entity":
      return b.fields.map((f) => [`${f.name}: ${f.typeText}`, [f.unique ? "UK" : "", f.required ? "*" : ""].filter(Boolean).join(" ")]);
    case "enum":
      return b.values.map((v) => [v]);
    default:
      return b.functions.map((f) => [f]);
  }
}

export function classShapes(design: ClassDesign): Shape[] {
  return design.boxes.map((b) => ({ name: b.name, x: b.x, y: b.y, rows: rowsOf(b), stereotype: STEREOTYPES[b.kind], tone: b.kind, cells: ["field", "keys"], group: b.kind === "module" ? b.name : b.module }));
}

/**
 * The relations as UML lines: an association points at the right box, a
 * composition or aggregation has its diamond at the left, and a uses
 * arrow (what a module's code uses) is dashed, without counts.
 */
export function classLinks(design: ClassDesign): Link[] {
  return design.relations.map((r) => {
    if (r.link === "uses") {
      return { left: r.left, right: r.right, leftEnd: { kind: "uml", head: "none", text: "" }, rightEnd: { kind: "uml", head: "arrow", text: "" }, dashed: true, label: r.label, via: r.via.map(([x, y]) => ({ x, y })) };
    }
    const whole = r.link === "composition" ? "filledDiamond" : r.link === "aggregation" ? "diamond" : "none";
    return {
      left: r.left,
      right: r.right,
      leftEnd: { kind: "uml", head: whole, text: MULTIPLICITY[r.leftCount] },
      rightEnd: { kind: "uml", head: r.link === "association" ? "arrow" : "none", text: MULTIPLICITY[r.rightCount] },
      dashed: false,
      label: r.label,
      via: r.via.map(([x, y]) => ({ x, y })),
    };
  });
}

/** Adds a box of `kind` at (x, y), with the first free name of its kind. */
export function addBox(design: ClassDesign, kind: ClassBoxKind, x: number, y: number): ClassDesign {
  const name = freeName(BASE_NAMES[kind], design.boxes.map((b) => b.name));
  const added: ClassBox = { name, kind, module: null, x: Math.round(x), y: Math.round(y), fields: [], values: [], functions: [] };
  return { ...design, boxes: [...design.boxes, added] };
}

export function moveBox(design: ClassDesign, name: string, x: number, y: number): ClassDesign {
  return { ...design, boxes: design.boxes.map((b) => (b.name === name ? { ...b, x: Math.round(x), y: Math.round(y) } : b)) };
}

/** Renames a box and its relations; the same design when the name is empty or in use. */
export function renameBox(design: ClassDesign, from: string, to: string): ClassDesign {
  const name = to.trim();
  if (!name || name === from || design.boxes.some((b) => b.name === name)) return design;
  return {
    boxes: design.boxes.map((b) => (b.name === from ? { ...b, name } : b)),
    relations: design.relations.map((r) => ({ ...r, left: r.left === from ? name : r.left, right: r.right === from ? name : r.right })),
  };
}

/** Takes a box away, with its relations. */
export function removeBox(design: ClassDesign, name: string): ClassDesign {
  return { boxes: design.boxes.filter((b) => b.name !== name), relations: design.relations.filter((r) => r.left !== name && r.right !== name) };
}

/** Draws a line of `link` from `from` to `to`: the referrer or the whole stands left. */
export function connectBoxes(design: ClassDesign, link: ClassLink, from: string, to: string): ClassDesign {
  if (from === to) return design;
  if (design.relations.some((r) => (r.left === from && r.right === to) || (r.left === to && r.right === from))) return design;
  const counts: [Cardinality, Cardinality] = link === "association" ? ["zeroOrMore", "exactlyOne"] : ["exactlyOne", "zeroOrMore"];
  return { ...design, relations: [...design.relations, { left: from, right: to, link, leftCount: counts[0], rightCount: counts[1], label: null, via: [] }] };
}

/** Makes an edit of the canvas in the design, every other box pinned where it is shown; `pick` is what to pick after it. */
export function classEdit(design: ClassDesign, edit: CanvasEdit, shown: Box[]): { next: ClassDesign; pick?: CanvasPick } {
  const pin = (next: ClassDesign) => ({ ...next, boxes: pinned(next.boxes, shown) });
  switch (edit.op) {
    case "add": {
      const kind = BOX_KINDS[edit.item];
      if (!kind) return { next: design };
      const next = addBox(design, kind, edit.x, edit.y);
      return { next: pin(next), pick: { kind: "box", name: next.boxes[next.boxes.length - 1].name } };
    }
    case "move":
      return { next: pin(moveBox(design, edit.name, edit.x, edit.y)) };
    case "moveAll":
      return { next: pin(edit.moves.reduce((d, m) => moveBox(d, m.name, m.x, m.y), design)) };
    case "connect": {
      const link = LINKS[edit.tool];
      const next = link ? connectBoxes(design, link, edit.from, edit.to) : design;
      return next === design ? { next: pin(design) } : { next: pin(next), pick: { kind: "link", index: next.relations.length - 1 } };
    }
    case "bend":
      return { next: pin({ ...design, relations: design.relations.map((r, i) => (i === edit.index ? { ...r, via: wholePoints(edit.via) } : r)) }) };
  }
}
