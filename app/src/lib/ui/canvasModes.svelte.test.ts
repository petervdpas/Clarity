import { describe, expect, it } from "vitest";
import { anchorOf, cleanSvg, objectAt, objectsOf, shapeOfDrag } from "./canvasModes";

describe("canvas modes on the screen (DESIGN 5.31)", () => {
  const drawing = [
    { op: "clear" },
    { op: "object", record: 7, x: 10, y: 10, width: 100, height: 50 },
    { op: "rect", x: 10, y: 10, width: 100, height: 50, color: "raised" },
    { op: "text", x: 60, y: 20, text: "A" },
    { op: "object", record: 8, x: 60, y: 30, width: 100, height: 50 },
    { op: "rect", x: 60, y: 30, width: 100, height: 50, color: "raised" },
  ];

  it("an object is its box and the shapes drawn after it until the next", () => {
    expect(objectsOf(drawing)).toEqual([
      { record: 7, x: 10, y: 10, width: 100, height: 50, from: 1, to: 4 },
      { record: 8, x: 60, y: 30, width: 100, height: 50, from: 4, to: 6 },
    ]);
  });

  it("a press takes the topmost object under it", () => {
    const objects = objectsOf(drawing);
    expect(objectAt(objects, { x: 70, y: 40 })?.record).toBe(8);
    expect(objectAt(objects, { x: 20, y: 20 })?.record).toBe(7);
    expect(objectAt(objects, { x: 500, y: 500 })).toBeNull();
  });

  it("a drag with a tool makes its shape, in the ink and brush", () => {
    const from = { x: 40, y: 30 };
    const to = { x: 10, y: 50 };
    expect(shapeOfDrag("line", [from, to], "#ff0000", 3)).toEqual({ op: "line", x1: 40, y1: 30, x2: 10, y2: 50, color: "#ff0000", width: 3 });
    expect(shapeOfDrag("rect", [from, to], "#ff0000", 3)).toEqual({ op: "rect", x: 10, y: 30, width: 30, height: 20, color: "#ff0000" });
    expect(shapeOfDrag("ellipse", [from, to], "#ff0000", 3)).toEqual({ op: "ellipse", x: 25, y: 40, rx: 15, ry: 10, color: "#ff0000" });
    expect(shapeOfDrag("path", [from, { x: 41, y: 31 }, to], "#ff0000", 3)).toEqual({ op: "path", points: [40, 30, 41, 31, 10, 50], color: "#ff0000", width: 3 });
    expect(shapeOfDrag("rect", [from, from], "#ff0000", 3)).toBeNull();
  });

  it("a shape is moved by its first place", () => {
    expect(anchorOf("rect", { x: "1", y: "2" })).toEqual({ x: 1, y: 2 });
    expect(anchorOf("ellipse", { cx: "3", cy: "4" })).toEqual({ x: 3, y: 4 });
    expect(anchorOf("line", { x1: "5", y1: "6" })).toEqual({ x: 5, y: 6 });
    expect(anchorOf("polyline", { points: "7,8 9,10" })).toEqual({ x: 7, y: 8 });
    expect(anchorOf("text", { x: "11", y: "12" })).toEqual({ x: 11, y: 12 });
    expect(anchorOf("circle", { cx: "1", cy: "1" })).toBeNull();
  });

  it("a drawing shows without anything that runs, and keeps its viewBox", () => {
    const text =
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 300">\n<rect x="1" y="2" width="3" height="4" fill="#ff0000" onclick="alert(1)"/>\n<script>alert(1)</script>\n<a href="javascript:alert(1)"><text x="1" y="1">x</text></a>\n<text x="5" y="6" fill="#000" font-size="12">a &amp; b</text>\n</svg>\n';
    const clean = cleanSvg(text);
    expect(clean.units).toEqual([400, 300]);
    // What is left out still counts, so each element keeps the place of its shape.
    expect(clean.shapes.map((s) => s.tag)).toEqual(["rect", "g", "g", "text"]);
    expect(clean.shapes[0].attrs).toEqual({ x: "1", y: "2", width: "3", height: "4", fill: "#ff0000" });
    expect(clean.shapes[3].text).toBe("a & b");
    expect(cleanSvg("").shapes).toEqual([]);
  });
});
