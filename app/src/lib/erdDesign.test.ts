import { test } from "node:test";
import assert from "node:assert/strict";
import type { Erd } from "./generated/Erd.ts";
import { addEntity, connect, erdEdit, erdLinks, erdShapes, moveEntity, removeEntity, renameEntity, rowsOf } from "./erdDesign.ts";

const erd = (): Erd => ({
  entities: [
    { name: "Customer", x: 40, y: 60, fields: [{ name: "name", typeText: "text(80)", required: true, unique: false, extra: "" }] },
    { name: "Order", x: null, y: null, fields: [] },
  ],
  relations: [{ left: "Order", right: "Customer", leftCount: "zeroOrMore", rightCount: "exactlyOne", identifying: false, label: "customer" }],
});

test("a box shows its fields as rows: type, name and keys", () => {
  assert.deepEqual(rowsOf(erd().entities[0]), [["text(80)", "name", "*"]]);
  const keyed = rowsOf({ name: "A", x: null, y: null, fields: [{ name: "c", typeText: "ref Customer", required: false, unique: true, extra: "" }] });
  assert.deepEqual(keyed, [["ref Customer", "c", "FK UK"]]);
});

test("an ERD is drawn as boxes and lines that show how many at each end, dashed when not owned", () => {
  const shapes = erdShapes(erd());
  assert.deepEqual(shapes.map((s) => [s.name, s.x, s.y]), [["Customer", 40, 60], ["Order", null, null]]);
  assert.deepEqual(erdLinks(erd()), [
    { left: "Order", right: "Customer", leftEnd: { kind: "count", count: "zeroOrMore" }, rightEnd: { kind: "count", count: "exactlyOne" }, dashed: true, label: "customer" },
  ]);
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

test("an edit on the canvas changes the ERD, pins the other boxes and says what to pick", () => {
  const shown = [{ name: "Customer", rows: [], stereotype: null, tone: "entity", placed: true, title: 32, x: 40, y: 60, w: 100, h: 40 },
    { name: "Order", rows: [], stereotype: null, tone: "entity", placed: false, title: 32, x: 300, y: 40, w: 100, h: 40 }];
  const added = erdEdit(erd(), { op: "add", item: "entity", x: 10, y: 20 }, shown);
  assert.deepEqual(added.pick, { kind: "box", name: "Entity1" });
  assert.deepEqual([added.next.entities[1].x, added.next.entities[1].y], [300, 40], "Order is pinned where it is shown");
  const moved = erdEdit(erd(), { op: "move", name: "Customer", x: 5, y: 6 }, shown);
  assert.deepEqual([moved.next.entities[0].x, moved.next.entities[0].y], [5, 6]);
  assert.equal(moved.pick, undefined);
  const connected = erdEdit(erd(), { op: "connect", tool: "relation", from: "Customer", to: "Order" }, shown);
  assert.deepEqual(connected.pick, undefined, "a relation between these two is there already");
});

test("several boxes move in one edit", () => {
  const { next } = erdEdit(erd(), { op: "moveAll", moves: [{ name: "Customer", x: 1, y: 2 }, { name: "Order", x: 3, y: 4 }] }, []);
  assert.deepEqual(next.entities.map((e) => [e.x, e.y]), [[1, 2], [3, 4]]);
});
