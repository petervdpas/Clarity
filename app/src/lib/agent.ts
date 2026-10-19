// The agent panel's list of what was said and done, built from the events
// of a turn, and the line each tool use shows as.

import type { AgentEvent } from "./generated/AgentEvent";
import type { ConnectionRow } from "./generated/ConnectionRow";

export type Item =
  | { kind: "user"; text: string }
  | { kind: "assistant"; text: string }
  | { kind: "tool"; id: string; name: string; input: unknown; result?: string; isError?: boolean }
  | { kind: "note"; text: string }
  | { kind: "error"; text: string };

/** The list after one event of a turn. */
export function applyEvent(items: Item[], event: AgentEvent): Item[] {
  const last = items.at(-1);
  switch (event.kind) {
    case "text":
      if (last?.kind === "assistant") return [...items.slice(0, -1), { kind: "assistant", text: last.text + event.text }];
      return [...items, { kind: "assistant", text: event.text }];
    case "toolStart":
      return [...items, { kind: "tool", id: event.id, name: event.name, input: event.input }];
    case "toolDone":
      return items.map((i) => (i.kind === "tool" && i.id === event.id ? { ...i, result: event.text, isError: event.isError } : i));
    case "stopped":
      return [...items, { kind: "note", text: "Stopped." }];
    case "failed":
      return [...items, { kind: "error", text: event.message }];
    default:
      return items;
  }
}

const text = (input: unknown, key: string): string => {
  const value = (input as Record<string, unknown> | null)?.[key];
  return typeof value === "string" ? value : "";
};

/** What a tool use did, in a few words. */
export function toolLine(name: string, input: unknown): string {
  const path = text(input, "path");
  const file = path === "project" ? "the project file" : path;
  switch (name) {
    case "write_file":
      return `Wrote ${file}`;
    case "read_file":
      return `Read ${file}`;
    case "project_overview":
      return "Looked at the project";
    case "check":
      return "Checked the project";
    case "parts":
      return "Looked up the parts";
    case "migrate":
      return `Wrote a migration step${text(input, "title") ? `: ${text(input, "title")}` : ""}`;
    case "new_project":
      return `Made the project ${text(input, "name")}`;
    case "design":
      return text(input, "section") ? `Read the design: ${text(input, "section")}` : "Read the design's contents";
    case "show_window":
      return `Showed ${text(input, "name")}`;
    default:
      return name;
  }
}

/** The connections the agent can talk to: those with a `kind` field, and
 *  the one named anthropic, which needs none. Anthropic's comes first. */
export function agentConnections(rows: ConnectionRow[]): string[] {
  const named = (r: ConnectionRow) => r.name.toLowerCase() === "anthropic";
  return rows
    .filter((r) => named(r) || r.fields.some(([field]) => field === "kind"))
    .sort((a, b) => Number(named(b)) - Number(named(a)))
    .map((r) => r.name);
}
