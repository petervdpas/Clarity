// A folder part (X2): a folder setting's path, typed, or chosen with the
// system's folder picker through its button, named in Clarity's own word
// (chooseFolder). Where nothing opens a picker, as in a browser, the path is
// only typed. The same as on Avalonia.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import FolderInput from "./FolderInput.svelte";

function folder(value: string, choose?: (title: string) => Promise<string | null>) {
  const holder = document.createElement("div");
  document.body.append(holder);
  const changes: string[] = [];
  const part = mount(FolderInput, { target: holder, props: { value, words: { choose: "Map kiezen" }, choose, onchange: (next: string) => changes.push(next) } });
  flushSync();
  const input = holder.querySelector("input")!;
  const button = holder.querySelector("button");
  return { input, button, changes, done: () => (unmount(part), holder.remove()) };
}

test("a typed folder is a change", () => {
  const f = folder("photos");
  f.input.value = "/mnt/photos";
  f.input.dispatchEvent(new Event("change", { bubbles: true }));
  expect(f.changes).toEqual(["/mnt/photos"]);
  f.done();
});

test("the button opens the picker and the chosen folder is a change at once", async () => {
  const asked: string[] = [];
  const f = folder("photos", async (title) => (asked.push(title), "/home/ada/Pictures"));
  expect(f.button?.getAttribute("aria-label")).toBe("Map kiezen");
  f.button!.click();
  await Promise.resolve();
  await Promise.resolve();
  expect(asked).toEqual(["Map kiezen"]);
  expect(f.changes).toEqual(["/home/ada/Pictures"]);
  f.done();
});

test("a cancelled picker changes nothing", async () => {
  const f = folder("photos", async () => null);
  f.button!.click();
  await Promise.resolve();
  await Promise.resolve();
  expect(f.changes).toEqual([]);
  f.done();
});

test("without a picker there is no button: the path is typed", () => {
  const f = folder("photos");
  expect(f.button).toBeNull();
  f.done();
});
