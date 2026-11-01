// How a generated window's markup becomes the parts its screen draws
// (DESIGN 7.8, G6b): each element adds its part to what holds it (the
// window, a container, or a tab, step, section or pane), and the window
// draws the whole of it with the screen's own grid and part modules.

import { getContext, setContext } from "svelte";
import type { Command } from "../generated/Command.ts";
import type { Menu } from "../generated/Menu.ts";
import type { Page } from "../generated/Page.ts";
import type { Preview } from "../generated/Preview.ts";
import type { Widget } from "../generated/Widget.ts";
import type { WidgetPage } from "../generated/WidgetPage.ts";
import type { PageData } from "../screen/pages.ts";

/** What the host gives a window it shows: the project, the window as declared, its data. */
export type WindowPlace = {
  preview: Preview;
  page: Page;
  data: PageData;
  onnavigate: (target: string, newWindow?: boolean) => void;
  /** Told the parts the markup made, once it has made them all. */
  made?: (widgets: Widget[]) => void;
};

const PLACE = Symbol("clarity.window.place");

export function provideWindowPlace(place: WindowPlace) {
  setContext(PLACE, place);
}

export function useWindowPlace(): WindowPlace | undefined {
  return getContext<WindowPlace | undefined>(PLACE);
}

/** Where an element adds its part: the parts here, the pages of a container, and the window's count of what it holds. */
export type Holder = {
  widgets: Widget[];
  pages?: WidgetPage[];
  /** A place of its own for each element, as a part's place in its file names it in the model. */
  next: () => number;
  /** A part's menu, which its `<Menu>` gives it. */
  setMenu?: (menu: Menu) => void;
  /** The commands of a menu or submenu, which its `<Item>`s add to. */
  commands?: Command[];
};

const HOLDER = Symbol("clarity.window.holder");

export function provideHolder(holder: Holder) {
  setContext(HOLDER, holder);
}

export function useHolder(): Holder {
  const holder = getContext<Holder | undefined>(HOLDER);
  if (!holder) throw new Error("a part of a window stands inside its <Window>");
  return holder;
}
