// The keys of a running window's frame (DESIGN 5.28): its menu items and its
// toolbar and ribbon buttons with a key, while they show and take clicks. A
// ribbon's keys work whichever tab is open; what code hid or disabled, and
// what is inside it, does not answer.

import type { ShownFrame } from "./frame";
import type { Command } from "./generated/Command";

/** The keys a command answers: the design's, or the person's (DESIGN 5.28); `appFrame` for one of the app's frame. */
export type KeysOf = (command: Command, appFrame: boolean) => string[];

/** The design's keys of a command. */
export const designedKeys: KeysOf = (c) => (c.props.shortcut ? [c.props.shortcut] : []);

/** The frame's commands with keys, in the order they are written: the menu, the toolbar, the ribbon. */
export function keyedCommands(
  frame: ShownFrame,
  shows: (name: string | null) => boolean,
  enabled: (name: string | null) => boolean,
  keysOf: KeysOf = designedKeys,
): { command: Command; keys: string[] }[] {
  const found: { command: Command; keys: string[] }[] = [];
  const walk = (items: Command[], appFrame: boolean) => {
    for (const c of items) {
      if (c.props.kind === "separator" || !shows(c.props.name) || !enabled(c.props.name)) continue;
      const keys = keysOf(c, appFrame);
      if (keys.length) found.push({ command: c, keys });
      walk(c.items, appFrame);
    }
  };
  const menu = frame.menu;
  if (menu && shows(menu.value.name) && enabled(menu.value.name)) walk(menu.value.items, menu.inherited);
  const toolbar = frame.toolbar;
  if (toolbar && shows(toolbar.value.name) && enabled(toolbar.value.name)) walk(toolbar.value.items, toolbar.inherited);
  const ribbon = frame.ribbon;
  if (ribbon && shows(ribbon.value.name) && enabled(ribbon.value.name)) {
    for (const tab of ribbon.value.tabs.filter((t) => shows(t.name))) {
      for (const group of tab.groups.filter((g) => shows(g.name))) walk(group.items, ribbon.inherited);
    }
  }
  return found;
}
