// A long engine call on the screen (DESIGN 5.12): a bar of how far it is
// and a Stop button, named in Clarity's own word (stopCall), the same as on
// Avalonia.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import ProgressBar from "./ProgressBar.svelte";

test("the bar says how far the call is, and Stop stops it once", () => {
  const holder = document.createElement("div");
  document.body.append(holder);
  let stopped = 0;
  const part = mount(ProgressBar, { target: holder, props: { done: 30, total: 120, words: { stop: "Stoppen" }, onstop: () => stopped++ } });
  flushSync();
  const bar = holder.querySelector("[role=progressbar]")!;
  expect(bar.getAttribute("aria-valuenow")).toBe("30");
  expect(bar.getAttribute("aria-valuemax")).toBe("120");
  expect((holder.querySelector(".done") as HTMLElement).style.width).toBe("25%");
  const stop = holder.querySelector("button")!;
  expect(stop.textContent).toBe("Stoppen");
  stop.click();
  flushSync();
  stop.click();
  flushSync();
  expect(stopped).toBe(1);
  expect(stop.disabled).toBe(true);
  unmount(part);
  holder.remove();
});

test("a total of none shows an empty bar, not a broken one", () => {
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(ProgressBar, { target: holder, props: { done: 3, total: 0, words: { stop: "Stop" }, onstop: () => {} } });
  flushSync();
  expect((holder.querySelector(".done") as HTMLElement).style.width).toBe("0%");
  unmount(part);
  holder.remove();
});
