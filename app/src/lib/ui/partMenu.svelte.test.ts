// A part's context menu (DESIGN 5.16), run through the real PartMenu: the
// items it shows, the greyed ones, and what a click and Escape do.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import type { Command } from "../generated/Command";
import PartMenu from "./PartMenu.svelte";

const span = (start: number) => ({ start, end: start + 1, line: 1, col: 1 });
const item = (start: number, caption: string, name: string | null, handler: string | null = null): Command =>
  ({ props: { kind: "item", caption, name, handler, onClick: null, newWindow: false, icon: null, shortcut: null, large: false }, items: [], span: span(start) }) as unknown as Command;
const separator = (start: number): Command => ({ props: { kind: "separator", caption: null, name: null }, items: [], span: span(start) }) as unknown as Command;

const ITEMS = [item(1, "Open", "openItem", "openClicked"), separator(2), item(3, "Delete", "deleteItem", "deleteClicked"), item(4, "Secret", "secretItem", "secretClicked")];

function opened(states: Record<string, { visible?: boolean; enabled?: boolean }>) {
  const ran: string[] = [];
  let closed = 0;
  const target = document.createElement("div");
  document.body.append(target);
  const menu = mount(PartMenu, {
    target,
    props: {
      items: ITEMS,
      x: 10,
      y: 20,
      caption: (text: string) => text.toUpperCase(),
      shows: (name: string | null) => !name || states[name]?.visible !== false,
      enabled: (name: string | null) => !name || (states[name]?.visible !== false && states[name]?.enabled !== false),
      onrun: (c: Command) => ran.push(c.props.handler ?? ""),
      onclose: () => closed++,
    },
  });
  flushSync();
  const buttons = () => [...target.querySelectorAll<HTMLButtonElement>("button")];
  return { target, buttons, ran, closed: () => closed, done: () => (unmount(menu), target.remove()) };
}

test("a context menu shows its items, leaves out the hidden and greys the disabled", () => {
  const menu = opened({ secretItem: { visible: false }, deleteItem: { enabled: false } });
  expect(menu.buttons().map((b) => b.textContent?.trim())).toEqual(["OPEN", "DELETE"]);
  expect(menu.buttons().map((b) => b.disabled)).toEqual([false, true]);
  expect(menu.target.querySelectorAll("[role=separator]").length).toBe(1);
  menu.buttons()[1].click();
  expect(menu.ran).toEqual([]);
  menu.done();
});

test("a click runs the item and closes the menu; Escape closes it", async () => {
  const menu = opened({});
  menu.buttons()[0].click();
  expect(menu.ran).toEqual(["openClicked"]);
  expect(menu.closed()).toBe(1);
  await new Promise((done) => setTimeout(done, 0));
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
  expect(menu.closed()).toBe(2);
  menu.done();
});
