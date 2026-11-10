// A guard for the IDE: every window of every example opens in the designer
// without an error, as the IDE gets them (`clarity spec model --samples`):
// its lists, inspector and layer. The window itself is drawn by Clarity.UI
// (DESIGN 8.1), whose markup every example is read with in its own tests.

import { execFileSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { flushSync, mount, unmount, type Component } from "svelte";
import { expect, test, vi } from "vitest";
import { findApp } from "@clarity/ui/screen/frame.ts";
import type { Preview } from "@clarity/ui/generated/Preview.ts";
import PageDesigner from "../PageDesigner.svelte";
import { appAsPage } from "@clarity/ui/screen/pages.ts";

vi.mock("@tauri-apps/api/core", () => ({ invoke: async () => null }));
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
} as never;

const root = join(import.meta.dirname, "../../../..");
const examples = readdirSync(join(root, "examples"))
  .filter((name) => name.endsWith(".clarity"))
  .map((name) => name.slice(0, -".clarity".length))
  .sort();

/** The project as the IDE gets it, from the command line Clarity that vitest.cli.setup.ts built. */
function previewOf(example: string): Preview {
  const built = join(root, "target/debug", process.platform === "win32" ? "clarity.exe" : "clarity");
  const project = join(root, "examples", `${example}.clarity`);
  const json = execFileSync(built, ["spec", "model", "--samples", project], { maxBuffer: 1 << 28 });
  return JSON.parse(json.toString());
}

/** Draws a part into the page and takes it away again; an error in drawing fails the test. */
function draws(part: Component<never>, props: Record<string, unknown>) {
  const target = document.createElement("div");
  document.body.append(target);
  const shown = mount(part, { target, props: props as never });
  flushSync();
  expect(target.childElementCount).toBeGreaterThan(0);
  unmount(shown);
  target.remove();
}

for (const example of examples) {
  test(`every window of ${example} opens in the designer`, () => {
    const preview = previewOf(example);
    const app = findApp(preview);
    for (const file of preview.project.files) {
      for (const page of file.pages) {
        draws(PageDesigner as Component<never>, { preview, page, file, onapplied: () => {} });
      }
      if (app && file.apps.some((a) => a.name.name === app.name.name)) {
        draws(PageDesigner as Component<never>, { preview, page: appAsPage(app), app, file, onapplied: () => {} });
      }
    }
  }, 120_000);
}
