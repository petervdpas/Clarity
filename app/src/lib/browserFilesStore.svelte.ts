// Clarity.UI's browser files for the whole IDE: how they stand (asked of
// the backend whenever a canvas cannot draw), and `reload`, after the
// browser files in use changed (a cDdev swapped them, Clarity settings >
// Swap): `generation` grows and every canvas loads again. Clarity never
// builds them.

import { invoke } from "@tauri-apps/api/core";
import type { BrowserFiles } from "./browserFiles";
import { resetPageScript } from "./clarityWindow";

class BrowserFilesStore {
  /** Grows each time the files in use changed: the canvases load again. */
  generation = $state(0);

  /** How the files stand now; fine when the backend cannot say. */
  async check(): Promise<BrowserFiles> {
    const said = await invoke<BrowserFiles | null>("browser_files").catch(() => null);
    return said?.state ? said : { state: "fine", repository: false };
  }

  /** The files in use changed: every canvas loads them again. */
  reload(): void {
    resetPageScript();
    this.generation += 1;
  }
}

export const browserFiles = new BrowserFilesStore();
