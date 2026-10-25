// Shortcut keys of the IDE (DESIGN 5.28): what is active answers a key, the
// innermost layer first, and the person sets the keys, swapping when one is taken.

import { test } from "node:test";
import assert from "node:assert/strict";
import { IDE_COMMANDS, alsoHas, assign, clashes, commandFor, eventKey, fieldKeeps, keysOf, refused, reset, shownKey, type Layer } from "./keys.ts";

const press = (key: string, more: Partial<KeyboardEventInit & { code: string }> = {}) => ({ key, code: "", ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...more });

test("a key press is written as Clarity writes keys, Ctrl for Cmd on macOS", () => {
  assert.equal(eventKey(press("n", { ctrlKey: true, shiftKey: true }), false), "Ctrl+Shift+N");
  assert.equal(eventKey(press("n", { metaKey: true }), true), "Ctrl+N");
  // On macOS the Control key is a key of its own.
  assert.equal(eventKey(press("n", { ctrlKey: true }), true), "Control+N");
  assert.equal(eventKey(press("F5"), false), "F5");
  assert.equal(eventKey(press("ArrowLeft", { altKey: true }), false), "Alt+Left");
  // A digit by its place, whatever the keyboard layout makes of it.
  assert.equal(eventKey(press("&", { ctrlKey: true, code: "Digit1" }), false), "Ctrl+1");
  assert.equal(eventKey(press(",", { ctrlKey: true }), false), "Ctrl+,");
  // A modifier on its own is no key yet.
  assert.equal(eventKey(press("Control", { ctrlKey: true }), false), null);
});

test("on macOS a key shows with Cmd", () => {
  assert.equal(shownKey("Ctrl+Shift+N", true), "Cmd+Shift+N");
  assert.equal(shownKey("Ctrl+Shift+N", false), "Ctrl+Shift+N");
});

test("the innermost active layer answers first, and a key no layer has answers nothing", () => {
  const active: Layer[] = ["vault.secrets", "vault", "global"];
  assert.equal(commandFor("Ctrl+N", active, IDE_COMMANDS, {}), "vault.add");
  assert.equal(commandFor("Ctrl+N", ["vault.connections", "vault", "global"], IDE_COMMANDS, {}), "vault.newConnection");
  assert.equal(commandFor("F1", active, IDE_COMMANDS, {}), "manual");
  // Reload is the Project view's; on Vault, F5 is nobody's.
  assert.equal(commandFor("F5", active, IDE_COMMANDS, {}), null);
  assert.equal(commandFor("F5", ["project", "global"], IDE_COMMANDS, {}), "project.reload");
});

test("the IDE's own keys do not clash within a layer", () => {
  assert.deepEqual(clashes(IDE_COMMANDS, {}), []);
});

test("the person's keys take the place of the design's, and only what differs is kept", () => {
  const choices = assign(IDE_COMMANDS, {}, "vault.lock", "Ctrl+K").choices;
  assert.deepEqual(choices, { "vault.lock": ["Ctrl+K"] });
  assert.deepEqual(keysOf(IDE_COMMANDS.find((c) => c.id === "vault.lock")!, choices), ["Ctrl+K"]);
  assert.equal(commandFor("Ctrl+K", ["vault", "global"], IDE_COMMANDS, choices), "vault.lock");
  assert.equal(commandFor("Ctrl+L", ["vault", "global"], IDE_COMMANDS, choices), null);
  // Back to the design's key: nothing is kept.
  assert.deepEqual(assign(IDE_COMMANDS, choices, "vault.lock", "Ctrl+L").choices, {});
  // A command without a key in the design gets one.
  assert.deepEqual(assign(IDE_COMMANDS, {}, "agent", "Ctrl+J").choices, { agent: ["Ctrl+J"] });
});

test("a key taken in the same layer swaps: the other command gets the key this one had", () => {
  const done = assign(IDE_COMMANDS, {}, "vault.add", "Ctrl+Shift+C");
  assert.equal(done.swapped, "vault.copy");
  assert.deepEqual(done.choices, { "vault.add": ["Ctrl+Shift+C"], "vault.copy": ["Ctrl+N"] });
  assert.deepEqual(clashes(IDE_COMMANDS, done.choices), []);
  // An outer layer's key is no clash: the inner one wins while it shows.
  assert.equal(assign(IDE_COMMANDS, {}, "vault.lock", "Ctrl+O").swapped, null);
  assert.equal(assign(IDE_COMMANDS, {}, "vault.lock", "Ctrl+N").swapped, null);
});

test("the keys the system and the inputs need are refused, saying why", () => {
  assert.equal(refused("Tab"), "tab");
  assert.equal(refused("Enter"), "enter");
  assert.equal(refused("Escape"), "escape");
  assert.equal(refused("A"), "typing");
  assert.equal(refused("Shift+A"), "typing");
  assert.equal(refused("Ctrl+A"), null);
  assert.equal(refused("F2"), null);
  assert.equal(refused("Delete"), null);
});

test("in a text field the field keeps its own keys", () => {
  assert.equal(fieldKeeps("Delete"), true);
  assert.equal(fieldKeeps("Ctrl+Z"), true);
  assert.equal(fieldKeeps("Ctrl+C"), true);
  assert.equal(fieldKeeps("Ctrl+Left"), true);
  assert.equal(fieldKeeps("F1"), false);
  assert.equal(fieldKeeps("Ctrl+1"), false);
  assert.equal(fieldKeeps("Ctrl+Shift+C"), false);
});

test("a key another layer has is allowed, and the window can say which command has it there", () => {
  assert.deepEqual(alsoHas(IDE_COMMANDS, {}, "vault.lock", "Ctrl+O"), ["project.open"]);
  assert.deepEqual(alsoHas(IDE_COMMANDS, {}, "vault.lock", "Ctrl+K"), []);
  // The same layer is a swap, not this.
  assert.deepEqual(alsoHas(IDE_COMMANDS, {}, "vault.add", "Ctrl+Shift+C"), []);
});

test("putting a command's key back swaps back what the swap gave away", () => {
  const swapped = assign(IDE_COMMANDS, {}, "vault.add", "Ctrl+Shift+C").choices;
  assert.deepEqual(reset(IDE_COMMANDS, swapped, "vault.add"), {});
  // Two keys come back as two; a command without a key in the design has none again.
  assert.deepEqual(reset(IDE_COMMANDS, { "design.redo": ["F9"] }, "design.redo"), {});
  assert.deepEqual(reset(IDE_COMMANDS, { agent: ["Ctrl+J"] }, "agent"), {});
});
