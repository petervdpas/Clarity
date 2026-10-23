// Frame regions around a window: which regions a window shows (its own or
// the app's), and finding a picked frame element by offset.

import type { App } from "./generated/App";
import type { Command } from "./generated/Command";
import type { Footer } from "./generated/Footer";
import type { Frame } from "./generated/Frame";
import type { Menu } from "./generated/Menu";
import type { Page } from "./generated/Page";
import type { Preview } from "./generated/Preview";
import type { Ribbon } from "./generated/Ribbon";
import type { RibbonGroup } from "./generated/RibbonGroup";
import type { RibbonTab } from "./generated/RibbonTab";
import type { Sidebar } from "./generated/Sidebar";
import type { Toolbar } from "./generated/Toolbar";
import type { Widget } from "./generated/Widget";

export function findApp(preview: Preview): App | null {
  return preview.project.files.flatMap((f) => f.apps)[0] ?? null;
}

/** A region as a window shows it; `inherited` when it comes from the app. */
export type Shown<T> = { value: T; inherited: boolean };

export type ShownFrame = {
  menu: Shown<Menu> | null;
  toolbar: Shown<Toolbar> | null;
  ribbon: Shown<Ribbon> | null;
  left: Shown<Sidebar> | null;
  right: Shown<Sidebar> | null;
  footer: Shown<Footer> | null;
};

export const NO_FRAME: ShownFrame = { menu: null, toolbar: null, ribbon: null, left: null, right: null, footer: null };

function pick<T>(own: T | null | undefined, app: T | null | undefined): Shown<T> | null {
  if (own) return { value: own, inherited: false };
  if (app) return { value: app, inherited: true };
  return null;
}

/** The frame a window shows: each of its own regions replaces the app's. */
export function shownFrame(app: App | null, own: Frame | null): ShownFrame {
  const a = app?.frame ?? null;
  const side = (frame: Frame | null, s: "left" | "right") => frame?.sidebars.find((x) => x.props.side === s) ?? null;
  return {
    menu: pick(own?.menu, a?.menu),
    toolbar: pick(own?.toolbar, a?.toolbar),
    ribbon: pick(own?.ribbon, a?.ribbon),
    left: pick(side(own, "left"), side(a, "left")),
    right: pick(side(own, "right"), side(a, "right")),
    footer: pick(own?.footer, a?.footer),
  };
}

/** The frame a page shows in the preview: windows and views sit in the app frame, dialogs do not. */
export function frameForPage(preview: Preview, page: Page): ShownFrame {
  if (page.kind === "dialog") return NO_FRAME;
  return shownFrame(findApp(preview), page.kind === "window" ? page.frame : null);
}

/** A picked frame element, with what the properties panel needs. */
export type FrameElement =
  | { kind: "menu"; menu: Menu }
  | { kind: "toolbar"; toolbar: Toolbar }
  | { kind: "ribbon"; ribbon: Ribbon }
  | { kind: "sidebar"; sidebar: Sidebar }
  | { kind: "footer"; footer: Footer }
  | { kind: "status"; footer: Footer }
  | { kind: "ribbonTab"; tab: RibbonTab; index: number; count: number }
  | { kind: "ribbonGroup"; group: RibbonGroup; index: number; count: number }
  | { kind: "command"; command: Command; index: number; count: number; parent: "menu" | "toolbar" | "group" };

export function findFrameElement(frame: Frame, offset: number, widgets: Widget[] = []): FrameElement | null {
  const inCommands = (
    commands: Command[],
    parent: "menu" | "toolbar" | "group",
  ): FrameElement | null => {
    for (const [index, command] of commands.entries()) {
      if (command.span.start === offset) return { kind: "command", command, index, count: commands.length, parent };
      const found = inCommands(command.items, parent);
      if (found) return found;
    }
    return null;
  };
  if (frame.menu) {
    if (frame.menu.span.start === offset) return { kind: "menu", menu: frame.menu };
    const found = inCommands(frame.menu.items, "menu");
    if (found) return found;
  }
  if (frame.toolbar) {
    if (frame.toolbar.span.start === offset) return { kind: "toolbar", toolbar: frame.toolbar };
    const found = inCommands(frame.toolbar.items, "toolbar");
    if (found) return found;
  }
  if (frame.ribbon) {
    const ribbon = frame.ribbon;
    if (ribbon.span.start === offset) return { kind: "ribbon", ribbon };
    for (const [index, tab] of ribbon.tabs.entries()) {
      if (tab.span.start === offset) return { kind: "ribbonTab", tab, index, count: ribbon.tabs.length };
      for (const [gi, group] of tab.groups.entries()) {
        if (group.span.start === offset) return { kind: "ribbonGroup", group, index: gi, count: tab.groups.length };
        const found = inCommands(group.items, "group");
        if (found) return found;
      }
    }
  }
  for (const sidebar of frame.sidebars) {
    if (sidebar.span.start === offset) return { kind: "sidebar", sidebar };
  }
  if (frame.footer) {
    if (frame.footer.span.start === offset) return { kind: "footer", footer: frame.footer };
    if (frame.footer.statusSpan?.start === offset) return { kind: "status", footer: frame.footer };
  }
  // A part's context menu and its items are edited as the frame's menu is (DESIGN 5.16).
  const inParts = (list: Widget[]): FrameElement | null => {
    for (const w of list) {
      if (w.menu) {
        if (w.menu.span.start === offset) return { kind: "menu", menu: w.menu };
        const found = inCommands(w.menu.items, "menu");
        if (found) return found;
      }
      const found = inParts(w.children) ?? w.pages.map((p) => inParts(p.children)).find((f) => f !== null) ?? null;
      if (found) return found;
    }
    return null;
  };
  return inParts([...widgets, ...frame.sidebars.flatMap((s) => s.widgets), ...(frame.footer?.widgets ?? [])]);
}

/** Widget grids inside a frame (sidebars and the footer), for locating widgets. */
export function frameGrids(frame: Frame): { offset: number; columns: number; widgets: Widget[] }[] {
  const grids = frame.sidebars.map((s) => ({ offset: s.span.start, columns: s.props.columns, widgets: s.widgets }));
  if (frame.footer) grids.push({ offset: frame.footer.span.start, columns: frame.footer.columns, widgets: frame.footer.widgets });
  return grids;
}
