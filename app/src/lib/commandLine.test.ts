import { test } from "node:test";
import assert from "node:assert/strict";
import { commandLine, programs, stackLabel } from "./commandLine.ts";

test("Run runs the program a stack makes, and asks when it makes two", () => {
  assert.deepEqual(programs(["desktop"]), ["desktop"]);
  assert.deepEqual(programs(["cli"]), ["cli"]);
  assert.deepEqual(programs(["desktop", "cli"]), ["desktop", "cli"]);
  // The web target builds the windowed program, as desktop does.
  assert.deepEqual(programs(["web"]), ["desktop"]);
  assert.deepEqual(programs(["desktop", "web", "cli"]), ["desktop", "cli"]);
});

test("a stack without windows names no UI", () => {
  assert.equal(stackLabel("Go", "Web renderer", ["Command line"], ["cli"]), "Go, Command line");
  assert.equal(stackLabel("Go", "Web renderer", ["Desktop", "Command line"], ["desktop", "cli"]), "Go, Web renderer, Desktop and Command line");
});

test("a command is named by its command line, camelCase written with hyphens", () => {
  assert.equal(commandLine(null, "backup"), "backup");
  assert.equal(commandLine("customer", "addAll"), "customer add-all");
  assert.equal(commandLine("stockItem", "list"), "stock-item list");
});
