import { test } from "node:test";
import assert from "node:assert/strict";
import type { ClassDesign } from "./generated/ClassDesign.ts";
import { addBox, classEdit, classLinks, classShapes, connectBoxes, removeBox, renameBox } from "./classDesign.ts";

const design = (): ClassDesign => ({
  boxes: [
    { name: "Order", kind: "entity", module: "shop", x: 40, y: 60, fields: [{ name: "number", typeText: "text(12)", required: true, unique: true, extra: "" }], values: [], functions: [] },
    { name: "Status", kind: "enum", module: null, x: null, y: null, fields: [], values: ["Pending", "Shipped"], functions: [] },
    { name: "Pricing", kind: "engine", module: null, x: null, y: null, fields: [], values: [], functions: ["price(order: integer) -> money"] },
  ],
  relations: [{ left: "Order", right: "Status", link: "association", leftCount: "zeroOrMore", rightCount: "exactlyOne", label: "status" }],
});

test("each kind of box shows its members, with its stereotype above the name", () => {
  const [order, status, pricing] = classShapes(design());
  assert.deepEqual(order.rows, [["number: text(12)", "UK *"]]);
  assert.equal(order.stereotype, null);
  assert.deepEqual([status.stereotype, status.tone, status.rows], ["enumeration", "enum", [["Pending"], ["Shipped"]]]);
  assert.deepEqual([pricing.stereotype, pricing.rows], ["engine", [["price(order: integer) -> money"]]]);
});

test("an association points at what is referred to; a composition has a filled diamond at the whole, an aggregation a hollow one", () => {
  let d = design();
  d = connectBoxes(d, "composition", "Order", "Pricing");
  d = connectBoxes(d, "aggregation", "Status", "Pricing");
  const [association, composition, aggregation] = classLinks(d);
  assert.deepEqual(association, {
    left: "Order",
    right: "Status",
    leftEnd: { kind: "uml", head: "none", text: "*" },
    rightEnd: { kind: "uml", head: "arrow", text: "1" },
    dashed: false,
    label: "status",
  });
  assert.deepEqual([composition.leftEnd, composition.rightEnd], [{ kind: "uml", head: "filledDiamond", text: "1" }, { kind: "uml", head: "none", text: "*" }]);
  assert.equal(aggregation.leftEnd.kind === "uml" && aggregation.leftEnd.head, "diamond");
});

test("the canvas edits: a new box of each kind, renaming keeps the relations, one relation between two boxes", () => {
  let d = addBox(design(), "enum", 10, 20);
  assert.deepEqual(d.boxes[3], { name: "Enum1", kind: "enum", module: null, x: 10, y: 20, fields: [], values: [], functions: [] });
  d = addBox(d, "module", 0, 0);
  assert.equal(d.boxes[4].name, "module1", "a module is named as modules are, in lower case");
  d = renameBox(d, "Status", "OrderStatus");
  assert.equal(d.relations[0].right, "OrderStatus");
  assert.equal(renameBox(d, "OrderStatus", "Order"), d, "a name in use is refused");
  assert.equal(connectBoxes(d, "association", "OrderStatus", "Order"), d, "one relation between the same two is enough");
  d = removeBox(d, "OrderStatus");
  assert.deepEqual(d.relations, [], "its relations go with it");
});

test("an edit on the canvas changes the design, pins the other boxes and says what to pick", () => {
  const shown = design().boxes.map((b, i) => ({ name: b.name, rows: [], stereotype: null, tone: "entity", placed: true, title: 32, x: 300 * i, y: 40, w: 100, h: 40 }));
  const added = classEdit(design(), { op: "add", item: "engine", x: 1, y: 2 }, shown);
  assert.deepEqual(added.pick, { kind: "box", name: "Engine1" });
  assert.deepEqual([added.next.boxes[1].x, added.next.boxes[1].y], [300, 40]);
  const connected = classEdit(design(), { op: "connect", tool: "composition", from: "Order", to: "Pricing" }, shown);
  assert.deepEqual(connected.pick, { kind: "link", index: 1 });
  assert.equal(connected.next.relations[1].link, "composition");
});

test("a box is drawn in its module's frame; a module's own box is drawn in the frame of its name", () => {
  let d = addBox(design(), "module", 0, 0);
  d = renameBox(d, "module1", "shop");
  const groups = classShapes(d).map((s) => [s.name, s.group]);
  assert.deepEqual(groups, [["Order", "shop"], ["Status", null], ["Pricing", null], ["shop", "shop"]]);
});

test("dragging a frame moves all its boxes in one edit", () => {
  const shown = design().boxes.map((b, i) => ({ name: b.name, rows: [], stereotype: null, tone: "entity", placed: true, title: 32, x: 300 * i, y: 40, w: 100, h: 40 }));
  const { next } = classEdit(design(), { op: "moveAll", moves: [{ name: "Order", x: 10.4, y: 20 }, { name: "Status", x: 30, y: 40 }] }, shown);
  assert.deepEqual(next.boxes.map((b) => [b.x, b.y]), [[10, 20], [30, 40], [600, 40]]);
});

test("a uses arrow is dashed with an open head and no counts", () => {
  const d = design();
  d.relations.push({ left: "Pricing", right: "Order", link: "uses", leftCount: "exactlyOne", rightCount: "exactlyOne", label: null });
  const uses = classLinks(d)[1];
  assert.deepEqual(uses, {
    left: "Pricing",
    right: "Order",
    leftEnd: { kind: "uml", head: "none", text: "" },
    rightEnd: { kind: "uml", head: "arrow", text: "" },
    dashed: true,
    label: null,
  });
});
