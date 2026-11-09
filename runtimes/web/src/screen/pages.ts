// Helpers for pages: lookups in the widget tree, data resolution for
// bindings, and grid geometry. Rules about what is valid live in the Rust
// checker; the geometry here only drives what the designer shows while
// dragging.

import type { App } from "../generated/App.ts";
import type { Entity } from "../generated/Entity.ts";
import type { Field } from "../generated/Field.ts";
import type { Page } from "../generated/Page.ts";
import type { Preview } from "../generated/Preview.ts";
import type { Widget } from "../generated/Widget.ts";
import type { WidgetInfo } from "../generated/WidgetInfo.ts";
import type { WidgetKind } from "../generated/WidgetKind.ts";
import type { WidgetProps } from "../generated/WidgetProps.ts";
import type { Command } from "../generated/Command.ts";
import type { Edit } from "../generated/Edit.ts";
import type { SpecFile } from "../generated/SpecFile.ts";
import { frameGrids } from "./frame";
import { findEntity, findEnum, tr, type Row } from "./model.ts";
import { dataEntity, isList, startValue, translatedDefaults, valueField } from "./windowData.ts";
import type { DataDecl } from "../generated/DataDecl.ts";

export { CONTENT_PADDING, GAP, POPOVER_COLUMN, ROW_HEIGHT, WINDOW_HEIGHT, WINDOW_WIDTH, gapOf, resizeEdits, rowAt, rowIn, scaledColumns, windowSize } from "./grid";

export function allPages(preview: Preview): Page[] {
  return preview.project.files.flatMap((f) => f.pages);
}

export function findPage(preview: Preview, name: string): Page | undefined {
  return allPages(preview).find((p) => p.name.name === name);
}

/**
 * The app seen as a window without content of its own, so the page designer
 * and the widget helpers can edit its frame. The body grid is never shown.
 */
export function appAsPage(app: App): Page {
  return {
    name: app.name,
    kind: "window",
    title: app.title,
    size: null,
    style: null,
    cell: null,
    gives: null,
    tells: [],
    data: app.data,
    columns: 12,
    widgets: [],
    frame: app.frame,
    span: app.span,
    handlers: app.handlers,
  };
}

export function widgetInfo(preview: Preview, kind: WidgetKind): WidgetInfo {
  const info = preview.catalog.find((w) => w.kind === kind);
  if (!info) throw new Error(`widget ${kind} is missing from the catalog`);
  return info;
}

/** The input widget that shows a field type by default (first match in the catalog). */
export function widgetForField(preview: Preview, field: Field): WidgetKind {
  const match = preview.catalog.find((w) => w.input && w.fieldTypes.includes(field.type.kind));
  return match?.kind ?? "label";
}

// ---------------------------------------------------------------------------
// Widget tree

/** A container in a page: the page itself, a panel, a tab or a pane. */
export type Container = { offset: number; columns: number; widgets: Widget[] };

export type WidgetLocation = { widget: Widget; container: Container };

/** The view of a line of a window's content, when it stands in the same file as the page: the designer designs it there. */
/**
 * How many bits of a designer's item are the place in a file
 * (clarity_spec::cxaml::DESIGN_PLACE_BITS): the parts of a view drawn in a
 * window's content, from the view's own file (a `.cxaml`), are told apart
 * from the window's by the file's number above them.
 */
export const DESIGN_PLACE = 2 ** 24;

/** The designer's item for `start` in the project's file number `file` (clarity_spec::cxaml::design_item). */
export function designItem(file: number, start: number): number {
  return (file + 1) * DESIGN_PLACE + start;
}

/** The view `view` and its file's number, and whether it is in the file of `page`. */
function viewOf(preview: Preview, page: Page, view: string | null): { view: Page; file: number; own: boolean } | null {
  if (!view) return null;
  const files = preview.project.files;
  const own = files.findIndex((f) => f.pages.some((p) => p.span.start === page.span.start && p.name.name === page.name.name));
  for (let i = 0; i < files.length; i++) {
    const found = files[i].pages.find((p) => p.name.name === view && p.kind === "view");
    if (found) return { view: found, file: i, own: i === own };
  }
  return null;
}

/** A view's parts with their places as designer's items of file number `file`. */
function asDesignItems(widgets: Widget[], file: number): Widget[] {
  return widgets.map((w) => ({
    ...w,
    span: { ...w.span, start: designItem(file, w.span.start) },
    menu: w.menu ? { ...w.menu, span: { ...w.menu.span, start: designItem(file, w.menu.span.start) }, items: commandItems(w.menu.items, file) } : w.menu,
    children: asDesignItems(w.children, file),
    pages: w.pages.map((p) => ({ ...p, span: { ...p.span, start: designItem(file, p.span.start) }, children: asDesignItems(p.children, file) })),
  }));
}

function commandItems(items: Command[], file: number): Command[] {
  return items.map((c) => ({ ...c, span: { ...c.span, start: designItem(file, c.span.start) }, items: commandItems(c.items, file) }));
}

/**
 * The page as the designer designs it (DESIGN 5.33): each line of a window's
 * content holds the parts of its view, as they show there, so the window is
 * designed as it looks. A view of the page's own file keeps its places; one
 * of its own file (`.cxaml`) has them as designer's items of that file, so an
 * edit of one changes the view (`routedEdit`).
 */
export function withViewsShown(preview: Preview, page: Page): Page {
  const fill = (widgets: Widget[], depth: number): Widget[] =>
    widgets.map((w) => {
      const pages = w.pages.map((inner) => {
        const found = w.props.kind === "content" && depth < 8 ? viewOf(preview, page, inner.view) : null;
        if (!found || inner.children.length > 0) return { ...inner, children: fill(inner.children, depth) };
        const parts = found.own ? found.view.widgets : asDesignItems(found.view.widgets, found.file);
        return { ...inner, children: fill(parts, depth + 1), columns: found.view.columns };
      });
      return { ...w, children: fill(w.children, depth), pages };
    });
  return { ...page, widgets: fill(page.widgets, 0) };
}

/** The fields of an edit that name a place in a file. */
const PLACE_FIELDS = ["item", "target", "container", "page", "owner", "parent", "app", "entity"] as const;

/**
 * The file an edit of the designer changes, and the edit in that file's
 * places: the page's own file, or the file of the view whose parts it names
 * (designer's items above DESIGN_PLACE). `base` turns a place in that file
 * back into the designer's item, for the part to stay picked.
 */
export function routedEdit(preview: Preview, file: SpecFile, edit: Edit): { file: SpecFile; edit: Edit; base: number } {
  const files = new Set<number>();
  const collect = (e: Record<string, unknown>) => {
    for (const key of PLACE_FIELDS) if (typeof e[key] === "number") files.add(Math.floor((e[key] as number) / DESIGN_PLACE));
    if (Array.isArray(e.edits)) for (const inner of e.edits) collect(inner as Record<string, unknown>);
  };
  collect(edit as unknown as Record<string, unknown>);
  const other = [...files].filter((f) => f > 0);
  if (other.length === 0) return { file, edit, base: 0 };
  if (other.length > 1 || files.has(0)) throw new Error("this change names parts of two files at once; change each in its own view");
  const target = preview.project.files[other[0] - 1];
  if (!target) throw new Error("the view this part belongs to is not in the project");
  const local = (e: Record<string, unknown>): Record<string, unknown> => {
    const out: Record<string, unknown> = { ...e };
    for (const key of PLACE_FIELDS) if (typeof out[key] === "number") out[key] = (out[key] as number) % DESIGN_PLACE;
    if (Array.isArray(out.edits)) out.edits = out.edits.map((inner) => local(inner as Record<string, unknown>));
    return out;
  };
  return { file: target, edit: local(edit as unknown as Record<string, unknown>) as unknown as Edit, base: other[0] * DESIGN_PLACE };
}

/**
 * The view each line of a window's content shows, by the line's place: the
 * view's parts are designed on that line's grid, which is the view's own;
 * a view of its own file by its designer's item.
 */
export function contentViews(preview: Preview, page: Page): Map<number, number> {
  const out = new Map<number, number>();
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      for (const inner of w.pages) {
        const found = w.props.kind === "content" ? viewOf(preview, page, inner.view) : null;
        if (found) out.set(inner.span.start, found.own ? found.view.span.start : designItem(found.file, found.view.span.start));
        visit(inner.children);
      }
      visit(w.children);
    }
  };
  visit(page.widgets);
  return out;
}

export function containers(preview: Preview, page: Page): Container[] {
  const out: Container[] = [{ offset: page.span.start, columns: page.columns, widgets: page.widgets }];
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      if (widgetInfo(preview, w.props.kind).container === "plain") {
        out.push({ offset: w.span.start, columns: w.props.width, widgets: w.children });
      }
      for (const inner of w.pages) {
        // A line of a window's content is its view: what is dropped there goes into the view, on its grid.
        const found = w.props.kind === "content" ? viewOf(preview, page, inner.view) : null;
        const offset = found ? (found.own ? found.view.span.start : designItem(found.file, found.view.span.start)) : inner.span.start;
        out.push(found ? { offset, columns: found.view.columns, widgets: inner.children } : { offset, columns: w.props.width, widgets: inner.children });
      }
      visit(w.children);
      for (const inner of w.pages) visit(inner.children);
    }
  };
  for (const grid of frameGrids(page.frame)) {
    out.push({ offset: grid.offset, columns: grid.columns, widgets: grid.widgets });
  }
  visit(page.widgets);
  for (const grid of frameGrids(page.frame)) visit(grid.widgets);
  return out;
}

export function findContainer(preview: Preview, page: Page, offset: number): Container | undefined {
  return containers(preview, page).find((c) => c.offset === offset);
}

export function locateWidget(preview: Preview, page: Page, offset: number): WidgetLocation | null {
  for (const container of containers(preview, page)) {
    const widget = container.widgets.find((w) => w.span.start === offset);
    if (widget) return { widget, container };
  }
  return null;
}

/** Tab or pane of a tabs/splitter widget, by offset. */
export function locateInnerPage(page: Page, offset: number): { widget: Widget; index: number } | null {
  const visit = (widgets: Widget[]): { widget: Widget; index: number } | null => {
    for (const w of widgets) {
      const index = w.pages.findIndex((p) => p.span.start === offset);
      if (index >= 0) return { widget: w, index };
      const found = visit(w.children) ?? w.pages.map((p) => visit(p.children)).find((x) => x) ?? null;
      if (found) return found;
    }
    return null;
  };
  return visit([...page.widgets, ...frameGrids(page.frame).flatMap((g) => g.widgets)]);
}

/** Rows a grid shows: enough for its widgets, plus room to drop below them. */
export function gridRows(widgets: Widget[], extra: number, minimum: number): number {
  const bottom = widgets.reduce((max, w) => Math.max(max, w.props.row + w.props.height - 1), 0);
  return Math.max(bottom + extra, minimum);
}

export type Rect = { col: number; row: number; width: number; height: number };

/**
 * What tells the parts of a grid apart: where they start in the file. A label
 * a form's label column made stands at its input's place, so it is told apart.
 */
export function partKey(widget: Widget): string {
  return widget.props.made ? `label ${widget.span.start}` : `${widget.span.start}`;
}

export function overlaps(a: Rect, b: Rect): boolean {
  return a.col < b.col + b.width && b.col < a.col + a.width && a.row < b.row + b.height && b.row < a.row + a.height;
}

// ---------------------------------------------------------------------------
// Data

/** The data a page shows, by name: one record or a list of records, or a
 *  plain value. A plain value is a record of one field named after it, and
 *  `value` is set. */
export type PageData = Record<string, { entity: Entity; list: boolean; rows: Row[]; value?: boolean }>;

/** A plain value as a record of one field, in `row`. */
export function valueEntry(decl: DataDecl, row: Row): PageData[string] | null {
  const field = valueField(decl);
  if (!field) return null;
  const entity: Entity = { name: decl.name, fields: [field], table: null, key: null, settings: false, record: false, span: decl.span };
  return { entity, list: false, rows: [row], value: true };
}

/** The first value of an enum, where a plain value of it starts. */
export function firstValue(preview: Preview, decl: DataDecl): string | null {
  if (decl.of.kind !== "value" || decl.of.ty.kind.kind !== "named") return null;
  return findEnum(preview, decl.of.ty.kind.name)?.variants[0]?.name.name ?? null;
}

/** Sample data a page shows in the designer. */
export function pageData(preview: Preview, page: Page): PageData {
  const data: PageData = {};
  for (const decl of page.data) {
    const name = dataEntity(decl);
    if (name === null) {
      // A text default shows in the designer's language, as labels do.
      const start = translatedDefaults([decl], { [decl.name.name]: startValue(decl, firstValue(preview, decl)) }, (text) => tr(preview, text));
      const entry = valueEntry(decl, start as Row);
      if (entry) data[decl.name.name] = entry;
      continue;
    }
    const entity = findEntity(preview, name);
    if (!entity) continue;
    const rows = preview.samples[entity.name.name] ?? [];
    data[decl.name.name] = { entity, list: isList(decl), rows: isList(decl) ? rows : rows.slice(0, 1) };
  }
  return data;
}

/** The field a binding points at, if it resolves: `data.field`, or a plain value by its name. */
export function boundField(data: PageData, bind: string | null): { record: Row; field: Field } | null {
  if (!bind) return null;
  const [name, fieldName] = bind.split(".");
  const entry = data[name];
  const wanted = fieldName ?? (entry?.value ? name : undefined);
  const field = wanted ? entry?.entity.fields.find((f) => f.name.name === wanted) : undefined;
  const record = entry?.rows[0];
  return field && record ? { record, field } : null;
}

/** Copies props with changes; the designer always sends complete props. */
export function withProps(props: WidgetProps, changes: Partial<WidgetProps>): WidgetProps {
  return { ...props, ...changes };
}

/**
 * Whether a part takes extra height in a fluid window. Tables, lists,
 * images and canvases do; a container (panel, tabs, splitter) only when
 * something inside it does, so a panel of inputs keeps its height.
 */
export function widgetGrows(preview: Preview, widget: Widget): boolean {
  // A custom part grows as its declaration says (`grows`, DESIGN 5.19).
  if (widget.props.kind === "custom") return widget.props.grows;
  const info = widgetInfo(preview, widget.props.kind);
  if (!info.grows) return false;
  if (info.container === null) return true;
  const inside = [...widget.children, ...widget.pages.flatMap((p) => p.children)];
  return inside.some((w) => widgetGrows(preview, w));
}

/** Every part of a page, its sidebars and footer included, nested parts too. */
/**
 * Where a part's edits are undone (DESIGN 5.29), as the project worked it
 * out for every screen: the form or panel it is in, or the part itself.
 * None for a part without undo (`no undo`, or one that edits nothing).
 */
export function undoScope(page: Page, name: string): string | null {
  return allParts(page).find((w) => w.props.name === name)?.props.undoScope ?? null;
}

export function allParts(page: Page): Widget[] {
  const out: Widget[] = [];
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      out.push(w);
      visit(w.children);
      for (const inner of w.pages) visit(inner.children);
    }
  };
  visit(page.widgets);
  for (const grid of frameGrids(page.frame)) visit(grid.widgets);
  return out;
}

/** A part by its name. */
export function findPart(page: Page, name: string | null): Widget | undefined {
  return name === null ? undefined : allParts(page).find((w) => w.props.name === name);
}

/** The field a `data.field` binding points at, without needing a record. */
export function fieldOf(data: PageData, bind: string | null): Field | undefined {
  if (!bind) return undefined;
  const [name, fieldName] = bind.split(".");
  return fieldName ? data[name]?.entity.fields.find((f) => f.name.name === fieldName) : undefined;
}

/** The DOM id of a named part, for labels and help texts that point at it. */
export function partId(name: string): string {
  return `part-${name}`;
}

/** The DOM id of the help text for a named input. */
export function helpId(name: string): string {
  return `help-for-${name}`;
}
