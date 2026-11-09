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

/** A container in a page of one file, by where it starts: the page itself, a panel, a tab or a pane. */
export type Container = { offset: number; columns: number; widgets: Widget[] };

export type WidgetLocation = { widget: Widget; container: Container };

// ---------------------------------------------------------------------------
// Design keys

/**
 * What the designer calls each thing it shows (clarity_spec::design_keys):
 * its file and name, `design/Main.cxaml#Main/saveButton`. The layer, the
 * pick and the page each container shows go by keys; an edit is made in the
 * places of the file its key names.
 */
export type DesignKey = string;

/** The key of the thing that starts at `at` in `file`. */
export function keyAt(preview: Preview, file: SpecFile, at: number): DesignKey | null {
  return preview.designKeys[file.path]?.[at] ?? null;
}

/** Where a key's thing is written: its file, the window, view or app (as a page) it belongs to, and where it starts. */
export type Placed = { file: SpecFile; page: Page; at: number };

const PLACES = new WeakMap<Preview, Map<DesignKey, Placed>>();

/** Where the thing `key` names is written; null for a key the project no longer has. */
export function placeOf(preview: Preview, key: DesignKey | null): Placed | null {
  if (key === null) return null;
  let places = PLACES.get(preview);
  if (!places) {
    places = new Map();
    for (const file of preview.project.files) {
      const owners: Page[] = [...file.pages, ...file.apps.map(appAsPage)].sort((a, b) => a.span.start - b.span.start);
      for (const [start, k] of Object.entries(preview.designKeys[file.path] ?? {})) {
        const at = Number(start);
        // The window, view or app it is in: the last that starts at or before it.
        const page = owners.filter((o) => o.span.start <= at).pop();
        if (page) places.set(k, { file, page, at });
      }
    }
    PLACES.set(preview, places);
  }
  return places.get(key) ?? null;
}

/** Something of a page the designer picks: it has a place in its file. */
type Spanned = { span: { start: number } };

/**
 * A window as the designer designs it, with the key of each thing it shows:
 * a thing of a view drawn in the window's content is of the view's file.
 */
export type ShownPage = { page: Page; file: SpecFile; keyOf: (thing: Spanned) => DesignKey | null };

/** The view `view`, and the file it is written in. */
function viewOf(preview: Preview, view: string | null): { view: Page; file: SpecFile } | null {
  if (!view) return null;
  for (const file of preview.project.files) {
    const found = file.pages.find((p) => p.name.name === view && p.kind === "view");
    if (found) return { view: found, file };
  }
  return null;
}

/**
 * The page as the designer designs it (DESIGN 5.33): each line of a window's
 * content holds the parts of its view, as they show there, so the window is
 * designed as it looks; an edit of one of them changes the view's file.
 */
export function shownPage(preview: Preview, page: Page, file: SpecFile): ShownPage {
  const keys = new Map<object, DesignKey | null>();
  // Everything of a view's part, in the view's file.
  const mark = (w: Widget, of: SpecFile) => {
    const key = (t: Spanned) => keys.set(t, keyAt(preview, of, t.span.start));
    const commands = (items: Command[]) => items.forEach((c) => (key(c), commands(c.items)));
    const levels = (all: Widget["levels"]) => all.forEach((l) => (key(l), levels(l.levels)));
    key(w);
    if (w.menu) (key(w.menu), commands(w.menu.items));
    levels(w.levels);
    w.selects.forEach(key);
    w.children.forEach((c) => mark(c, of));
    w.pages.forEach((p) => (key(p), p.children.forEach((c) => mark(c, of))));
  };
  const keyOf = (t: Spanned) => (keys.has(t) ? (keys.get(t) ?? null) : keyAt(preview, file, t.span.start));
  // A copy is the thing it copies.
  const same = <T extends Spanned>(copy: T, of: Spanned): T => (keys.set(copy, keyOf(of)), copy);
  const fill = (widgets: Widget[], depth: number): Widget[] =>
    widgets.map((w) => {
      const pages = w.pages.map((inner) => {
        const found = w.props.kind === "content" && depth < 8 ? viewOf(preview, inner.view) : null;
        if (!found || inner.children.length > 0) return same({ ...inner, children: fill(inner.children, depth) }, inner);
        found.view.widgets.forEach((v) => mark(v, found.file));
        return same({ ...inner, children: fill(found.view.widgets, depth + 1), columns: found.view.columns }, inner);
      });
      return same({ ...w, children: fill(w.children, depth), pages }, w);
    });
  return { page: { ...page, widgets: fill(page.widgets, 0) }, file, keyOf };
}

/** A part of the shown page by its key, and the container it is in. */
export function locateKey(preview: Preview, shown: ShownPage, key: DesignKey | null): { widget: Widget; container: ShownContainer } | null {
  if (key === null) return null;
  for (const container of shownContainers(preview, shown)) {
    const widget = container.widgets.find((w) => shown.keyOf(w) === key);
    if (widget) return { widget, container };
  }
  return null;
}

/** A tab, step, section or pane of the shown page by its key. */
export function innerPageOf(shown: ShownPage, key: DesignKey | null): { widget: Widget; index: number } | null {
  if (key === null) return null;
  const visit = (widgets: Widget[]): { widget: Widget; index: number } | null => {
    for (const w of widgets) {
      const index = w.pages.findIndex((p) => shown.keyOf(p) === key);
      if (index >= 0) return { widget: w, index };
      const found = visit(w.children) ?? w.pages.map((p) => visit(p.children)).find((x) => x) ?? null;
      if (found) return found;
    }
    return null;
  };
  return visit([...shown.page.widgets, ...frameGrids(shown.page.frame).flatMap((g) => g.widgets)]);
}

/**
 * The view each line of a window's content shows, by the line's key: the
 * view's parts are designed on that line's grid, which is the view's own.
 */
export function contentViews(preview: Preview, shown: ShownPage): Map<DesignKey, DesignKey> {
  const out = new Map<DesignKey, DesignKey>();
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      for (const inner of w.pages) {
        const found = w.props.kind === "content" ? viewOf(preview, inner.view) : null;
        const line = shown.keyOf(inner);
        const view = found ? keyAt(preview, found.file, found.view.span.start) : null;
        if (line !== null && view !== null) out.set(line, view);
        visit(inner.children);
      }
      visit(w.children);
    }
  };
  visit(shown.page.widgets);
  return out;
}

/** A container of the shown page by its key: the page, a panel, a tab or pane, a line of a content (its view), a frame's grid. */
export type ShownContainer = { key: DesignKey | null; columns: number; widgets: Widget[] };

export function shownContainers(preview: Preview, shown: ShownPage): ShownContainer[] {
  const page = shown.page;
  const out: ShownContainer[] = [{ key: shown.keyOf(page), columns: page.columns, widgets: page.widgets }];
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      if (widgetInfo(preview, w.props.kind).container === "plain") {
        out.push({ key: shown.keyOf(w), columns: w.props.width, widgets: w.children });
      }
      for (const inner of w.pages) {
        // A line of a window's content is its view: what is dropped there goes into the view, on its grid.
        const found = w.props.kind === "content" ? viewOf(preview, inner.view) : null;
        out.push(found ? { key: keyAt(preview, found.file, found.view.span.start), columns: found.view.columns, widgets: inner.children } : { key: shown.keyOf(inner), columns: w.props.width, widgets: inner.children });
      }
      visit(w.children);
      for (const inner of w.pages) visit(inner.children);
    }
  };
  for (const grid of frameGrids(page.frame)) {
    out.push({ key: keyAt(preview, shown.file, grid.offset), columns: grid.columns, widgets: grid.widgets });
  }
  visit(page.widgets);
  for (const grid of frameGrids(page.frame)) visit(grid.widgets);
  return out;
}

export function containers(preview: Preview, page: Page): Container[] {
  const out: Container[] = [{ offset: page.span.start, columns: page.columns, widgets: page.widgets }];
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      if (widgetInfo(preview, w.props.kind).container === "plain") {
        out.push({ offset: w.span.start, columns: w.props.width, widgets: w.children });
      }
      // The pages of one file's window; a line of a content shows its view from the view's own file (shownContainers).
      for (const inner of w.pages) out.push({ offset: inner.span.start, columns: w.props.width, widgets: inner.children });
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
