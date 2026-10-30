// The edit bus: each edit is a message; Ctrl+Z takes back the last one of the scope with the focus.

import assert from "node:assert/strict";
import test from "node:test";
import { changesOf, EditBus, setPath, type Edit } from "./editBus.ts";

const edit = (part: string, scope: string, path: string, before: string, after: string, step = 0): Edit => ({ part, scope, changes: [{ path, before, after }], step });

test("in a form, Ctrl+Z walks back its fields one by one, the last first", () => {
  const bus = new EditBus();
  bus.push(edit("nameBox", "customerForm", "current.name", "Ada", "Ann"));
  bus.push(edit("diagram", "diagram", "moved", "a", "b", 7));
  bus.push(edit("cityBox", "customerForm", "current.city", "Rome", "Oslo"));
  assert.equal(bus.undo("customerForm")?.part, "cityBox");
  assert.equal(bus.undo("customerForm")?.part, "nameBox");
  assert.equal(bus.undo("customerForm"), null);
  // The canvas outside the form keeps its own, with its backend step.
  assert.equal(bus.undo("diagram")?.step, 7);
});

test("redo does an undone edit again; a new edit in the scope ends that; a refusal keeps it", () => {
  const bus = new EditBus();
  bus.push(edit("nameBox", "f", "current.name", "Ada", "Ann"));
  const undone = bus.undo("f")!;
  assert.equal(bus.canRedo("f"), true);
  assert.equal(bus.redo("f"), undone);
  bus.refused(undone, false);
  assert.equal(bus.canRedo("f"), true);
  bus.push(edit("cityBox", "f", "current.city", "Rome", "Oslo"));
  assert.equal(bus.canRedo("f"), false);
});

test("an edit that changed nothing is no message", () => {
  const bus = new EditBus();
  bus.push({ part: "a", scope: "a", changes: [], step: 0 });
  assert.equal(bus.canUndo("a"), false);
});

test("what a handler changed is the window's data that differs, by name; a path is set back", () => {
  assert.deepEqual(changesOf({ types: [1], moving: 0 }, { types: [2], moving: 0 }), [{ path: "types", before: [1], after: [2] }]);
  const data = { current: { name: "Ann" } };
  setPath(data, "current.name", "Ada");
  assert.deepEqual(data, { current: { name: "Ada" } });
});

test("a cell of a list is set back by its place", () => {
  const data = { tasks: [{ title: "a" }, { title: "b" }] };
  setPath(data, "tasks.1.title", "was b");
  assert.deepEqual(data.tasks[1], { title: "was b" });
});
