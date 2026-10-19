// The agent panel's list: what the person said, the answer as it streams,
// and each tool the agent used, with its result.

import { test } from "node:test";
import assert from "node:assert/strict";

import { addUsage, agentConnections, applyEvent, NO_USAGE, tokens, toolLine, usageLine, type Item } from "./agent.ts";
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

test("reading the design document shows which part", () => {
  assert.equal(toolLine("design", {}), "Read the design's contents");
  assert.equal(toolLine("design", { section: "roadmap" }), "Read the design: roadmap");
});

test("tokens add up per turn and per session, and read briefly", () => {
  const first = { input: 30, cacheWrite: 18000, cacheRead: 0, output: 400 };
  const second = { input: 50, cacheWrite: 600, cacheRead: 18000, output: 250 };
  const turn = addUsage(addUsage(NO_USAGE, first), second);
  assert.deepEqual(turn, { input: 80, cacheWrite: 18600, cacheRead: 18000, output: 650 });
  assert.equal(tokens(950), "950");
  assert.equal(tokens(18_600), "18.6k");
  assert.equal(tokens(2_000), "2k");
  assert.equal(tokens(1_250_000), "1.3M");
  // The whole prompt, how much of it the cache served, and the answers.
  assert.equal(usageLine(turn, addUsage(turn, first)), "This turn 36.7k in (18k from the cache), 650 out. Session 54.7k in, 1.1k out.");
  // Before any answer there is nothing to tell.
  assert.equal(usageLine(NO_USAGE, NO_USAGE), "");
});

test("reading the manual and the reference shows which page", () => {
  assert.equal(toolLine("manual", { topic: "code.queries" }), "Read the manual: code.queries");
  assert.equal(toolLine("reference", { section: "Queries" }), "Read the reference: Queries");
  assert.equal(toolLine("reference", {}), "Read the reference's contents");
});
