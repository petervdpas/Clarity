// What the status bar at the foot of Clarity says.

import { test } from "node:test";
import assert from "node:assert/strict";
import { buildStatus, counted, openLabel } from "./statusBar.ts";

test("what is open is named with its kind", () => {
  assert.equal(openLabel({ kind: "window", name: "Main" }), "Window Main");
  assert.equal(openLabel({ kind: "app", name: "Shop" }), "App Shop");
  assert.equal(openLabel({ kind: "settings", name: "" }), "Project settings");
  assert.equal(openLabel({ kind: "api", name: "shop" }), "Web API shop");
  assert.equal(openLabel(null), "Nothing open");
});

test("a count says one or more", () => {
  assert.equal(counted(0, "error", "errors"), "0 errors");
  assert.equal(counted(1, "error", "errors"), "1 error");
  assert.equal(counted(3, "warning", "warnings"), "3 warnings");
});

test("the build state follows the build", () => {
  const idle = { building: false, commandRunning: false, running: false, done: null };
  assert.deepEqual(buildStatus(idle), { text: "Ready", tone: "idle" });
  assert.deepEqual(buildStatus({ ...idle, building: true }), { text: "Building...", tone: "busy" });
  assert.deepEqual(buildStatus({ ...idle, building: true, commandRunning: true }), { text: "Running the command line", tone: "busy" });
  assert.deepEqual(buildStatus({ ...idle, running: true, done: { ok: true, message: "running" } }), { text: "Running", tone: "busy" });
  assert.deepEqual(buildStatus({ ...idle, done: { ok: true, message: "Built in 3 s" } }), { text: "Built in 3 s", tone: "ok" });
  assert.deepEqual(buildStatus({ ...idle, done: { ok: false, message: "cargo failed" } }), { text: "Build failed: cargo failed", tone: "failed" });
});
