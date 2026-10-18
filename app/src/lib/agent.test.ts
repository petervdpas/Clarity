// The agent panel's list: what the person said, the answer as it streams,
// and each tool the agent used, with its result.

import { test } from "node:test";
import assert from "node:assert/strict";

import { agentConnections, applyEvent, toolLine, type Item } from "./agent.ts";
import type { ConnectionRow } from "./generated/ConnectionRow";

test("text pieces join the answer, and tools show with their results", () => {
  let items: Item[] = [{ kind: "user", text: "Make a todo app" }];
  items = applyEvent(items, { kind: "text", text: "I will " });
  items = applyEvent(items, { kind: "text", text: "start." });
  items = applyEvent(items, { kind: "toolStart", id: "t1", name: "write_file", input: { path: "todo.clarity", text: "module todo" } });
  items = applyEvent(items, { kind: "toolDone", id: "t1", name: "write_file", text: "Wrote todo.clarity. No problems.", isError: false });
  items = applyEvent(items, { kind: "text", text: "Done." });
  items = applyEvent(items, { kind: "finished" });
  assert.deepEqual(items, [
    { kind: "user", text: "Make a todo app" },
    { kind: "assistant", text: "I will start." },
    { kind: "tool", id: "t1", name: "write_file", input: { path: "todo.clarity", text: "module todo" }, result: "Wrote todo.clarity. No problems.", isError: false },
    { kind: "assistant", text: "Done." },
  ]);
});

test("a stop and a failure leave a note", () => {
  assert.deepEqual(applyEvent([], { kind: "stopped" }), [{ kind: "note", text: "Stopped." }]);
  assert.deepEqual(applyEvent([], { kind: "failed", message: "the model refused" }), [{ kind: "error", text: "the model refused" }]);
});

test("a tool line says what the tool did to what", () => {
  assert.equal(toolLine("write_file", { path: "todo.clarity", text: "x" }), "Wrote todo.clarity");
  assert.equal(toolLine("read_file", { path: "project" }), "Read the project file");
  assert.equal(toolLine("migrate", { title: "Tasks" }), "Wrote a migration step: Tasks");
  assert.equal(toolLine("new_project", { name: "todo" }), "Made the project todo");
  assert.equal(toolLine("show_window", { name: "TaskBrowse" }), "Showed TaskBrowse");
  assert.equal(toolLine("check", {}), "Checked the project");
});

test("the agent picks from connections that name a model kind, anthropic first", () => {
  const rows = [
    { name: "shop", fields: [["host", { mode: "plain", value: "db" }]] },
    { name: "local", fields: [["kind", { mode: "plain", value: "openai" }]] },
    { name: "anthropic", fields: [["apikey", { mode: "vault", category: "AI", key: "anthropic" }]] },
  ] as ConnectionRow[];
  assert.deepEqual(agentConnections(rows), ["anthropic", "local"]);
  assert.deepEqual(agentConnections([]), []);
});
