// The frame loop of an engine beside the screen, on the page (DESIGN 5.12):
// the page asks for frames at the screen's pace and the worker answers each
// with its shapes; a frame is skipped while the last is busy, and a worker
// that runs past its time limit is stopped from outside
// (conformance/screen-engines.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { ScreenRun, type FromWorker, type ToWorker, type WorkerLike } from "./screenEngine.ts";
import type { ScreenEngine } from "./generated/ScreenEngine";

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
  const run = new ScreenRun(engine, {
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
  h.answer({ kind: "ready" });
  h.step(1000);
  // The first frame gives no time; each next one the time since the last.
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 0, language: "nl" });
  h.answer({ kind: "drawn", shapes: [{ op: "clear", color: "background" }] });
  assert.deepEqual(h.drawn, [[{ op: "clear", color: "background" }]]);
  h.step(1016);
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 16, language: "nl" });
});

test("a frame is skipped while the last is busy, and its time is not lost", () => {
  const h = harness();
  h.run.start();
  h.answer({ kind: "ready" });
  h.step(1000);
  const frames = h.sent.length;
  h.step(1016);
  h.step(1033);
  assert.equal(h.sent.length, frames, "nothing is sent while the engine is busy");
  h.answer({ kind: "drawn", shapes: [] });
  h.step(1050);
  assert.deepEqual(h.sent.at(-1), { kind: "frame", ms: 50, language: "nl" });
});

test("a frame that runs past the time limit stops the engine", () => {
  const h = harness();
  h.run.start();
  h.answer({ kind: "ready" });
  h.step(1000);
  h.expire();
  assert.ok(h.terminated());
  assert.deepEqual(h.failed, ["engine Screen failed in clarity_tick: it ran longer than 1 second"]);
  assert.ok(!h.waiting(), "no more frames");
  // Once ticked, the time limit is the draw's.
  const d = harness({ ...ENGINE, seconds: 2 });
  d.run.start();
  d.answer({ kind: "ready" });
  d.step(1000);
  d.answer({ kind: "ticked" });
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
  h.answer({ kind: "ready" });
  h.step(1000);
  h.answer({ kind: "failed", message: "engine Screen failed in clarity_tick: a frame of more than ten seconds" });
  assert.deepEqual(h.failed, ["engine Screen failed in clarity_tick: a frame of more than ten seconds"]);
  assert.ok(h.terminated() && !h.waiting());
  assert.equal(h.timers(), 0);
  const s = harness();
  s.run.start();
  s.answer({ kind: "ready" });
  s.run.stop();
  assert.ok(s.terminated() && !s.waiting());
  // A late answer after it stopped draws nothing.
  s.answer({ kind: "drawn", shapes: [] });
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
