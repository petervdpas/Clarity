// Clarity.UI's browser files for the whole IDE: how they stand (asked of
// the backend when the IDE starts and whenever a canvas cannot draw), and
// Rebuild, which asks first and builds them in the Output panel. When the
// build ends well, `generation` grows and every canvas loads again.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BuildDone } from "@clarity/ui/generated/BuildDone.ts";
import { BROWSER_FILES_TITLE, type BrowserFiles, type BrowserFilesNotice } from "./browserFiles";
import { resetPageScript } from "./clarityWindow";
import { dialogs } from "./dialogStore.svelte";

class BrowserFilesStore {
  /** Grows each time the files were built again: the canvases load again. */
  generation = $state(0);
  /** A rebuild runs. */
  rebuilding = $state(false);
  /** Why the last rebuild did not start or did not build. */
  failure = $state<string | null>(null);

  /** How the files stand now; fine when the backend cannot say. */
  async check(): Promise<BrowserFiles> {
    const said = await invoke<BrowserFiles | null>("browser_files").catch(() => null);
    return said?.state ? said : { state: "fine", repository: false };
  }

  /** Asks (Clarity's own dialog) whether to build the files again, and does on yes. */
  async offer(notice: BrowserFilesNotice): Promise<void> {
    if (!notice.rebuild || this.rebuilding) return;
    if (await dialogs.confirm({ text: notice.text, title: BROWSER_FILES_TITLE, action: "Rebuild" })) await this.rebuild();
  }

  /** Builds the files again in the Output panel; when that ends well, every canvas loads again. */
  async rebuild(): Promise<void> {
    this.rebuilding = true;
    this.failure = null;
    // Only one build runs at a time, so the next end of a build is this one's.
    let ended: (done: BuildDone) => void = () => {};
    const done = new Promise<BuildDone>((resolve) => (ended = resolve));
    const unlisten = await listen<BuildDone>("build-done", (e) => ended(e.payload));
    try {
      await invoke("rebuild_browser_files");
      const result = await done;
      if (result.ok) {
        resetPageScript();
        this.generation += 1;
      } else {
        this.failure = result.message;
      }
    } catch (err) {
      this.failure = String(err);
    } finally {
      unlisten();
      this.rebuilding = false;
    }
  }
}

export const browserFiles = new BrowserFilesStore();
