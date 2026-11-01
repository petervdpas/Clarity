// A generated window draws what the model draws (DESIGN 7.8, G6b): every
// window of every example, written as markup by `clarity spec windows`, is
// drawn with the same page as the screen draws it from the model. The places
// parts have in their files are left out of the comparison: markup has none.

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { flushSync, mount, unmount, type Component } from "svelte";
import { expect, test, vi } from "vitest";
import GridBox from "../grid/GridBox.svelte";
import WindowPlace from "./WindowPlace.svelte";
import type { Preview } from "../generated/Preview.ts";
import { pageData } from "../screen/pages.ts";

vi.mock("@tauri-apps/api/core", () => ({ invoke: async () => null }));
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
} as never;

const root = join(import.meta.dirname, "../../../..");
const clarity = join(root, "target/debug/clarity");
// Every example, and the every-part project, which has popovers and part menus.
const projects = [
  ...readdirSync(join(root, "examples"))
    .filter((name) => name.endsWith(".clarity"))
    .map((name) => ({ name: name.slice(0, -".clarity".length), file: join(root, "examples", name) }))
    .sort((a, b) => a.name.localeCompare(b.name)),
  { name: "everything", file: join(root, "crates/clarity-gen/tests/fixtures/everything") },
];

function run(args: string[]): Buffer {
  return existsSync(clarity)
    ? execFileSync(clarity, args, { maxBuffer: 1 << 28 })
    : execFileSync("cargo", ["run", "-q", "-p", "clarity-cli", "--", ...args], { cwd: root, maxBuffer: 1 << 28, env: { ...process.env, CARGO_BUILD_JOBS: "2" } });
}

/** The parts as they are, without their places in their files, what the project works out itself, and the sounds and timers markup leaves out. */
function parts(widgets: unknown): unknown {
  return JSON.parse(
    JSON.stringify(widgets, (key, value) => (["span", "made", "undoScope"].includes(key) ? undefined : value)),
    (key, value) => (Array.isArray(value) ? value.filter((w) => !["sound", "timer"].includes(w?.props?.kind)) : value),
  );
}

/** What a window drew, without the places parts have in their files. */
function drawn(part: Component<never>, props: Record<string, unknown>): string {
  const target = document.createElement("div");
  document.body.append(target);
  const shown = mount(part, { target, props: props as never });
  flushSync();
  flushSync();
  const html = target.innerHTML
    .replace(/<!--[^>]*-->/g, "")
    .replace(/ data-widget="\d+"/g, "")
    .replace(/ data-grid="\d+"/g, "")
    .replace(/data-tab-switch="\d+:/g, 'data-tab-switch="')
    .replace(/radio-\d+/g, "radio")
    // A type-ahead names its list of offers at random.
    .replace(/offered-[a-z0-9]+/g, "offered")
    // A canvas names its grid's pattern at random.
    .replace(/canvas-grid-[a-z0-9]+/g, "canvas-grid")
    // The markup's indentation between elements is no part of the page.
    .replace(/>\s+</g, "><")
    .trim();
  unmount(shown);
  target.remove();
  return html;
}

for (const { name: example, file: project } of projects) {
  test(`every window of ${example} draws the same from its markup`, async () => {
    const preview: Preview = JSON.parse(run(["spec", "model", "--samples", project]).toString());
    const folder = join(root, "target/window-markup", example);
    rmSync(folder, { recursive: true, force: true });
    run(["spec", "windows", project, folder]);
    const written = existsSync(folder) ? readdirSync(folder) : [];
    for (const file of preview.project.files) {
      for (const page of file.pages) {
        if (!written.includes(`${page.name.name}.svelte`)) continue;
        const window = (await import(/* @vite-ignore */ join(folder, `${page.name.name}.svelte`))).default as Component;
        const data = () => structuredClone(pageData(preview, page));
        const fromModel = drawn(GridBox as Component<never>, { preview, page, data: data(), design: false, widgets: page.widgets, columns: page.columns, container: page.span.start, onnavigate: () => {} });
        let made: unknown = null;
        const fromMarkup = drawn(WindowPlace as Component<never>, { window, preview, page, data: data(), onnavigate: () => {}, made: (widgets: unknown) => (made = widgets) });
        // The same parts, their menus and the parts of popovers and pages too.
        expect(parts(made), `${example}: ${page.name.name}`).toEqual(parts(page.widgets));
        if (fromMarkup !== fromModel) {
          // Where they part, enough to see why.
          let at = 0;
          while (at < fromModel.length && fromModel[at] === fromMarkup[at]) at++;
          expect(fromMarkup.slice(Math.max(0, at - 200), at + 200), `${example}: ${page.name.name}`).toBe(fromModel.slice(Math.max(0, at - 200), at + 200));
        }
      }
    }
  });
}
