// Each part of a window is guarded: one that fails while it is drawn shows
// its name and the reason in its own place, and the parts beside it are drawn.

import { createRawSnippet, flushSync, mount, unmount } from "svelte";
import { expect, test, vi } from "vitest";
import type { Widget } from "../generated/Widget";
import PartGuard from "./PartGuard.svelte";

const part = (name: string): Widget =>
  ({ props: { name, kind: "canvas", col: 3, row: 2, width: 4, height: 5 }, span: { start: 1, end: 2, line: 1, col: 1 }, children: [], pages: [], menu: null }) as unknown as Widget;

test("a part that fails shows its name and the reason in its place; the one beside it is drawn", () => {
  vi.spyOn(console, "error").mockImplementation(() => {});
  const target = document.createElement("div");
  const broken = createRawSnippet(() => ({
    render: () => "<p>never</p>",
    setup: () => {
      throw new Error("no drawing");
    },
  }));
  const fine = createRawSnippet(() => ({ render: () => "<p>drawn</p>" }));
  const a = mount(PartGuard, { target, props: { name: "dialCanvas", place: part("dialCanvas").props, children: broken } });
  const b = mount(PartGuard, { target, props: { name: "nameBox", place: part("nameBox").props, children: fine } });
  flushSync();
  const failed = target.querySelector<HTMLElement>("[role=alert]")!;
  expect(failed.textContent).toBe("dialCanvas: no drawing");
  expect(failed.style.gridColumn).toBe("3 / span 4");
  expect(target.textContent).toContain("drawn");
  unmount(a);
  unmount(b);
});
