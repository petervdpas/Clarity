// What the grid and its parts need of a designer: what is selected, what is
// being dragged and where it would land, the tab shown in each container,
// and the designer's actions (DESIGN 7.8, G6a). The IDE provides its page
// designer; a running application has none, and gets the idle one, which
// still keeps the tab each container shows.

import { getContext, setContext } from "svelte";
import type { Dock } from "../generated/Dock.ts";
import type { Edit } from "../generated/Edit.ts";
import type { RegionKind } from "../generated/RegionKind.ts";
import type { WidgetKind } from "../generated/WidgetKind.ts";
import type { WidgetProps } from "../generated/WidgetProps.ts";

export type PageDragSource =
  | { kind: "new"; widget: WidgetKind; width: number; height: number; label: string; part?: string }
  | { kind: "field"; widget: WidgetKind; bind: string; caption: string; width: number; height: number; label: string }
  | { kind: "move"; item: number; from: number; props: WidgetProps; grab: { dc: number; dr: number }; label: string }
  | { kind: "resize"; item: number; props: WidgetProps; label: string }
  | { kind: "region"; region: RegionKind; docks: Dock[]; label: string };

/** Where a drop would land: a container and the rectangle in its grid. */
export type Placement = { container: number; columns: number; col: number; row: number; width: number; height: number };

export interface Designer {
  /** The element selected, by its offset in its file. */
  selected: number | null;
  /** The tab, step or section each container shows, by the container's offset. */
  activeTabs: Record<number, number>;
  /** Show the snap grid's cells. */
  readonly showGrid: boolean;
  /** What is being dragged, and where it would land. */
  readonly drag: PageDragSource | null;
  readonly hover: Placement | null;
  /** A press on a part: a drag once the pointer moves, else a click. */
  press(e: PointerEvent, source: PageDragSource, onclick?: () => void): void;
  /** An edit of the design. */
  edit(edit: Edit): void;
  /** Opens the code behind a part. */
  openPartCode(item: number): void;
}

/** The designer of a running application: nothing to design, only the tabs shown. */
class Idle implements Designer {
  selected = $state<number | null>(null);
  activeTabs = $state<Record<number, number>>({});
  readonly showGrid = false;
  readonly drag = null;
  readonly hover = null;
  press(_e: PointerEvent, _source: PageDragSource, onclick?: () => void) {
    onclick?.();
  }
  edit() {}
  openPartCode() {}
}

const idle = new Idle();
const KEY = Symbol("clarity.designer");

/** Makes `designer` the one the grids and parts below use. */
export function provideDesigner(designer: Designer) {
  setContext(KEY, designer);
}

/** The designer above, or the idle one in a running application. */
export function useDesigner(): Designer {
  return getContext<Designer | undefined>(KEY) ?? idle;
}
