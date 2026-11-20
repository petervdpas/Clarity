// Clarity.UI's worker script (runtimes/rust-ui-browser/js/clarity-ui-worker.js),
// as the IDE's webview runs it on the page's thread: when the library's
// wasm stops (the engine's own error, such as "Out of bounds call_indirect"),
// in its start or in a frame, the page hears "failed" once, with what it was
// doing, and the worker draws no more; nothing escapes as an unhandled error.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const source = readFileSync(new URL("../../../runtimes/rust-ui-browser/js/clarity-ui-worker.js", import.meta.url), "utf8");

type Message = { type: string; what?: string };

/** The worker with a stand-in library: `stops` says where its wasm stops. */
function worker(stops: "start" | "frame" | "panic") {
  const frames: (() => void)[] = [];
  const posted: Message[] = [];
  // The options the worker made the library with: its error output among them.
  let made: { printErr?: (line: string) => void } = {};
  const library = {
    stringToNewUTF8: () => 1,
    UTF8ToString: () => "",
    _free: () => {},
    _clarity_ui_string_free: () => {},
    _clarity_ui_start: () => {
      if (stops === "start") throw new WebAssembly.RuntimeError("Out of bounds call_indirect");
      if (stops === "panic") {
        made.printErr?.("panicked: no pool (src/report.rs:12)");
        throw new WebAssembly.RuntimeError("Unreachable code should not be executed");
      }
      return 0;
    },
    _clarity_ui_frame: () => {
      throw new WebAssembly.RuntimeError("Out of bounds call_indirect");
    },
    _clarity_ui_next_wake: () => 0,
    _clarity_ui_resize: () => {},
    GL: { registerContext: () => 1, makeContextCurrent: () => {}, deleteContext: () => {} },
  };
  class Request {
    status = 200;
    responseText = "{}";
    open() {}
    send() {}
    setRequestHeader() {}
  }
  const context: Record<string, unknown> = {
    window: {},
    document: { currentScript: null },
    location: { href: "http://x/" },
    self: {},
    XMLHttpRequest: Request,
    WebAssembly,
    console: { error: () => {}, log: () => {} },
    setTimeout,
    clearTimeout,
  };
  runInNewContext(source + "\n;globalThis.clarityUiWorker = clarityUiWorker;", context);
  const make = context.clarityUiWorker as (scope: object) => (message: object) => void;
  const handle = make({
    post: (message: Message) => posted.push(message),
    load: async () => async (options: typeof made) => ((made = options), library),
    frame: (act: () => void) => frames.push(act),
    base: "http://x/",
  });
  const canvas = { width: 0, height: 0, getContext: () => ({}) };
  return { handle, canvas, posted, frames };
}

const settle = () => new Promise((r) => setTimeout(r, 20));

test("a start the wasm stops in says failed, with what it was doing", async () => {
  const w = worker("start");
  w.handle({ type: "start", canvas: w.canvas, width: 800, height: 600, dpr: 1, files: "http://x/files.json" });
  await settle();
  const failed = w.posted.filter((m) => m.type === "failed");
  assert.equal(failed.length, 1, JSON.stringify(w.posted));
  assert.match(failed[0].what ?? "", /starting the window/);
  assert.match(failed[0].what ?? "", /Out of bounds call_indirect/);
  assert.ok(!w.posted.some((m) => m.type === "started"));
});

test("a frame the wasm stops in says failed once and draws no more", async () => {
  const w = worker("frame");
  w.handle({ type: "start", canvas: w.canvas, width: 800, height: 600, dpr: 1, files: "http://x/files.json" });
  await settle();
  assert.ok(w.posted.some((m) => m.type === "started"));
  for (let i = 0; i < 3 && w.frames.length; i++) w.frames.shift()!();
  const failed = w.posted.filter((m) => m.type === "failed");
  assert.equal(failed.length, 1, JSON.stringify(w.posted));
  assert.match(failed[0].what ?? "", /drawing/);
  assert.equal(w.frames.length, 0, "no more frames asked for");
  w.handle({ type: "resize", width: 10, height: 10, dpr: 1 });
  assert.equal(w.posted.filter((m) => m.type === "failed").length, 1, "a stopped library is not called again");
});

test("an engine that panics and stops says its panic's words and place", async () => {
  const w = worker("panic");
  w.handle({ type: "start", canvas: w.canvas, width: 800, height: 600, dpr: 1, files: "http://x/files.json" });
  await settle();
  const failed = w.posted.filter((m) => m.type === "failed");
  assert.equal(failed.length, 1, JSON.stringify(w.posted));
  assert.match(failed[0].what ?? "", /Unreachable code/);
  assert.match(failed[0].what ?? "", /panicked: no pool \(src\/report\.rs:12\)/);
});
