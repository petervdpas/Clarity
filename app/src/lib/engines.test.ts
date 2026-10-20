import { test } from "node:test";
import assert from "node:assert/strict";
import { engineText, formOf, freeName, functionLine, paramsOf, paramsText, projectEngines, signature } from "./engines.ts";
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

test("an engine's declaration reads into the inspector's form and writes back as Clarity text", () => {
  const form = formOf({ ...pricing, uses: [ident("clock"), ident("log")], state: "window", opens: true, seconds: 2, megabytes: null } as never);
  assert.deepEqual(form, {
    name: "Pricing",
    file: "pricing.wasm",
    uses: ["clock", "log"],
    state: "window",
    opens: true,
    seconds: 2,
    megabytes: null,
    functions: [{ name: "total", params: [{ name: "lines", type: "list<Line>" }], returns: "money" }],
  });
  assert.equal(
    engineText(form),
    'engine Pricing from "pricing.wasm"\n    uses clock, log\n    state per window\n    on open\n    limit 2 seconds\n    function total(lines: list<Line>) -> money\n',
  );
  // Only what is said: no services, no state, the default limits.
  const plain = { ...form, uses: [], state: "none" as const, opens: false, seconds: null, megabytes: null };
  assert.equal(engineText(plain), 'engine Pricing from "pricing.wasm"\n    function total(lines: list<Line>) -> money\n');
  assert.equal(engineText({ ...plain, seconds: 2, megabytes: 64 }).split("\n")[1], "    limit 2 seconds, 64 MB");
  assert.equal(engineText({ ...plain, megabytes: 64 }).split("\n")[1], "    limit 64 MB");
  // Services in the order Clarity lists them, whatever order they were ticked in.
  assert.equal(engineText({ ...plain, uses: ["log", "clock"] }).split("\n")[1], "    uses clock, log");
});

test("a function is found in the engine's source by its line, for jumping to it", () => {
  const source = "clarity_engine::interface!();\n\n// play\nclarity_engine::function!(play, |(s,): (i64,)| {\n    Ok(s)\n});\n";
  assert.equal(functionLine(source, "play"), 4);
  assert.equal(functionLine(source, "pla"), null);
  assert.equal(functionLine(source, "mark"), null);
});

test("a function's values are typed in one cell, as the declaration writes them", () => {
  assert.deepEqual(paramsOf("square: integer,  level:Level"), [{ name: "square", type: "integer" }, { name: "level", type: "Level" }]);
  assert.deepEqual(paramsOf("  "), []);
  assert.deepEqual(paramsOf("lines: list<Line>"), [{ name: "lines", type: "list<Line>" }]);
  // A value without a type is kept as typed, for the checker to say what is missing.
  assert.deepEqual(paramsOf("square"), [{ name: "square", type: "" }]);
  assert.equal(paramsText([{ name: "a", type: "integer" }, { name: "b", type: "text" }]), "a: integer, b: text");
});

test("a new function gets a name no other function of the engine has", () => {
  assert.equal(freeName([], "compute"), "compute");
  assert.equal(freeName(["compute", "compute2"], "compute"), "compute3");
});
