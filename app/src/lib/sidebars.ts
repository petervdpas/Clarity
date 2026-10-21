// A sidebar docked to a window's edge (conformance/sidebars.json): the width
// the person drags it to, within its limits, and whether it is folded to its
// button or fills the window, as the person or a handler left it.

/** The widths a sidebar may have (clarity_spec::model::SIDEBAR_WIDTHS). */
export const NARROWEST = 100;
export const WIDEST = 1600;
/** The width of a folded sidebar: room for its button. */
export const STRIP_WIDTH = 28;

/** The width a drag to `dragged` pixels gives: within the limits, in whole pixels. */
export function sidebarWidth(dragged: number, min: number | null, max: number | null): number {
  return Math.round(Math.min(max ?? WIDEST, Math.max(min ?? NARROWEST, dragged)));
}

