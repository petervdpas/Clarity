import { test } from "node:test";
import assert from "node:assert/strict";
import { boxes, columns, endMark, ends, frames, freeName, meeting, pinned, umlMark, type Box } from "./designCanvas.ts";

/** Every character is 7 wide, as a monospace font at 12px nearly is. */
const measure = (text: string) => text.length * 7;

const box = (name: string, x: number, y: number, w: number, h: number): Box => ({ name, rows: [], stereotype: null, tone: "entity", placed: true, title: 32, x, y, w, h });

test("a box is as wide as its widest row and as high as its rows; a box without a place gets one in the grid", () => {
  const [customer, order] = boxes(
    [
      { name: "Customer", x: 40, y: 60, rows: [["text(80)", "name", "*"]] },
      { name: "Order", x: null, y: null, rows: [] },
    ],
    measure,
  );
  assert.equal(customer.x, 40);
  assert.equal(customer.y, 60);
  assert.ok(customer.w >= measure("text(80)  name  *"));
  assert.equal(customer.h, 32 + 22 * 1 + 6);
  assert.ok(order.x > customer.x + customer.w, "beside the placed box, not on it");
  assert.equal(order.h, 32 + 6);
});

test("a stereotype takes a line of the title, and the box is wide enough for it", () => {
  const [status] = boxes([{ name: "S", x: 0, y: 0, rows: [["Pending"]], stereotype: "enumeration", tone: "enum" }], measure);
  assert.equal(status.title, 32 + 14);
  assert.equal(status.h, 32 + 14 + 22 + 6);
  assert.ok(status.w >= measure("«enumeration»"));
  assert.equal(status.tone, "enum");
});

test("the cells of a row stand in columns as wide as their widest cell", () => {
  const [b] = boxes([{ name: "A", x: 0, y: 0, rows: [["integer", "level", ""], ["text", "name", "*"]] }], measure);
  const [type, name, keys] = columns(b, measure);
  assert.equal(type, 12);
  assert.equal(name, 12 + 7 * 7 + 14);
  assert.equal(keys, name + 5 * 7 + 14);
});

test("a line meets each box at its edge, on the line between their centers", () => {
  const [from, to] = meeting(box("a", 0, 0, 100, 50), box("b", 300, 0, 100, 50));
  assert.deepEqual(from, { x: 100, y: 25 });
  assert.deepEqual(to, { x: 300, y: 25 });
});

test("lines meeting the same side of a box are spread along it, in the order of their other ends", () => {
  const album = box("Album", 0, 0, 100, 120);
  const track = box("Track", 300, -40, 100, 40);
  const loan = box("Loan", 300, 140, 100, 40);
  const [toTrack, toLoan] = ends([album, track, loan], [{ left: "Track", right: "Album" }, { left: "Loan", right: "Album" }]);
  // Both meet Album's right side: Track's above Loan's, a third and two thirds down.
  assert.deepEqual(toTrack?.[1], { x: 100, y: 40 });
  assert.deepEqual(toLoan?.[1], { x: 100, y: 80 });
  // Alone on its side, a line meets the middle of it.
  assert.deepEqual(toTrack?.[0], { x: 300, y: -20 });
});

test("a line to itself or to a box not shown has no ends", () => {
  assert.deepEqual(ends([box("A", 0, 0, 100, 40)], [{ left: "A", right: "A" }, { left: "A", right: "B" }]), [null, null]);
});

test("an ERD end shows how many: a bar for one, a ring for zero, a crow's foot for many", () => {
  const one = endMark({ x: 100, y: 25 }, { x: 1, y: 0 }, "exactlyOne");
  assert.equal(one.bars, 2);
  assert.equal(one.ring, false);
  assert.equal(one.foot, false);
  const many = endMark({ x: 100, y: 25 }, { x: 1, y: 0 }, "zeroOrMore");
  assert.equal(many.foot, true);
  assert.equal(many.ring, true);
  assert.match(many.path, /^M/);
});

test("a UML end has an open arrow, a hollow diamond or a filled one touching the box", () => {
  const at = { x: 100, y: 25 };
  const along = { x: 1, y: 0 };
  const arrow = umlMark(at, along, "arrow");
  assert.equal(arrow.closed, false);
  assert.match(arrow.path, /L100\.0,25\.0/);
  const diamond = umlMark(at, along, "diamond");
  assert.deepEqual([diamond.closed, diamond.filled], [true, false]);
  assert.match(diamond.path, /^M100\.0,25\.0.*Z$/);
  assert.equal(umlMark(at, along, "filledDiamond").filled, true);
  assert.equal(umlMark(at, along, "none").path, "");
});

test("an edit pins every box where it is shown, so nothing else moves", () => {
  const items = [
    { name: "A", x: 500, y: 500 },
    { name: "B", x: null, y: null },
  ];
  const shown = [box("A", 0, 0, 10, 10), box("B", 300.4, 40, 10, 10)];
  assert.deepEqual(pinned(items, shown), [
    { name: "A", x: 500, y: 500 },
    { name: "B", x: 300, y: 40 },
  ]);
});

test("a new box takes the first free numbered name", () => {
  assert.equal(freeName("Entity", ["Entity1", "Entity3"]), "Entity2");
  assert.equal(freeName("Class", []), "Class1");
});

test("the boxes of a group stand in a frame around them, named above its top left; boxes without one have none", () => {
  const a = { ...box("Album", 100, 100, 120, 60), group: "music" };
  const t = { ...box("Track", 300, 200, 100, 40), group: "music" };
  const l = { ...box("Loan", 600, 100, 100, 40), group: "loans" };
  const free = box("Note", 0, 0, 50, 50);
  assert.deepEqual(frames([a, t, l, free]), [
    { name: "music", x: 100 - 16, y: 100 - 16 - 20, w: 300 + 32, h: 140 + 32 + 20 },
    { name: "loans", x: 600 - 16, y: 100 - 16 - 20, w: 100 + 32, h: 40 + 32 + 20 },
  ]);
});

test("boxes without a place are laid out a group at a time, each group starting a row below the tallest box above it", () => {
  const shape = (name: string, group: string | null, rows: number) => ({ name, x: null, y: null, group, rows: Array.from({ length: rows }, () => ["text", "f"]) });
  const laid = boxes([shape("A", "one", 10), shape("X", "two", 1), shape("B", "one", 1), shape("Y", "two", 1)], measure);
  const at = Object.fromEntries(laid.map((b) => [b.name, b]));
  assert.equal(at.A.y, at.B.y, "A and B of group one share a row");
  assert.ok(at.B.x >= at.A.x + at.A.w, "B stands beside A");
  assert.equal(at.X.y, at.Y.y, "X and Y of group two share a row");
  assert.ok(at.X.y >= at.A.y + at.A.h + 16 + 20 + 16, "group two starts below group one's frame");
});
