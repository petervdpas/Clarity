// A color part on the web: its swatch shows the color picked, as Avalonia's
// does (conformance/colors.json), and a crossed-out swatch when there is none.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import ColorInput from "./ColorInput.svelte";

test("the swatch shows the color picked, and none as none", () => {
  const holder = document.createElement("div");
  document.body.append(holder);
  const props = $state({ value: "#ff0000" as string | null, onchange: () => {} });
  const part = mount(ColorInput, { target: holder, props });
  flushSync();
  const swatch = holder.querySelector(".swatch") as HTMLButtonElement;
  expect(swatch.style.backgroundColor).toBe("rgb(255, 0, 0)");
  props.value = "";
  flushSync();
  expect(swatch.classList.contains("none")).toBe(true);
  unmount(part);
  holder.remove();
});
