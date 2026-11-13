// The project's Agents part: whether the project tree shows it, and the commands it offers to AI agents.

import { test } from "node:test";
import assert from "node:assert/strict";
import { agentCommands, agentSettings, commandRows, offersAgents } from "./agentsPart.ts";

function command(group: string | null, name: string, agent: boolean) {
  return { group: group ? { name: group } : null, name: { name }, agent };
}

test("the commands for agents are named as typed, the others counted", () => {
  const files = [{ commands: [command("project", "add", true), command(null, "backup", false)] }, { commands: [command("review", "setAll", true)] }];
  assert.deepEqual(agentCommands(files), { offered: ["project add", "review set-all"], others: 1 });
});

test("the part is there once a command is for agents or the guidance is written", () => {
  assert.equal(offersAgents([{ commands: [command(null, "backup", false)] }], null), false);
  assert.equal(offersAgents([{ commands: [command(null, "backup", true)] }], null), true);
  assert.equal(offersAgents([{ commands: [] }], "# Shop for agents\n"), true);
});

test("the part says which switches the app's settings offer, and in which file", () => {
  const field = (name: string) => ({ name: { name } });
  const files = [
    { path: "a.clarity", entities: [{ settings: false, fields: [field("agentReadsWhileClosed")] }] },
    { path: "b.clarity", entities: [{ settings: true, fields: [field("language"), field("agentChangesWhileClosed")] }] },
  ];
  assert.deepEqual(agentSettings(files), { reads: false, changes: true, file: "b.clarity" });
  assert.deepEqual(agentSettings([{ path: "only.clarity", entities: [] }]), { reads: false, changes: false, file: "only.clarity" });
});

test("every command is a row with its file and place, ticked when it is for agents", () => {
  const at = (c: ReturnType<typeof command>, start: number) => ({ ...c, span: { start } });
  const files = [
    { path: "shop.clarity", commands: [at(command("project", "add", true), 10), at(command(null, "backup", false), 40)] },
    { path: "review.clarity", commands: [at(command("review", "setAll", false), 7)] },
  ];
  assert.deepEqual(commandRows(files), [
    { line: "project add", agent: true, path: "shop.clarity", item: 10 },
    { line: "backup", agent: false, path: "shop.clarity", item: 40 },
    { line: "review set-all", agent: false, path: "review.clarity", item: 7 },
  ]);
});
