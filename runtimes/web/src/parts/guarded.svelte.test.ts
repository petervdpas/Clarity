// The IDE's main area is guarded: a part that fails while it is drawn shows
// the error in its place, and choosing something else draws it afresh.

import { createRawSnippet, flushSync, mount, unmount } from "svelte";
import { expect, test, vi } from "vitest";
import Guarded from "./Guarded.svelte";

test("an error while drawing shows in place of the part, and another choice draws afresh", () => {
  vi.spyOn(console, "error").mockImplementation(() => {});
  const state = $state({ shows: "window Broken" });
  const children = createRawSnippet(() => ({
    render: () => "<p>drawn</p>",
    setup: () => {
      if (state.shows === "window Broken") throw new Error("two parts have the same key");
    },
  }));
  const target = document.createElement("div");
  const shown = mount(Guarded, {
    target,
    props: {
      get shows() {
        return state.shows;
      },
      children,
    },
  });
  flushSync();
  expect(target.querySelector("[role=alert]")?.textContent).toContain("two parts have the same key");
  state.shows = "window Fine";
  flushSync();
  expect(target.querySelector("[role=alert]")).toBeNull();
  expect(target.textContent).toContain("drawn");
  unmount(shown);
});
