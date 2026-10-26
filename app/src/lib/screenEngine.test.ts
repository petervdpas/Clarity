// The frame loop of an engine beside the screen, on the page (DESIGN 5.12):
// the page asks for frames at the screen's pace and the worker answers each
// with its shapes; a frame is skipped while the last is busy, and a worker
// that runs past its time limit is stopped from outside
// (conformance/screen-engines.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { CanvasHears, HeldKeys, keyName, queue, ScreenRun, type FromWorker, type ToWorker, type WorkerLike } from "./screenEngine.ts";
import { pointNear } from "./drawing.ts";
import type { Input } from "./engineHost.ts";
import type { ScreenEngine } from "./generated/ScreenEngine";

const screen = JSON.parse(readFileSync(new URL("../../../conformance/screen-engines.json", import.meta.url), "utf8"));
const ENGINE: ScreenEngine = { name: "Screen", wasm: "AGFzbQ==", uses: [], opens: true, seconds: 1, megabytes: 16 };

/** A worker the test answers for, a screen whose frames it steps, and timers it fires. */
function harness(engine = ENGINE) {
  const sent: ToWorker[] = [];
  let terminated = false;
  const worker: WorkerLike = { postMessage: (m) => sent.push(m), terminate: () => (terminated = true), onmessage: null, onerror: null };
  let frame: ((now: number) => void) | null = null;
  const timers = new Map<number, () => void>();
  let timerId = 0;
  const drawn: unknown[] = [];
  const failed: string[] = [];
  const heard: unknown[] = [];
  const told: unknown[] = [];
  const run = new ScreenRun(engine, {
    onTell: (tell, values) => told.push([tell, values]),
    onReady: (hears) => heard.push(hears),
    worker: () => worker,
    frames: { request: (f) => ((frame = f), 1), cancel: () => (frame = null) },
    timers: { set: (f) => (timers.set(++timerId, f), timerId), clear: (id) => timers.delete(id) },
    language: () => "nl",
    onDraw: (shapes) => drawn.push(shapes),
    onFail: (message) => failed.push(message),
    onLog: () => {},
  });
  return {
    run,
    sent,
    drawn,
    failed,
    heard,
    told,
    terminated: () => terminated,
    answer: (m: FromWorker) => worker.onmessage?.({ data: m }),
    crash: (message: string) => worker.onerror?.({ message }),
    step: (now: number) => {
      const f = frame;
      frame = null;
      f?.(now);
    },
    waiting: () => frame !== null,
    expire: () => [...timers.values()].forEach((f) => f()),
    timers: () => timers.size,
  };
}

test("it starts the engine, then asks for a frame at the screen's pace", () => {
  const h = harness();
  h.run.start();
  assert.deepEqual(h.sent, [{ kind: "start", engine: ENGINE, language: "nl" }]);
  assert.ok(!h.waiting(), "no frame before the engine is ready");
  h.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  h.step(1000);
  // The first frame gives no time; each next one the time since the last.
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 0, input: [], language: "nl" });
  h.answer({ kind: "drawn", shapes: [{ op: "clear", color: "background" }], tells: [] });
  assert.deepEqual(h.drawn, [[{ op: "clear", color: "background" }]]);
  h.step(1016);
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 16, input: [], language: "nl" });
});

test("a frame is skipped while the last is busy, and its time is not lost", () => {
  const h = harness();
  h.run.start();
  h.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  h.step(1000);
  const frames = h.sent.length;
  h.step(1016);
  h.step(1033);
  assert.equal(h.sent.length, frames, "nothing is sent while the engine is busy");
  h.answer({ kind: "drawn", shapes: [], tells: [] });
  h.step(1050);
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 50, input: [], language: "nl" });
});

test("a frame that runs past the time limit stops the engine", () => {
  const h = harness();
  h.run.start();
  h.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  h.step(1000);
  h.expire();
  assert.ok(h.terminated());
  assert.deepEqual(h.failed, ["engine Screen failed in clarity_tick: it ran longer than 1 second"]);
  assert.ok(!h.waiting(), "no more frames");
  // Once ticked, the time limit is the draw's.
  const d = harness({ ...ENGINE, seconds: 2 });
  d.run.start();
  d.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  d.step(1000);
  d.answer({ kind: "calling", call: "clarity_draw" });
  d.expire();
  assert.deepEqual(d.failed, ["engine Screen failed in clarity_draw: it ran longer than 2 seconds"]);
});

test("on open has its time limit too", () => {
  const h = harness();
  h.run.start();
  h.expire();
  assert.deepEqual(h.failed, ["engine Screen failed in clarity_open: it ran longer than 1 second"]);
  assert.ok(h.terminated());
});

test("a frame that fails ends the engine; stopping ends it too", () => {
  const h = harness();
  h.run.start();
  h.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  h.step(1000);
  h.answer({ kind: "failed", message: "engine Screen failed in clarity_tick: a frame of more than ten seconds" });
  assert.deepEqual(h.failed, ["engine Screen failed in clarity_tick: a frame of more than ten seconds"]);
  assert.ok(h.terminated() && !h.waiting());
  assert.equal(h.timers(), 0);
  const s = harness();
  s.run.start();
  s.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  s.run.stop();
  assert.ok(s.terminated() && !s.waiting());
  // A late answer after it stopped draws nothing.
  s.answer({ kind: "drawn", shapes: [], tells: [] });
  assert.deepEqual(s.drawn, []);
});

test("a worker that cannot run says why", () => {
  const h = harness();
  h.run.start();
  h.crash("SyntaxError: no such thing");
  assert.deepEqual(h.failed, ["engine Screen failed in clarity_open: SyntaxError: no such thing"]);
  assert.ok(h.terminated());
});

test("the worker gets a plain copy of the engine, and a message it cannot take ends the run", () => {
  const h = harness();
  h.run.start();
  const sent = h.sent[0] as { engine: ScreenEngine };
  assert.notEqual(sent.engine, ENGINE, "a copy: a reactive proxy cannot cross to a worker");
  assert.deepEqual(sent.engine, ENGINE);
  const failed: string[] = [];
  const refusing: WorkerLike = {
    postMessage: () => {
      throw new Error("DataCloneError: the object can not be cloned");
    },
    terminate: () => {},
    onmessage: null,
    onerror: null,
  };
  const run = new ScreenRun(ENGINE, {
    worker: () => refusing,
    frames: { request: () => 1, cancel: () => {} },
    timers: { set: () => 1, clear: () => {} },
    language: () => "",
    onDraw: () => {},
    onFail: (m) => failed.push(m),
    onLog: () => {},
  });
  run.start();
  assert.deepEqual(failed, ["engine Screen failed in clarity_open: DataCloneError: the object can not be cloned"]);
});

test("pointer and keys wait for the next frame, and go with it", () => {
  const h = harness();
  h.run.start();
  h.run.pointer("down", 1, 1);
  h.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  assert.deepEqual(h.heard, [{ pointer: true, keys: true }]);
  h.run.pointer("down", 4, 5);
  h.run.key("down", "a");
  h.step(1000);
  // What came before the engine was ready is not kept.
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 0, input: [{ pointer: "down", x: 4, y: 5 }, { key: "down", name: "a" }], language: "nl" });
  // While the frame is busy, input waits for the next one.
  h.run.pointer("move", 6, 7);
  h.step(1016);
  h.answer({ kind: "drawn", shapes: [], tells: [] });
  h.step(1033);
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 33, input: [{ pointer: "move", x: 6, y: 7 }], language: "nl" });
  // The time limit is the first call's.
  h.run.key("up", "a");
  h.answer({ kind: "drawn", shapes: [], tells: [] });
  h.step(1050);
  h.expire();
  assert.deepEqual(h.failed, ["engine Screen failed in clarity_key: it ran longer than 1 second"]);
});

test("input an engine does not hear is not sent", () => {
  const h = harness();
  h.run.start();
  h.answer({ kind: "ready", hears: { pointer: false, keys: true } });
  h.run.pointer("down", 1, 1);
  h.run.key("down", "a");
  h.step(1000);
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 0, input: [{ key: "down", name: "a" }], language: "nl" });
});

test("moves in a row keep only the last", () => {
  for (const c of screen.queued) {
    const kept: Input[] = [];
    for (const input of c.given) queue(kept, input);
    assert.deepEqual(kept, c.sent);
  }
});

test("a pointer that is down is at the drawing's nearest edge beside it", () => {
  for (const c of screen.near) {
    const got = pointNear(screen.units, c.part, c.at);
    assert.deepEqual(got, c.x === null ? null : { x: c.x, y: c.y }, JSON.stringify(c));
  }
});

test("keys go by Clarity's names, once down and once up", () => {
  // The page's own names of the keys, which keyName gives Clarity's for.
  const page = (name: string) => (name === "Space" ? " " : name.length === 1 ? name.toUpperCase() : name);
  for (const name of screen.keys) {
    assert.equal(keyName(page(name)), name);
    assert.equal(keyName(name === "Space" ? " " : name), name);
  }
  for (const other of ["Tab", "Meta", "é", "Dead", "+", "Unidentified"]) assert.equal(keyName(other), null, other);
  const held = new HeldKeys();
  assert.equal(held.down("a"), true);
  assert.equal(held.down("a"), false, "a key that repeats is not sent again");
  assert.equal(held.down("ArrowLeft"), true);
  assert.equal(held.up("a"), true);
  assert.equal(held.up("a"), false);
  assert.deepEqual(held.all(), ["ArrowLeft"], "losing the keyboard lets go of every key still down");
  assert.deepEqual(held.all(), []);
});

test("a hear waits for the next frame, even before the engine is ready", () => {
  const h = harness();
  h.run.start();
  h.run.hear("start", [2]);
  h.answer({ kind: "ready", hears: { pointer: false, keys: false } });
  h.step(1000);
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 0, input: [{ hear: "start", values: [2] }], language: "nl" });
  // Its time limit is its own.
  h.expire();
  assert.deepEqual(h.failed, ["engine Screen failed in start: it ran longer than 1 second"]);
});

test("what the engine tells goes to the window once its frame is drawn, in order", () => {
  const h = harness();
  h.run.start();
  h.answer({ kind: "ready", hears: { pointer: true, keys: true } });
  h.step(1000);
  h.answer({ kind: "drawn", shapes: [], tells: [{ tell: "lap", values: [1] }, { tell: "finished", values: [1700] }] });
  assert.deepEqual(h.told, [["lap", [1]], ["finished", [1700]]]);
  assert.equal(h.drawn.length, 1);
});

test("hears go to the canvas of their window, and wait while it does not show", () => {
  const hears = new CanvasHears();
  const got: unknown[] = [];
  // Told before the canvas shows: an `on open` handler's reply comes first.
  hears.hear(3, "sky", "start", [1]);
  const stop = hears.listen(3, "sky", (hear, values) => got.push([hear, values]));
  assert.deepEqual(got, [["start", [1]]]);
  hears.hear(3, "sky", "start", [2]);
  hears.hear(4, "sky", "start", [9]);
  assert.deepEqual(got, [["start", [1]], ["start", [2]]], "another window's canvas is its own");
  stop();
  hears.hear(3, "sky", "start", [5]);
  assert.equal(got.length, 2);
  // A window that closed keeps nothing.
  hears.closed(3);
  hears.listen(3, "sky", (hear, values) => got.push([hear, values]));
  assert.equal(got.length, 2);
  hears.listen(4, "sky", (hear, values) => got.push([hear, values]));
  assert.deepEqual(got.at(-1), ["start", [9]]);
});
