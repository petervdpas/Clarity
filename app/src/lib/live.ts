// The connection between the renderer and a running application. The
// generated app sets it; the designer and the preview do not, so there the
// renderer works as before. Through it, parts report their events and the
// backend's state (a part hidden, disabled or renamed by a handler) reaches
// the parts.

import type { Shape } from "./drawing";
import { getContext, setContext } from "svelte";
import type { Event as PartEvent } from "./generated/Event";
import type { ShownText } from "./format";

const LIVE = Symbol("clarity-live");

/** A pointer's event on a canvas, in the canvas's units. */
export type Pointer = { x: number; y: number };

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
};

export interface Live {
  /** Part properties by part name. */
  parts: Record<string, PartState>;
  /** The selected row of each table or list, by part name. */
  selected: Record<string, number>;
  /** A part's event, wired to `handler`. */
  event(part: string, event: PartEvent, handler: string): void;
  select(part: string, row: number): void;
  /** Keeps what the user typed in an unbound text input, for the next event. */
  setText(part: string, text: string): void;
  /** Keeps whether an unbound checkbox or switch is on, for the next event. */
  setChecked(part: string, checked: boolean): void;
  /** Keeps the pointer's event on a canvas, for its handler. */
  setPointer(part: string, pointer: Pointer): void;
  /** The records a reference field can point at, as id and display text. */
  refOptions(entity: string): { id: number; text: string }[];
  /**
   * A record's stored picture as a URL to show, fetched on its own:
   * undefined while it loads, null when there is none.
   */
  picture(entity: string, id: number, field: string): string | null | undefined;
  /** A file of a store as a URL to show: a picture linked before its record is saved. */
  storeFile(store: string, path: string): string | null | undefined;
  /** Forgets a fetched picture that a new one replaces. */
  forgetPicture(entity: string, id: number, field: string): void;
}

export function provideLive(live: Live) {
  setContext(LIVE, live);
}

export function useLive(): Live | undefined {
  return getContext<Live | undefined>(LIVE);
}
