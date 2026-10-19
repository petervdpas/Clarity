import { test } from "node:test";
import assert from "node:assert/strict";
import { projectEngines, signature } from "./engines.ts";
import type { Project } from "./generated/Project";

const span = (start: number, end: number) => ({ start, end, line: 1, col: 1 });
const ident = (name: string) => ({ name, span: span(0, 0) });

const pricing = {
  name: ident("Pricing"),
  file: "pricing.wasm",
  span: span(10, 90),
  functions: [
    {
      name: ident("total"),
      params: [{ name: ident("lines"), ty: { kind: { kind: "list", item: "Line" }, span: span(0, 0) } }],
      returns: { kind: { kind: "money" }, span: span(0, 0) },
      span: span(40, 80),
    },
  ],
};

const project = {
  files: [
    { path: "shop/sales.clarity", module: { name: "sales" }, engines: [pricing] },
    { path: "shop/customers.clarity", module: { name: "customers" }, engines: [] },
  ],
  diagnostics: [
    { severity: "error", message: "engine Pricing: 'pricing.wasm' has no function total", path: "shop/sales.clarity", span: span(10, 90) },
    { severity: "warning", message: "a warning inside", path: "shop/sales.clarity", span: span(50, 60) },
    { severity: "error", message: "elsewhere in the file", path: "shop/sales.clarity", span: span(200, 210) },
    { severity: "error", message: "another file", path: "shop/customers.clarity", span: span(20, 30) },
  ],
} as unknown as Project;

test("the engines of a project, each with its module and the errors inside its declaration", () => {
  const engines = projectEngines(project);
  assert.equal(engines.length, 1);
  assert.equal(engines[0].engine.name.name, "Pricing");
  assert.equal(engines[0].module, "sales");
  assert.deepEqual(engines[0].problems, ["engine Pricing: 'pricing.wasm' has no function total"]);
});

test("a function of an engine reads as it is declared", () => {
  assert.equal(signature(pricing.functions[0] as never), "total(lines: list<Line>) -> money");
});
