import { test } from "node:test";
import assert from "node:assert/strict";
import type { Erd } from "./generated/Erd.ts";
import { addEntity, boxes, pinned, connect, endMark, ends, meeting, moveEntity, removeEntity, renameEntity, rowsOf } from "./erdDesign.ts";

const erd = (): Erd => ({
  entities: [
    { name: "Customer", x: 40, y: 60, fields: [{ name: "name", typeText: "text(80)", required: true, unique: false, extra: "" }] },
    { name: "Order", x: null, y: null, fields: [] },
  ],
  relations: [{ left: "Order", right: "Customer", leftCount: "zeroOrMore", rightCount: "exactlyOne", identifying: false, label: "customer" }],
});

/** Every character is 7 wide, as a monospace font at 12px nearly is. */
const measure = (text: string) => text.length * 7;

test("a box shows its fields as rows: type, name and keys", () => {
  assert.deepEqual(rowsOf(erd().entities[0]), [{ type: "text(80)", name: "name", keys: "*" }]);
  const keyed = rowsOf({ name: "A", x: null, y: null, fields: [{ name: "c", typeText: "ref Customer", required: false, unique: true, extra: "" }] });
  assert.deepEqual(keyed, [{ type: "ref Customer", name: "c", keys: "FK UK" }]);
});

test("a box is as wide as its widest line and as high as its rows; a box without a place gets one in the grid", () => {
  const [customer, order] = boxes(erd(), measure);
  assert.equal(customer.x, 40);
  assert.equal(customer.y, 60);
  assert.ok(customer.w >= measure("text(80)  name  *"));
  assert.equal(customer.h, 32 + 22 * 1 + 6);
  assert.ok(order.x > customer.x + customer.w, "beside the placed box, not on it");
  assert.equal(order.h, 32 + 6);
});

test("a relation meets each box at its edge, on the line between their centers", () => {
  const a = { x: 0, y: 0, w: 100, h: 50 };
  const b = { x: 300, y: 0, w: 100, h: 50 };
  const [from, to] = meeting(a, b);
  assert.deepEqual(from, { x: 100, y: 25 });
  assert.deepEqual(to, { x: 300, y: 25 });
});

test("each end shows how many: a bar for one, a ring for zero, a crow's foot for many", () => {
  const one = endMark({ x: 100, y: 25 }, { x: 1, y: 0 }, "exactlyOne");
  assert.equal(one.bars, 2);
  assert.equal(one.ring, false);
  assert.equal(one.foot, false);
  const many = endMark({ x: 100, y: 25 }, { x: 1, y: 0 }, "zeroOrMore");
  assert.equal(many.foot, true);
  assert.equal(many.ring, true);
  assert.match(many.path, /^M/);
});

test("the canvas edits: a new entity takes a free name, moving and renaming keep the relations", () => {
  let e = addEntity(erd(), 200, 300);
  assert.equal(e.entities[2].name, "Entity1");
  assert.deepEqual([e.entities[2].x, e.entities[2].y], [200, 300]);
  e = addEntity(e, 0, 0);
  assert.equal(e.entities[3].name, "Entity2");
  e = moveEntity(e, "Order", 400.4, 81.6);
  assert.deepEqual([e.entities[1].x, e.entities[1].y], [400, 82]);
  e = renameEntity(e, "Customer", "Client");
  assert.equal(e.relations[0].right, "Client");
  assert.equal(renameEntity(e, "Client", "Order"), e, "a name in use is refused");
  e = connect(e, "Entity1", "Client");
  assert.deepEqual(e.relations[1], { left: "Entity1", right: "Client", leftCount: "zeroOrMore", rightCount: "exactlyOne", identifying: false, label: null });
  assert.equal(connect(e, "Entity1", "Client"), e, "one relation between the same two is enough");
  e = removeEntity(e, "Client");
  assert.deepEqual(e.relations, [], "its relations go with it");
});

test("an edit pins every box where it is shown, so nothing else moves", () => {
  const shown = boxes(erd(), measure);
  const pinnedErd = pinned(moveEntity(erd(), "Customer", 500, 500), shown);
  assert.deepEqual([pinnedErd.entities[1].x, pinnedErd.entities[1].y], [shown[1].x, shown[1].y]);
  assert.deepEqual([pinnedErd.entities[0].x, pinnedErd.entities[0].y], [500, 500]);
});

test("relations meeting the same side of a box are spread along it, in the order of their other ends", () => {
  const album = { name: "Album", rows: [], placed: true, x: 0, y: 0, w: 100, h: 120 };
  const track = { name: "Track", rows: [], placed: true, x: 300, y: -40, w: 100, h: 40 };
  const loan = { name: "Loan", rows: [], placed: true, x: 300, y: 140, w: 100, h: 40 };
  const relation = (left: string, right: string) => ({ left, right, leftCount: "zeroOrMore" as const, rightCount: "exactlyOne" as const, identifying: false, label: null });
  const [toTrack, toLoan] = ends([album, track, loan], [relation("Track", "Album"), relation("Loan", "Album")]);
  // Both meet Album's right side: Track's above Loan's, a third and two thirds down.
  assert.deepEqual(toTrack?.[1], { x: 100, y: 40 });
  assert.deepEqual(toLoan?.[1], { x: 100, y: 80 });
  // Alone on its side, a relation meets the middle of it.
  assert.deepEqual(toTrack?.[0], { x: 300, y: -20 });
});

test("a relation to itself or to a box not shown has no ends", () => {
  const a = { name: "A", rows: [], placed: true, x: 0, y: 0, w: 100, h: 40 };
  const relation = (left: string, right: string) => ({ left, right, leftCount: "zeroOrMore" as const, rightCount: "exactlyOne" as const, identifying: false, label: null });
  assert.deepEqual(ends([a], [relation("A", "A"), relation("A", "B")]), [null, null]);
});
