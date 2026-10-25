// The keys of a running window's frame (DESIGN 5.28): its menu items and its
// toolbar and ribbon buttons, while they show and take clicks; a ribbon's
// keys work whichever tab is open.

import { test } from "node:test";
import assert from "node:assert/strict";
import { keyedCommands } from "./frameKeys.ts";
import type { ShownFrame } from "./frame.ts";
import type { Command } from "./generated/Command";

let at = 0;
function command(name: string, shortcut: string | null, items: Command[] = []): Command {
  at += 1;
  return {
    props: { kind: "item", caption: name, name, icon: null, shortcut, large: false, onClick: "Card", newWindow: false, handler: null },
    items,
    span: { start: at, end: at },
  } as Command;
}

const frame = (): ShownFrame =>
  ({
    menu: { value: { name: "mainMenu", items: [command("file", null, [command("newItem", "Ctrl+N"), command("plain", null)])], span: { start: 0, end: 0 } }, inherited: false },
    toolbar: { value: { name: "mainToolbar", items: [command("saveButton", "Ctrl+S")], span: { start: 0, end: 0 } }, inherited: true },
    ribbon: {
      value: {
        name: "mainRibbon",
        collapsible: false,
        collapsed: false,
        tabs: [
          { title: "Home", name: "homeTab", groups: [{ title: "Edit", name: "editGroup", items: [command("copyButton", "Ctrl+C")], span: { start: 0, end: 0 } }], span: { start: 0, end: 0 } },
          { title: "Insert", name: "insertTab", groups: [{ title: "Pictures", name: "picturesGroup", items: [command("pictureButton", "Ctrl+P")], span: { start: 0, end: 0 } }], span: { start: 0, end: 0 } },
        ],
        span: { start: 0, end: 0 },
      },
      inherited: false,
    },
    left: null,
    right: null,
    footer: null,
  }) as ShownFrame;

const names = (found: { command: Command }[]) => found.map((f) => f.command.props.name);

test("every command with a key in the frame answers, a ribbon's on every tab", () => {
  assert.deepEqual(names(keyedCommands(frame(), () => true, () => true)), ["newItem", "saveButton", "copyButton", "pictureButton"]);
});

test("what code hid or disabled does not answer, nor what is inside it", () => {
  const hidden = new Set(["insertTab", "file"]);
  const disabled = new Set(["mainToolbar"]);
  assert.deepEqual(
    names(keyedCommands(frame(), (n) => !n || !hidden.has(n), (n) => !n || !disabled.has(n))),
    ["copyButton"],
  );
});

test("the person's keys take the design's place; a command of the app's frame is known by its name", () => {
  const person: Record<string, string[]> = { saveButton: ["Ctrl+Shift+S"], "Card.newItem": [] };
  const keysOf = (c: Command, app: boolean) => person[app ? c.props.name! : `Card.${c.props.name}`] ?? (c.props.shortcut ? [c.props.shortcut] : []);
  const found = keyedCommands(frame(), () => true, () => true, keysOf);
  // The menu is the window's own, the toolbar the app's.
  assert.deepEqual(found.map((f) => [f.command.props.name, f.keys]), [["saveButton", ["Ctrl+Shift+S"]], ["copyButton", ["Ctrl+C"]], ["pictureButton", ["Ctrl+P"]]]);
});
