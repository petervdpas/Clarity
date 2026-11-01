// A list, table or tree in a container (a tab page, a pane, a panel) shows no
// border of its own, its container framing it, unless it has `frame`; one
// beside the window's grid (a sidebar's) keeps its border (DESIGN 7.8).

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { flushSync, mount, unmount, type Component } from "svelte";
import { expect, test, vi } from "vitest";
import GridBox from "./GridBox.svelte";
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

function preview(): Preview {
  const args = ["spec", "model", "--samples", join(root, "crates/clarity-gen/tests/fixtures/everything")];
  const json = existsSync(clarity) ? execFileSync(clarity, args, { maxBuffer: 1 << 28 }) : execFileSync("cargo", ["run", "-q", "-p", "clarity-cli", "--", ...args], { cwd: root, maxBuffer: 1 << 28 });
  return JSON.parse(json.toString());
}

test("a list in a tab page has no border of its own, unless it has frame", () => {
  const model = preview();
  const page = model.project.files.flatMap((f) => f.pages).find((p) => p.name.name === "Gallery")!;
  const draw = (widgets: typeof page.widgets) => {
    const target = document.createElement("div");
    document.body.append(target);
    const shown = mount(GridBox as Component<never>, { target, props: { preview: model, page: { ...page, widgets }, data: pageData(model, page), design: false, widgets, columns: page.columns, container: page.span.start, onnavigate: () => {} } as never });
    flushSync();
    const list = target.querySelector('[data-part="albumList"] ul.list');
    const frameless = list?.classList.contains("frameless");
    unmount(shown);
    target.remove();
    return frameless;
  };
  expect(draw(page.widgets)).toBe(true);
  // The same list with `frame`.
  const framed = JSON.parse(JSON.stringify(page.widgets), (key, value) => (value?.props?.name === "albumList" ? { ...value, props: { ...value.props, frame: true } } : value));
  expect(draw(framed)).toBe(false);
});
