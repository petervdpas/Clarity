// How the renderer reaches the backend of the application it runs in.

import { test } from "node:test";
import assert from "node:assert/strict";
import { isTauri, transport, unwrap } from "./backend.ts";

test("a Tauri window has Tauri's internals; a Wails window does not", () => {
  assert.equal(isTauri({ __TAURI_INTERNALS__: {} }), true);
  assert.equal(isTauri({ _wails: {} }), false);
});

test("the Go shell answers with the value, or with what the screen shows as the error", () => {
  assert.deepEqual(unwrap({ value: { state: 1 } }), { state: 1 });
  assert.equal(unwrap({ value: null }), null);
  const refusal = { text: "Order {n} is over", values: {} };
  assert.throws(() => unwrap({ error: refusal }), (e) => e === refusal);
  assert.throws(() => unwrap({ error: "storage: locked" }), (e) => e === "storage: locked");
});

test("each shell is told by what it puts in the page", () => {
  assert.equal(transport({ __TAURI_INTERNALS__: {} }), "tauri");
  // The C and C++ shells bind one function into the page, through webview.
  assert.equal(transport({ clarityInvoke: () => {} }), "webview");
  assert.equal(transport({}), "wails");
});
