import type { RefOption } from "./dropdownOptions";
// The connection between the renderer and a running application. The
// generated app sets it; the designer and the preview do not, so there the
// renderer works as before. Through it, parts report their events and the
// backend's state (a part hidden, disabled or renamed by a handler) reaches
// the parts.

import type { Shape } from "./drawing";
import { getContext, setContext } from "svelte";
import type { Event as PartEvent } from "./generated/Event";
import type { ShownText } from "./format";
import type { Place } from "./customParts";
import type { JsonValue } from "./generated/serde_json/JsonValue";

const LIVE = Symbol("clarity-live");

/** A pointer's event on a canvas, in the canvas's units. */
/** Where a canvas was clicked or dragged, in its units; a drag also where it was pressed. */
export type Pointer = { x: number; y: number; fromX?: number; fromY?: number; bent?: string; via?: number[] };

/** What a handler set on a part; anything unset keeps the design. */
export type PartState = {
  visible?: boolean;
  enabled?: boolean;
  caption?: ShownText | null;
  /** A message at an input; empty when there is none. */
  error?: ShownText | null;
  readonly?: boolean;
  /** What an unbound text input holds. */
  text?: string | null;
  /** Whether an unbound checkbox or switch is on. */
  checked?: boolean | null;
  /**
   * The pointer's last event on a canvas: where, in its own units
   * (conformance/drawing.json). A record that grows: more about the
   * pointer can join x and y.
   */
  pointer?: Pointer | null;
  /** A canvas's drawing: the shapes its draw handler sent (conformance/drawing.json). */
  drawing?: Shape[] | null;
  /** A canvas's grid in its units (0 for none) and whether code snaps to it, as a handler set them; none keeps the design. */
  grid?: number | null;
  snap?: boolean | null;
  /** What a canvas's clear() fills it with, as a handler set it; empty is the theme's panel. */
  background?: string | null;
  /** A canvas's grid dots, as a handler set them: pixels across, and color (empty is the theme's border). */
  dotSize?: number | null;
  dotColor?: string | null;
  /** Whether a foldable panel or form is folded, as the person or a handler left it (conformance/folding.json). */
  folded?: boolean | null;
  /** Whether a collapsible sidebar is folded to its button (conformance/sidebars.json). */
  collapsed?: boolean | null;
  /** Whether a maximizable sidebar fills the window. */
  maximized?: boolean | null;
  /** The open tab of a ribbon, counted from 1 (conformance/ribbons.json). */
  tab?: number | null;
  /** The step a wizard (a steps part) is on, counted from 1 (conformance/steps.json). */
  step?: number | null;
  /** The open section of an accordion, counted from 1, 0 for none (conformance/accordions.json). */
  open?: number | null;
  /** A custom part's properties as code set them, by name (conformance/custom-parts.json). */
  values?: Record<string, JsonValue> | null;
};

export interface Live {
  /** Part properties by part name. */
  parts: Record<string, PartState>;
  /** The selected row of each table or list, by part name. */
  selected: Record<string, number>;
  /** A part's event, wired to `handler`; false when the handler refused or failed. */
  event(part: string, event: PartEvent, handler: string): Promise<boolean>;
  /** A bound input changed a path of the window's data, for undo (DESIGN 5.29). */
  edited?(part: string, path: string, before: JsonValue | undefined, after: JsonValue | undefined): void;
  select(part: string, row: number): void;
  /** Keeps what the user typed in an unbound text input, for the next event. */
  setText(part: string, text: string): void;
  /** Keeps whether an unbound checkbox or switch is on, for the next event. */
  setChecked(part: string, checked: boolean): void;
  /** Keeps the pointer's event on a canvas, for its handler. */
  setPointer(part: string, pointer: Pointer): void;
  /** Keeps whether the person folded a foldable part, sent with the next event. */
  setFolded(part: string, folded: boolean): void;
  /** Keeps whether the person folded a sidebar or let it fill the window, sent with the next event. */
  setSidebar(part: string, change: { collapsed?: boolean; maximized?: boolean }): void;
  /** Keeps the tab the person opened on a ribbon, and whether it is folded, sent with the next event. */
  setRibbon(part: string, change: { tab?: number; collapsed?: boolean }): void;
  /** Keeps the step a wizard is on, sent with the next event. */
  setStep(part: string, step: number): void;
  /** Keeps the section the person opened in an accordion (0: none), sent with the next event. */
  setOpen(part: string, open: number): void;
  /** Keeps whether a part shows: a popover the person opened or closed, sent with the next event. */
  setVisible(part: string, visible: boolean): void;
  /** Where a custom part is in the webview and whether it shows: the shell keeps its native window there (DESIGN 6.5). */
  placePart(part: string, kind: string, place: Place, shown: boolean): void;
  /** Gives a custom part one of its values: a property, `value` for what it binds, or `enabled`. */
  setPartValue(part: string, name: string, value: JsonValue): void;
  /**
   * A canvas an engine beside the screen draws (DESIGN 5.12) shows: what
   * handlers tell it reaches `hear`, those told before it showed first.
   * `tell` runs the handler wired to what the engine told, with its values;
   * `stop` when the canvas goes.
   */
  screen(part: string, hear: (hear: string, values: JsonValue[]) => void): { tell(tell: string, values: JsonValue[]): void; stop(): void };
  /** The person changed a setting (DESIGN 5.17) through an input bound to it: the shell keeps it. */
  keepSettings(): void;
  /**
   * The keys a command answers (DESIGN 5.28): the person's where the project
   * lets them set keys, else the design's. A command of the app's frame is
   * known by its name, a window's own by the window's name and its name.
   */
  keysOf(part: string | null, designed: string | null, appFrame: boolean): string[];
  /** The records a reference field can point at, as id and display text, with the fields a search dropdown shows. */
  refOptions(entity: string): RefOption[];
  /** The values a text field already has, which a typed dropdown offers (U1c). */
  fieldValues(entity: string, field: string): string[];
  /**
   * A record's stored picture as a URL to show, fetched on its own:
   * undefined while it loads, null when there is none.
   */
  picture(entity: string, id: number, field: string): string | null | undefined;
  /** A file of a store as a URL to show: a picture linked before its record is saved. */
  storeFile(store: string, path: string): string | null | undefined;
  /** Forgets a fetched picture that a new one replaces. */
  forgetPicture(entity: string, id: number, field: string): void;
  /** A record's stored file, fetched on its own, as base64; null when there is none. */
  fileData(entity: string, id: number, field: string): Promise<string | null>;
  /** Opens a file in the program the system uses for its kind. */
  openFile(name: string, data: string): Promise<void>;
  /** Saves a copy of a file in the person's Downloads folder, and says so. */
  saveFile(name: string, data: string): Promise<void>;
  /** Opens the system's folder picker with a title: the folder chosen, or null. None where there is no picker, as in a browser. */
  chooseFolder?: (title: string) => Promise<string | null>;
}

export function provideLive(live: Live) {
  setContext(LIVE, live);
}

export function useLive(): Live | undefined {
  return getContext<Live | undefined>(LIVE);
}
