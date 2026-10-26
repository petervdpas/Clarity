// The renderer's own engine host (DESIGN 5.12), which runs engines beside
// the screen in a Web Worker: it speaks the engine interface as every
// backend host does (conformance/engines.json) and runs the frame loop of
// conformance/screen-engines.json.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { EngineFailed, loadEngine, type Input, type Services } from "./engineHost.ts";

const read = (name: string) => readFileSync(new URL(`../../../conformance/${name}`, import.meta.url));
const cases = JSON.parse(read("engines.json").toString("utf8"));
const screen = JSON.parse(read("screen-engines.json").toString("utf8"));
const TEST = new Uint8Array(read("engines/test.wasm"));
const SCREEN = new Uint8Array(read("engines/screen.wasm"));

const hex = (text: string) => new Uint8Array(text.match(/../g)!.map((b) => parseInt(b, 16)));

function services(log: string[] = []): Services {
  return { clock: () => cases.services.clock, seed: () => BigInt(cases.services.seed), log: (line) => log.push(line), language: () => cases.services.language };
}

const limits = { seconds: 5, megabytes: 256 };

test("an engine's functions give their value or their error", async () => {
  // Without state every call has an instance of its own, as on the backend.
  for (const call of cases.calls) {
    const engine = await loadEngine("Test", TEST, { uses: cases.uses, opens: false, ...limits }, services());
    try {
      assert.deepEqual({ value: engine.call(call.function, call.args) }, call.gives, call.function);
    } catch (e) {
      assert.ok(e instanceof EngineFailed && "error" in call.gives, `${call.function}: ${e}`);
      assert.equal(e.message, `engine Test failed in ${call.function}: ${call.gives.error}`);
    }
  }
});

test("a module that is no engine is refused, saying why", async () => {
  for (const refused of cases.refused) {
    const bytes = refused.bytes ? hex(refused.bytes) : TEST;
    const declared = { uses: refused.uses ?? cases.uses, opens: refused.open ?? false, functions: refused.functions ?? [], ...limits };
    await assert.rejects(loadEngine("Test", bytes, declared, services()), (e: Error) => e.message.startsWith(`engine Test ${refused.message}`), refused.what ?? refused.message);
  }
});

test("the services are the renderer's, as every host gives them", async () => {
  const log: string[] = [];
  const engine = await loadEngine("Test", TEST, { uses: cases.uses, opens: false, ...limits }, services(log));
  for (const served of cases.served) {
    log.length = 0;
    assert.deepEqual({ value: engine.call(served.function, served.args) }, served.gives, served.function);
    assert.deepEqual(log, served.log ?? [], served.function);
  }
});

test("an engine that grows past its memory limit fails", async () => {
  // A call that fails drops its instance, so each call has one of its own.
  for (const call of cases.limited.filter((c: { function: string }) => c.function !== "spin")) {
    const engine = await loadEngine("Test", TEST, { uses: cases.uses, opens: false, ...cases.limits }, services());
    if (call.fails) assert.throws(() => engine.call(call.function, call.args), (e: Error) => e.message.startsWith(call.fails), call.function);
    else assert.deepEqual({ value: engine.call(call.function, call.args) }, call.gives);
  }
});

test("an engine beside the screen draws its frames", async () => {
  for (const run of screen.frames) {
    const engine = await loadEngine(screen.name, SCREEN, { uses: [], opens: run.opens, ...limits, screen: true }, services());
    const drawn = run.ticks.map((ms: number) => engine.frame(ms).shapes);
    assert.deepEqual(drawn, run.draws, run.name);
  }
});

test("a frame the engine refuses fails it", async () => {
  // A tick of less than no time runs until the page stops it, which screenEngine.test.ts covers.
  for (const refused of screen.fails.filter((f: { tick: number }) => f.tick >= 0)) {
    const engine = await loadEngine(screen.name, SCREEN, { uses: [], opens: true, seconds: refused.seconds, megabytes: 256, screen: true }, services());
    for (const ms of refused.ticks) engine.frame(ms);
    assert.throws(() => engine.frame(refused.tick, refused.input ?? []), { message: refused.fails }, refused.name);
  }
});

test("an engine beside the screen exports its frame loop", async () => {
  await assert.rejects(loadEngine("Test", TEST, { uses: cases.uses, opens: false, ...limits, screen: true }, services()), {
    message: "engine Test lacks clarity_tick, which an engine beside the screen needs",
  });
});

test("pointer and keys reach the engine before its tick", async () => {
  for (const run of screen.input) {
    const engine = await loadEngine(screen.name, SCREEN, { uses: [], opens: run.opens, ...limits, screen: true }, services());
    assert.deepEqual(engine.hears, { pointer: true, keys: true });
    const drawn = run.frames.map((f: { tick: number; input: Input[] }) => engine.frame(f.tick, f.input).shapes);
    assert.deepEqual(drawn, run.frames.map((f: { draws: unknown }) => f.draws), run.name);
  }
});

test("each call of a frame is told before it runs, for its time limit", async () => {
  const engine = await loadEngine(screen.name, SCREEN, { uses: [], opens: true, ...limits, screen: true }, services());
  const calls: string[] = [];
  engine.frame(0, [{ key: "down", name: "a" }, { pointer: "down", x: 1, y: 2 }], (call) => calls.push(call));
  assert.deepEqual(calls, ["clarity_key", "clarity_pointer", "clarity_tick", "clarity_draw"]);
});

test("an engine without clarity_pointer or clarity_key hears neither", async () => {
  const engine = await loadEngine("Test", TEST, { uses: cases.uses, opens: false, ...limits }, services());
  assert.deepEqual(engine.hears, { pointer: false, keys: false });
});

test("an engine tells with its answers, and hears through its functions", async () => {
  for (const run of screen.tells) {
    const engine = await loadEngine(screen.name, SCREEN, { uses: [], opens: run.opens, ...limits, screen: true }, services());
    run.frames.forEach((f: { tick: number; input: Input[]; tells: unknown; draws: unknown }, i: number) => {
      assert.deepEqual(engine.frame(f.tick, f.input), { shapes: f.draws, tells: f.tells }, `${run.name}, frame ${i}`);
    });
  }
  // A hear is told before it runs, by its own name.
  const engine = await loadEngine(screen.name, SCREEN, { uses: [], opens: true, ...limits, screen: true }, services());
  const calls: string[] = [];
  engine.frame(0, [{ hear: "start", values: [1] }], (call) => calls.push(call));
  assert.deepEqual(calls, ["start", "clarity_tick", "clarity_draw"]);
});
