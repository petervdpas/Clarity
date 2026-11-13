// What the properties panel works out for a part's lists: columns in their written order, link and drop handlers, a canvas's tools.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  CANVAS_MODE_TOOLS,
  DROP_KINDS,
  TOOL_ACTIONS,
  dropSources,
  moved,
  objectsFor,
  shownOrder,
  toggled,
  toolsToggled,
  withDrop,
  withLink,
} from "./partProperties.ts";

test("a ticked column goes last, keeping the order as written", () => {
  assert.deepEqual(toggled(["title", "year"], "artist", true), ["title", "year", "artist"]);
  assert.deepEqual(toggled(["title", "year"], "title", true), ["title", "year"]);
  assert.deepEqual(toggled(["title", "artist", "year"], "artist", false), ["title", "year"]);
});

test("a column moves up or down by one, and stays at the ends", () => {
  assert.deepEqual(moved(["a", "b", "c"], "b", -1), ["b", "a", "c"]);
  assert.deepEqual(moved(["a", "b", "c"], "b", 1), ["a", "c", "b"]);
  assert.deepEqual(moved(["a", "b", "c"], "a", -1), ["a", "b", "c"]);
  assert.deepEqual(moved(["a", "b", "c"], "c", 1), ["a", "b", "c"]);
  assert.deepEqual(moved(["a", "b"], "x", 1), ["a", "b"]);
});

test("the panel lists the chosen fields in their order, then the others in the entity's", () => {
  assert.deepEqual(shownOrder(["id", "title", "artist", "year"], ["year", "title"]), ["year", "title", "id", "artist"]);
  // A chosen name the entity no longer has is left out of the list.
  assert.deepEqual(shownOrder(["title"], ["gone", "title"]), ["title"]);
});

test("a column's link handler is set, replaced and taken away", () => {
  const links = [{ column: "chapter", handler: "toChapter" }];
  assert.deepEqual(withLink(links, "layer", "toLayer"), [...links, { column: "layer", handler: "toLayer" }]);
  assert.deepEqual(withLink(links, "chapter", "openChapter"), [{ column: "chapter", handler: "openChapter" }]);
  assert.deepEqual(withLink(links, "chapter", null), []);
});

test("a drop source is added, rewired and removed", () => {
  const drops = [{ from: "fragmentList", handler: "linkFragment" }];
  assert.deepEqual(withDrop(drops, "noteList", "linkNote"), [...drops, { from: "noteList", handler: "linkNote" }]);
  assert.deepEqual(withDrop(drops, "fragmentList", "fileFragment"), [{ from: "fragmentList", handler: "fileFragment" }]);
  assert.deepEqual(withDrop(drops, "fragmentList", null), []);
});

test("rows are dragged from the window's other lists, tables and trees", () => {
  const parts = [
    { name: "chapterList", kind: "list" },
    { name: "fragmentList", kind: "list" },
    { name: "notes", kind: "treeTable" },
    { name: "saveButton", kind: "button" },
    { name: null, kind: "table" },
  ] as const;
  assert.deepEqual(dropSources(parts, "chapterList"), ["fragmentList", "notes"]);
  assert.ok(DROP_KINDS.includes("memo"));
  assert.ok(!DROP_KINDS.includes("button"));
});

test("a canvas's tools: all of the mode while none are written", () => {
  assert.deepEqual(toolsToggled([], "paint", "spray", false), CANVAS_MODE_TOOLS.paint.filter((t) => t !== "spray"));
  assert.deepEqual(toolsToggled(["pen", "eraser"], "paint", "fill", true), ["pen", "eraser", "fill"]);
  assert.deepEqual(toolsToggled(["pen", "eraser"], "paint", "pen", false), ["eraser"]);
  // The last one stays: none at all cannot be written.
  assert.deepEqual(toolsToggled(["pen"], "paint", "pen", false), ["pen"]);
  // Every tool ticked is the same as none written.
  assert.deepEqual(toolsToggled(["select"], "objects", "connect", true), []);
});

test("a canvas in mode objects starts on a list and its x and y fields", () => {
  assert.deepEqual(objectsFor("types", ["id", "x", "y", "width"]), { list: "types", x: "x", y: "y" });
  assert.deepEqual(objectsFor("boxes", ["left", "top"]), { list: "boxes", x: "left", y: "top" });
  assert.equal(objectsFor("boxes", []), null);
});

// The lists stand in the spec (clarity_spec::model); these guard that the panel's copy follows it.
const model = readFileSync(new URL("../../../crates/clarity-spec/src/model.rs", import.meta.url), "utf8");
function words(text: string): string[] {
  return [...text.matchAll(/"([A-Za-z]+)"/g)].map((m) => m[1]);
}

test("the tool actions are the spec's TOOL_ACTIONS", () => {
  const written = model.match(/pub const TOOL_ACTIONS: \[&str; \d+\] = \[([^\]]*)\]/);
  assert.ok(written, "TOOL_ACTIONS in model.rs");
  assert.deepEqual([...TOOL_ACTIONS], words(written[1]));
});

test("each mode's tools are the spec's CanvasMode::tools", () => {
  for (const [mode, name] of [["objects", "Objects"], ["paint", "Paint"], ["vector", "Vector"]] as const) {
    const written = model.match(new RegExp(`CanvasMode::${name} => &\\[([^\\]]*)\\]`));
    assert.ok(written, `CanvasMode::${name} in model.rs`);
    assert.deepEqual([...CANVAS_MODE_TOOLS[mode]], words(written[1]));
  }
});
