// A button's key against a key press, as WidgetView checks it.

import { test } from "node:test";
import assert from "node:assert/strict";
import { matchesShortcut, shortcutKeys } from "./shortcut.ts";

type Press = Partial<Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">>;
const press = (p: Press) => ({ key: "", ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...p }) as KeyboardEvent;

test("a shortcut is its modifiers and one key", () => {
  assert.deepEqual(shortcutKeys("Ctrl++"), [["Ctrl"], "+"]);
  assert.deepEqual(shortcutKeys("+"), [[], "+"]);
  assert.deepEqual(shortcutKeys("Ctrl+Shift+F2"), [["Ctrl", "Shift"], "F2"]);
});

test("keys with modifiers match exactly", () => {
  assert.ok(matchesShortcut(press({ key: "s", ctrlKey: true }), "Ctrl+S", false));
  assert.ok(!matchesShortcut(press({ key: "s" }), "Ctrl+S", false));
  assert.ok(!matchesShortcut(press({ key: "s", ctrlKey: true, shiftKey: true }), "Ctrl+S", false));
  // Typing in an input does not stop a key with Ctrl.
  assert.ok(matchesShortcut(press({ key: "s", ctrlKey: true }), "Ctrl+S", true));
});

test("a typed key matches the character, from the keyboard or the number pad", () => {
  assert.ok(matchesShortcut(press({ key: "7" }), "7", false));
  // + is Shift and = on many keyboards; the character is what counts.
  assert.ok(matchesShortcut(press({ key: "+", shiftKey: true }), "+", false));
  assert.ok(matchesShortcut(press({ key: "*" }), "*", false));
  assert.ok(matchesShortcut(press({ key: "." }), ".", false));
  assert.ok(!matchesShortcut(press({ key: "+", ctrlKey: true }), "+", false));
  assert.ok(!matchesShortcut(press({ key: "-" }), "+", false));
});

test("a typed key leaves typing in an input alone", () => {
  assert.ok(!matchesShortcut(press({ key: "7" }), "7", true));
  assert.ok(!matchesShortcut(press({ key: "+" }), "+", true));
  // Named keys are no typing.
  assert.ok(matchesShortcut(press({ key: "F2" }), "F2", true));
});
