// What Clarity's file browser gives back for each use (docs/PROJECT-BROWSER-PLAN.md),
// apart from the window, so it is tested on its own.

import type { ProjectInfo } from "@clarity/ui/generated/ProjectInfo.ts";
import type { BrowsePick } from "./fileBrowserStore.svelte";

/** What the choosing needs of a listed entry. */
export type Pickable = { path: string; folder: boolean; project: ProjectInfo | null; projects: ProjectInfo[] };

/**
 * The path the browser gives back, or null while nothing fits:
 * - project: a project file picked, or a folder picked that holds exactly one;
 * - folder: the folder picked, else the folder shown;
 * - file: a file picked;
 * - save: `name` in the folder shown, with one of `kinds`.
 */
export function chosen(pick: BrowsePick, selected: Pickable | null, folder: string, name: string, kinds: string[]): string | null {
  switch (pick) {
    case "project":
      if (selected?.project) return selected.project.file;
      if (selected?.folder && selected.projects.length === 1) return selected.projects[0].file;
      return null;
    case "folder":
      return selected?.folder ? selected.path : folder || null;
    case "file":
      return selected && !selected.folder ? selected.path : null;
    case "save": {
      const trimmed = name.trim();
      if (!trimmed || /[\\/]/.test(trimmed) || !folder) return null;
      return joinPath(folder, withKind(trimmed, kinds));
    }
  }
}

/** `name` with the first of `kinds` unless it ends in one of them already (any case). */
export function withKind(name: string, kinds: string[]): string {
  if (kinds.length === 0) return name;
  const lower = name.toLowerCase();
  return kinds.some((k) => lower.endsWith("." + k.toLowerCase())) ? name : `${name}.${kinds[0]}`;
}

/** `name` in `folder`, with the folder's own separator (a Windows path keeps its backslash). */
export function joinPath(folder: string, name: string): string {
  const separator = folder.includes("\\") && !folder.includes("/") ? "\\" : "/";
  return folder.endsWith(separator) ? folder + name : folder + separator + name;
}
