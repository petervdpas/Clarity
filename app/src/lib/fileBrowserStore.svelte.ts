// Clarity's own file browser for the whole app (docs/PROJECT-BROWSER-PLAN.md),
// in place of the system's file and folder windows: `browse({...})` shows it
// (FileBrowser.svelte, mounted once in App) and gives the path chosen, or
// null on Cancel, as Clarity's dialogs module does for its dialogs.

/** What a use picks: a project to open, a folder, an existing file of `kinds`, or a new file's place and name. */
export type BrowsePick = "project" | "folder" | "file" | "save";

export type BrowseRequest = {
  /** The use, so the browser starts where this use was left last time: `open-project`, `app-folder`, `openapi`... */
  purpose: string;
  title: string;
  pick: BrowsePick;
  /** For `file` and `save`: the kinds of file, by extension without the dot. */
  kinds?: string[];
  /** The folder to start in the first time (the one the caller shows now), when there is one. */
  start?: string | null;
  /** For `save`: the name to suggest. */
  name?: string;
  /** For `project`: "New project here", given the folder the browser shows. */
  newProject?: (folder: string) => void;
};

class FileBrowserStore {
  current = $state<BrowseRequest | null>(null);
  #resolve: ((path: string | null) => void) | null = null;

  /** Shows the browser; the path chosen, or null when cancelled. One at a time: a new one cancels the one shown. */
  browse(request: BrowseRequest): Promise<string | null> {
    this.#resolve?.(null);
    this.current = request;
    return new Promise((resolve) => (this.#resolve = resolve));
  }

  /** The browser's answer. */
  answer(path: string | null) {
    const resolve = this.#resolve;
    this.#resolve = null;
    this.current = null;
    resolve?.(path);
  }
}

export const fileBrowser = new FileBrowserStore();

/** Shows Clarity's file browser; the path chosen, or null. */
export function browse(request: BrowseRequest): Promise<string | null> {
  return fileBrowser.browse(request);
}
