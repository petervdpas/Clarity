// Forgetting Clarity.UI's scripts after its browser files were built again:
// the page script, the worker script and the library all load again, so an
// old library never starts the new wasm ("Out of bounds call_indirect").

import { test } from "node:test";
import assert from "node:assert/strict";

type Fake = { src: string; removed: boolean; remove(): void };

test("resetPageScript forgets the page script, the worker script and the library", async () => {
  const scripts: Fake[] = ["clarity-ui-page.js", "clarity-ui-worker.js", "clarity-ui.js", "other.js"].map((name) => ({
    src: `http://127.0.0.1:9/${name}`,
    removed: false,
    remove() {
      this.removed = true;
    },
  }));
  const page = { ClarityUiPage: {}, ClarityUiPageWorker: () => ({}) } as Record<string, unknown>;
  // The library's global, as Emscripten's `var ClarityUi` makes it: it cannot be deleted.
  Object.defineProperty(page, "ClarityUi", { value: () => ({}), writable: true, configurable: false, enumerable: true });
  (globalThis as Record<string, unknown>).window = page;
  (globalThis as Record<string, unknown>).document = {
    querySelectorAll: (selector: string) => {
      const end = /\$="([^"]+)"/.exec(selector)?.[1] ?? "";
      return scripts.filter((s) => s.src.endsWith(`/${end}`));
    },
  };
  const { resetPageScript } = await import("./clarityWindow.ts");
  resetPageScript();
  assert.equal(page.ClarityUiPage, undefined);
  assert.equal(page.ClarityUiPageWorker, undefined);
  assert.equal(page.ClarityUi, undefined, "a global that cannot be deleted is blanked");
  assert.deepEqual(
    scripts.filter((s) => s.removed).map((s) => s.src.split("/").pop()),
    ["clarity-ui-page.js", "clarity-ui-worker.js", "clarity-ui.js"],
  );
});
