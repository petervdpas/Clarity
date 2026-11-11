// The IDE's icons: every one is a Lucide icon from Clarity's one icon set
// (crates/clarity-spec/icons/lucide.json, the set generated apps draw
// from too), drawn by the one Icon part, so the ribbon, the file browser
// and every other place look alike. A place names an icon by the IDE's
// word for it; this map says which Lucide icon that is, written out whole.

export const IDE_ICONS = {
  new: "file-plus",
  open: "folder-open",
  reload: "rotate-cw",
  settings: "settings",
  window: "app-window",
  app: "layout-panel-top",
  check: "check",
  undo: "undo-2",
  redo: "redo-2",
  delete: "trash",
  grid: "grid-3x3",
  save: "save",
  key: "key-round",
  link: "link",
  edit: "pencil",
  copy: "copy",
  tag: "tag",
  lock: "lock",
  back: "arrow-left",
  text: "type",
  code: "code",
  build: "hammer",
  play: "play",
  stop: "square",
  globe: "globe",
  up: "arrow-up",
  home: "house",
} as const;

export type IdeIconName = keyof typeof IDE_ICONS;

/** Each IDE icon's path, from the icon set's paths by Lucide name. */
export function idePaths(set: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(IDE_ICONS).map(([word, lucide]) => [word, set[lucide] ?? ""]));
}
