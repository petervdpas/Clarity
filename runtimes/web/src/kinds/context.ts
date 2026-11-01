// What every kind of part gets from the screen: one small, fixed interface
// (DESIGN 7.8, G6a). A part module reads what it shows from here, and tells
// what the person did through `fire`, `setValue` and the rest; what it keeps
// of its own (a splitter's divider, a wizard's step, a table's columns) it
// keeps itself. The designer's own behavior comes in here too, so no part
// depends on the IDE.

import type { Snippet } from "svelte";
import type { Event as PartEvent } from "../generated/Event.ts";
import type { Field } from "../generated/Field.ts";
import type { JsonValue } from "../generated/serde_json/JsonValue.ts";
import type { Page } from "../generated/Page.ts";
import type { Preview } from "../generated/Preview.ts";
import type { Widget } from "../generated/Widget.ts";
import type { WidgetInfo } from "../generated/WidgetInfo.ts";
import type { WidgetProps } from "../generated/WidgetProps.ts";
import type { Live, PartState } from "../screen/live.ts";
import type { Row } from "../screen/model.ts";
import type { PageData } from "../screen/pages.ts";

/** A grid of parts inside a container: its tab, pane, section or own children. */
export type GridArgs = {
  widgets: Widget[];
  columns: number;
  /** The offset of what holds the grid: the container, or its tab, pane or section. */
  container: number;
  /** A grid of one row of fixed height, its parts joined (a button group). */
  fixedRows?: number;
  joined?: boolean;
};

export interface PartContext {
  // What the part is, and where.
  readonly preview: Preview;
  readonly page: Page;
  readonly data: PageData;
  readonly widget: Widget;
  readonly p: WidgetProps;
  readonly info: WidgetInfo;
  /** Where the part is written in its file, which names it in the designer. */
  readonly offset: number;
  /** Shown in the designer: nothing runs, and what code fills in shows the part's name. */
  readonly design: boolean;
  /** In a container (a tab page, a pane, a panel, a section, a step, a scroll box), which frames it: a list, table or tree then shows no border of its own unless it has `frame` (DESIGN 7.8). */
  readonly inside: boolean;

  // What it shows.
  /** The record and field it binds, if it binds one. */
  readonly bound: { record: Row; field: Field } | null;
  /** The bound field's value. */
  readonly value: JsonValue | null | undefined;
  /** What a text input shows: its field, or the text it keeps unbound. */
  readonly shown: string;
  /** The list it binds, when it binds a whole list of the window's data. */
  readonly collection: PageData[string] | undefined;
  /** Its caption, as code set it or as written, translated. */
  readonly caption: string | null;
  /** In the designer, a text part without text of its own shows its name. */
  readonly unset: string | null;
  /** The error a handler set on it, shown. */
  readonly error: string | null;
  /** The class of its style in the project's theme. */
  readonly partClass: string | null;
  /** Its element id, and the id of the help text that describes it. */
  readonly id: string | undefined;
  readonly describedBy: string | undefined;
  /** A label's or help text's input: its field, and the error on it. */
  readonly targetField: Field | undefined;
  readonly targetError: string | null;

  // The running application, if one runs.
  readonly live: Live | undefined;
  readonly partState: PartState | undefined;
  readonly disabled: boolean;
  readonly readonly: boolean;
  readonly selectedRow: number | undefined;

  // What the person did.
  /** Sets the bound field, as one edit for undo. */
  setValue(next: JsonValue): void;
  /** Sets the bound field, or the text an unbound input keeps. */
  setText(next: string): void;
  /** Runs the handler wired to `event`; false when it refused. */
  fire(event: PartEvent): Promise<boolean>;
  /** Enter in an input of a form: runs the form's `on submit`; false when there is none. */
  submit(): boolean;
  /** A click: its handler, or the window or popover it opens. */
  click(): void;
  /** Chooses a row; `select` runs when it is another row than before. */
  selectRow(index: number): void;
  /** Opens the part's menu at a place on the screen (a split button's arrow). */
  openMenu(x: number, y: number): void;
  /** Goes to another window, from a click that opens one. */
  onnavigate(target: string, newWindow?: boolean): void;

  // The designer's: in a running application these do nothing.
  /** The tab, step or section shown while designing. */
  readonly activeTab: number;
  /** Whether the designer has this element selected. */
  isSelected(offset: number): boolean;
  /** A press on a tab, step or section header in the designer. */
  pressTab(e: PointerEvent, index: number, tabOffset: number): void;
  /** A table column's width dragged in the designer; 0 takes it away. */
  designWidth(field: string, width: number): void;

  /** Draws a grid of parts inside this one, as the screen draws every grid. */
  grid: Snippet<[GridArgs]>;
}
