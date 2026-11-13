// The project's Agents part: whether the project tree shows it, and the commands it offers to AI agents.

import { test } from "node:test";
import assert from "node:assert/strict";
import { agentCommands, agentSettings, offersAgents } from "./agentsPart.ts";

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
