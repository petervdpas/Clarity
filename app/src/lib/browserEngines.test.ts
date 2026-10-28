// The engines beside the screen of C# with Avalonia in a browser (DESIGN
// 7.6): .NET calls them on the page's own thread, each call answered at
// once, so the canvas's frame loop is the desktop's.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { close, frame, hears, load } from "./browserEngines.ts";

const read = (name: string) => readFileSync(new URL(`../../../conformance/${name}`, import.meta.url));
const screen = JSON.parse(read("screen-engines.json").toString());
const wasm = read("engines/screen.wasm").toString("base64");

test("an engine is loaded at once and draws its frames as the cases say", () => {
  for (const run of screen.frames) {
    const engine = load(screen.name, wasm, JSON.stringify({ uses: screen.uses, opens: run.opens, seconds: 5, megabytes: 64 }));
    assert.equal(typeof JSON.parse(hears(engine)).pointer, "boolean", run.name);
    const drawn = run.ticks.map((ms: number) => JSON.parse(frame(engine, ms, "[]")).shapes);
    assert.deepEqual(drawn, run.draws, run.name);
    close(engine);
  }
});

test("an engine that is no WebAssembly says so", () => {
  assert.throws(() => load("Broken", Buffer.from("nope").toString("base64"), JSON.stringify({ uses: [], opens: false, seconds: 5, megabytes: 64 })), /engine Broken is no WebAssembly module/);
});
