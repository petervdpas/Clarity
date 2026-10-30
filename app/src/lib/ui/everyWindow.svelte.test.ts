// A guard for the IDE: every window of every example opens in the designer
// and in the preview without an error, as the IDE gets them
// (`clarity spec model --samples`). A part the designer cannot draw, such as
// two parts with the same key, fails here and not in front of the person.

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { flushSync, mount, unmount, type Component } from "svelte";
import { expect, test, vi } from "vitest";
import { findApp } from "../frame";
import type { Preview } from "../generated/Preview";
import PageDesigner from "../PageDesigner.svelte";
import { appAsPage } from "../pages";
import PageView from "../PageView.svelte";

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

/** The project as the IDE gets it, from the command line Clarity the build made, or else built now. */
function previewOf(example: string): Preview {
  const built = join(root, "target/debug/clarity");
  const project = join(root, "examples", `${example}.clarity`);
  const args = ["spec", "model", "--samples", project];
  const json = existsSync(built)
    ? execFileSync(built, args, { maxBuffer: 1 << 28 })
    : execFileSync("cargo", ["run", "-q", "-p", "clarity-cli", "--", ...args], { cwd: root, maxBuffer: 1 << 28, env: { ...process.env, CARGO_BUILD_JOBS: "2" } });
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
  test(`every window of ${example} opens in the designer and the preview`, () => {
    const preview = previewOf(example);
    const app = findApp(preview);
    for (const file of preview.project.files) {
      for (const page of file.pages) {
        draws(PageDesigner as Component<never>, { preview, page, file, onapplied: () => {} });
        draws(PageView as Component<never>, { preview, page, onnavigate: () => {} });
      }
      if (app && file.apps.some((a) => a.name.name === app.name.name)) {
        draws(PageDesigner as Component<never>, { preview, page: appAsPage(app), app, file, onapplied: () => {} });
      }
    }
  }, 120_000);
}
