// A password input shows what is typed or hides it again, and copies it,
// with its buttons named in Clarity's own words (showSecret, hideSecret,
// copySecret), the same as on Avalonia.

import { flushSync, mount, unmount } from "svelte";
import { expect, test, vi } from "vitest";
import TextInput from "./TextInput.svelte";

const words = { show: "Tonen", hide: "Verbergen", copy: "Kopiëren" };

function password(value: string) {
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(TextInput, { target: holder, props: { kind: "password", label: "Token", value, words, onchange: () => {} } });
  flushSync();
  const input = holder.querySelector("input")!;
  const button = (name: string) => [...holder.querySelectorAll("button")].find((b) => b.getAttribute("aria-label") === name);
  return { holder, input, button, done: () => (unmount(part), holder.remove()) };
}

test("the eye shows what is typed and hides it again", () => {
  const p = password("s3cr3t");
  expect(p.input.type).toBe("password");
  const eye = p.button("Tonen")!;
  expect(eye.getAttribute("aria-pressed")).toBe("false");
  eye.click();
  flushSync();
  expect(p.input.type).toBe("text");
  // Pressed, it says what it does next.
  const shut = p.button("Verbergen")!;
  expect(shut.getAttribute("aria-pressed")).toBe("true");
  shut.click();
  flushSync();
  expect(p.input.type).toBe("password");
  p.done();
});

test("copy puts what is in the input on the clipboard, typed or not yet taken", async () => {
  const writeText = vi.fn(async () => {});
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  const p = password("s3cr3t");
  p.input.value = "typed";
  p.button("Kopiëren")!.click();
  await Promise.resolve();
  expect(writeText).toHaveBeenCalledWith("typed");
  p.done();
});

test("a text input has neither", () => {
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(TextInput, { target: holder, props: { value: "x", words, onchange: () => {} } });
  flushSync();
  expect(holder.querySelectorAll("button").length).toBe(0);
  unmount(part);
  holder.remove();
});
