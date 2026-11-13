// The project's Agents part (docs/ledger/app-mcp.md): what the app offers AI
// agents through its MCP service. The commands marked `agent`, the guidance
// in agent/guidance.md, and what agents may do while the app is closed.

import { commandLine } from "./commandLine.ts";

type Command = { group?: { name: string } | null; name: { name: string }; agent: boolean };

/** The commands offered to agents, as typed after the program's name, and how many others there are. */
export function agentCommands(files: readonly { commands: readonly Command[] }[]): { offered: string[]; others: number } {
  const all = files.flatMap((f) => f.commands);
  return {
    offered: all.filter((c) => c.agent).map((c) => commandLine(c.group?.name ?? null, c.name.name)),
    others: all.filter((c) => !c.agent).length,
  };
}

type Placed = Command & { span: { start: number } };

/** Every command as a row of the Agents part: as typed, whether it is for agents, and its file and place, for the edit that ticks it. */
export function commandRows(files: readonly { path: string; commands: readonly Placed[] }[]): { line: string; agent: boolean; path: string; item: number }[] {
  return files.flatMap((f) => f.commands.map((c) => ({ line: commandLine(c.group?.name ?? null, c.name.name), agent: c.agent, path: f.path, item: c.span.start })));
}

/** Whether the app offers agents anything yet: a command for agents, or its guidance. */
export function offersAgents(files: readonly { commands: readonly Command[] }[], guidance: string | null): boolean {
  return guidance !== null || files.some((f) => f.commands.some((c) => c.agent));
}

type Settings = { path: string; entities: readonly { settings: boolean; fields: readonly { name: { name: string } }[] }[] };

/** Whether the app's settings offer the switches for while the app is closed, and the file
 *  whose settings block takes them (the one that has it, else the first). */
export function agentSettings(files: readonly Settings[]): { reads: boolean; changes: boolean; file: string } {
  const holder = files.find((f) => f.entities.some((e) => e.settings));
  const names = holder?.entities.filter((e) => e.settings).flatMap((e) => e.fields.map((f) => f.name.name)) ?? [];
  return { reads: names.includes("agentReadsWhileClosed"), changes: names.includes("agentChangesWhileClosed"), file: (holder ?? files[0])?.path ?? "" };
}
