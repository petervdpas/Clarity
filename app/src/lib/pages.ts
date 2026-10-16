// Helpers for pages: lookups in the widget tree, data resolution for
// bindings, and grid geometry. Rules about what is valid live in the Rust
// checker; the geometry here only drives what the designer shows while
// dragging.

import type { App } from "./generated/App";
import type { Entity } from "./generated/Entity";
import type { Field } from "./generated/Field";
import type { Page } from "./generated/Page";
import type { Preview } from "./generated/Preview";
import type { Widget } from "./generated/Widget";
import type { WidgetInfo } from "./generated/WidgetInfo";
import type { WidgetKind } from "./generated/WidgetKind";
import type { WidgetProps } from "./generated/WidgetProps";
import { frameGrids } from "./frame";
import { findEntity, type Row } from "./model";

/** Height of one grid row and the gap between cells, in pixels. */
export const ROW_HEIGHT = 36;
export const GAP = 8;

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
    data: app.data,
    columns: 12,
    widgets: [],
    frame: app.frame,
    span: app.span,
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

export function containers(preview: Preview, page: Page): Container[] {
  const out: Container[] = [{ offset: page.span.start, columns: page.columns, widgets: page.widgets }];
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      if (widgetInfo(preview, w.props.kind).container === "plain") {
        out.push({ offset: w.span.start, columns: w.props.width, widgets: w.children });
      }
      for (const inner of w.pages) {
        out.push({ offset: inner.span.start, columns: w.props.width, widgets: inner.children });
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

export function overlaps(a: Rect, b: Rect): boolean {
  return a.col < b.col + b.width && b.col < a.col + a.width && a.row < b.row + b.height && b.row < a.row + a.height;
}

// ---------------------------------------------------------------------------
// Data

/** Sample data a page shows: one record or a list of records per data declaration. */
export type PageData = Record<string, { entity: Entity; list: boolean; rows: Row[] }>;

export function pageData(preview: Preview, page: Page): PageData {
  const data: PageData = {};
  for (const decl of page.data) {
    const entity = findEntity(preview, decl.entity.name);
    if (!entity) continue;
    const rows = preview.samples[entity.name.name] ?? [];
    data[decl.name.name] = { entity, list: decl.list, rows: decl.list ? rows : rows.slice(0, 1) };
  }
  return data;
}

/** The field a `data.field` binding points at, if it resolves. */
export function boundField(data: PageData, bind: string | null): { record: Row; field: Field } | null {
  if (!bind) return null;
  const [name, fieldName] = bind.split(".");
  const entry = data[name];
  const field = fieldName ? entry?.entity.fields.find((f) => f.name.name === fieldName) : undefined;
  const record = entry?.rows[0];
  return field && record ? { record, field } : null;
}

/** Copies props with changes; the designer always sends complete props. */
export function withProps(props: WidgetProps, changes: Partial<WidgetProps>): WidgetProps {
  return { ...props, ...changes };
}
