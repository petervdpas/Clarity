// An application's keys set by the person (DESIGN 5.28): the model's catalog
// as commands per place, the person's keys over the design's, and the key a
// running part answers.

import { test } from "node:test";
import assert from "node:assert/strict";
import { appCommands, keysOfCommand, personKeys } from "./appKeys.ts";
import { assign, alsoHas, commandFor } from "./keys.ts";
import type { KeyLayer } from "./generated/KeyLayer";

const catalog: KeyLayer[] = [
  { id: "", title: null, commands: [{ id: "newItem", name: "newItem", caption: "@newCaption", keys: ["Ctrl+N"] }, { id: "quitItem", name: "quitItem", caption: "@quitCaption", keys: ["Ctrl+Q"] }] },
  { id: "Card", title: "@cardTitle", commands: [{ id: "Card.saveButton", name: "saveButton", caption: "@saveCaption", keys: ["Ctrl+S"] }, { id: "Card.printButton", name: "printButton", caption: null, keys: [] }] },
];
const commands = appCommands(catalog, (caption, name) => caption?.replace("@", "") ?? name);

test("the catalog is commands per place, each named by its caption or else its name", () => {
  assert.deepEqual(commands, [
    { id: "newItem", name: "newCaption", layers: [""], keys: ["Ctrl+N"] },
    { id: "quitItem", name: "quitCaption", layers: [""], keys: ["Ctrl+Q"] },
    { id: "Card.saveButton", name: "saveCaption", layers: ["Card"], keys: ["Ctrl+S"] },
    { id: "Card.printButton", name: "printButton", layers: ["Card"], keys: [] },
  ]);
  // In a window its own commands go first, then the app's.
  assert.equal(commandFor("Ctrl+N", ["Card", ""], commands, {}), "newItem");
});

test("a swap happens within one place; the app's key taken in a window is allowed", () => {
  assert.equal(assign(commands, {}, "quitItem", "Ctrl+N").swapped, "newItem");
  assert.equal(assign(commands, {}, "Card.printButton", "Ctrl+N").swapped, null);
  assert.deepEqual(alsoHas(commands, {}, "Card.printButton", "Ctrl+N"), ["newItem"]);
});

test("a part answers the person's keys, or the design's where they kept none", () => {
  const choices = { "Card.saveButton": ["Ctrl+Shift+S"], newItem: [] };
  assert.deepEqual(keysOfCommand("Card.saveButton", "Ctrl+S", choices), ["Ctrl+Shift+S"]);
  assert.deepEqual(keysOfCommand("newItem", "Ctrl+N", choices), []);
  assert.deepEqual(keysOfCommand("quitItem", "Ctrl+Q", choices), ["Ctrl+Q"]);
  assert.deepEqual(keysOfCommand("Card.printButton", null, choices), []);
});

test("what the settings file keeps is taken back only as lists of keys", () => {
  assert.deepEqual(personKeys({ newItem: ["Ctrl+K"], bad: "Ctrl+B", worse: [1] }), { newItem: ["Ctrl+K"] });
  assert.deepEqual(personKeys(null), {});
});
