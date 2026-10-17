// The connection between the renderer and a running application. The
// generated app sets it; the designer and the preview do not, so there the
// renderer works as before. Through it, parts report their events and the
// backend's state (a part hidden, disabled or renamed by a handler) reaches
// the parts.

import { getContext, setContext } from "svelte";
import type { Event as PartEvent } from "./generated/Event";

const LIVE = Symbol("clarity-live");

/** What a handler set on a part; anything unset keeps the design. */
export type PartState = {
  visible?: boolean;
  enabled?: boolean;
  caption?: string | null;
  /** A message at an input; empty when there is none. */
  error?: string | null;
  readonly?: boolean;
  /** What an unbound text input holds. */
  text?: string | null;
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
  /** The records a reference field can point at, as id and display text. */
  refOptions(entity: string): { id: number; text: string }[];
}

export function provideLive(live: Live) {
  setContext(LIVE, live);
}

export function useLive(): Live | undefined {
  return getContext<Live | undefined>(LIVE);
}
