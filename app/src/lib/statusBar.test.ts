// What the status bar at the foot of Clarity says.

import { test } from "node:test";
import assert from "node:assert/strict";
import { AGENT_CALL_SHOWN_SECONDS, agentCall, buildStatus, counted, openLabel } from "./statusBar.ts";

test("what is open is named with its kind", () => {
  assert.equal(openLabel({ kind: "window", name: "Main" }), "Window Main");
  assert.equal(openLabel({ kind: "app", name: "Shop" }), "App Shop");
  assert.equal(openLabel({ kind: "settings", name: "" }), "Project settings");
  assert.equal(openLabel({ kind: "api", name: "shop" }), "Web API shop");
  assert.equal(openLabel({ kind: "readme", name: "" }), "README");
  assert.equal(openLabel({ kind: "signin", name: "" }), "Sign-in");
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

test("an outside agent's last call shows for two minutes", () => {
  const call = { seq: 7, time: 1000, client: "claude-code", tool: "add_part", path: "design/Main.cxaml", ok: true, said: "Added textbox emailBox to Main." };
  assert.deepEqual(agentCall(call, 1060), {
    text: "claude-code: add_part",
    ok: true,
    title: "claude-code called add_part (design/Main.cxaml): Added textbox emailBox to Main. Opens the agent activity.",
  });
  assert.equal(agentCall(call, 1000 + AGENT_CALL_SHOWN_SECONDS + 1), null, "gone after two minutes without calls");
  assert.equal(agentCall({ ...call, ok: false, path: null }, 1001)?.ok, false, "a refused call shows as refused");
  assert.equal(agentCall(null, 1001), null);
  assert.equal(agentCall({ ...call, client: "" }, 1001)?.text, "Agent: add_part", "a program that gave no name");
});
