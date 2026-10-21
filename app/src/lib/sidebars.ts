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


/** How the person left a sidebar: folded or filling, and whether it started folded then. */
export type SidebarHere = { collapsed?: boolean; maximized?: boolean; designed?: boolean };

/**
 * Whether a sidebar is folded and fills the window now: as a handler set it,
 * else as the person left it, else as designed. What the person did counts
 * only while the design still starts it as it did then, so the designer
 * shows a change to "starts folded" at once.
 */
export function sidebarNow(
  props: { collapsible: boolean; collapsed: boolean; maximizable: boolean },
  part: { collapsed?: boolean | null; maximized?: boolean | null } | undefined,
  here: SidebarHere | undefined,
): { collapsed: boolean; maximized: boolean } {
  const own = here && here.designed === props.collapsed ? here : undefined;
  return {
    collapsed: props.collapsible && (part?.collapsed ?? own?.collapsed ?? props.collapsed),
    maximized: props.maximizable && (part?.maximized ?? own?.maximized ?? false),
  };
}
