// Context menus and popovers (DESIGN 5.16, conformance/overlays.json), apart
// from the screen: whether a popover shows and whether a context menu's item
// shows and takes clicks, by their part state.

import type { PartState } from "./live";

/** A popover shows once opened: closed until its part state says it is visible. */
export function popoverShown(part: PartState | undefined): boolean {
  return part?.visible === true;
}

/** A context menu's item shows unless code hid it. */
export function itemShown(part: PartState | undefined): boolean {
  return part?.visible !== false;
}

/** A context menu's item takes clicks while it shows and code did not disable it. */
export function itemClicks(part: PartState | undefined): boolean {
  return itemShown(part) && part?.enabled !== false;
}
