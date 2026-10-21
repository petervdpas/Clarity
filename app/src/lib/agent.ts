// The agent panel's list of what was said and done, built from the events
// of a turn, and the line each tool use shows as.

import type { AgentAnswered } from "./generated/AgentAnswered";
import type { AgentAsk } from "./generated/AgentAsk";
import type { AgentEvent } from "./generated/AgentEvent";
import type { ConnectionRow } from "./generated/ConnectionRow";
import type { Runs } from "./generated/Runs";
import type { Usage } from "./generated/Usage";

export type Item =
  | { kind: "user"; text: string }
  | { kind: "assistant"; text: string }
  | { kind: "tool"; id: string; name: string; input: unknown; result?: string; isError?: boolean }
  | { kind: "note"; text: string }
  | { kind: "error"; text: string }
  /** The agent asks whether it may run something; `allowed` once answered. */
  | { kind: "ask"; id: number; what: string; allowed?: boolean };

/** What the agent may run, as Clarity's settings (Agent) offer it. */
export const RUNS_CHOICES: { value: Runs; label: string; note: string }[] = [
  { value: "ask", label: "Ask every time", note: "The agent asks before it runs or builds the application; you allow or deny it." },
  { value: "allow", label: "Allow without asking", note: "The agent runs and builds the application when its task needs it." },
  { value: "never", label: "Never", note: "The agent cannot run or build; it asks you to press Run or Build." },
];

export function isRuns(value: unknown): value is Runs {
  return RUNS_CHOICES.some((c) => c.value === value);
}

/** The list with the agent's question at the end. */
export function applyAsk(items: Item[], ask: AgentAsk): Item[] {
  return [...items, { kind: "ask", id: ask.id, what: ask.what }];
}

/** The list with the answer on its question. */
export function applyAnswer(items: Item[], answered: AgentAnswered): Item[] {
  return items.map((i) => (i.kind === "ask" && i.id === answered.id ? { ...i, allowed: answered.allowed } : i));
}

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
    case "overBudget":
      return [...items, { kind: "note", text: `Stopped at the turn's budget: ${tokens(event.spent)} tokens at full price, of ${tokens(event.budget)}. Ask again to go on.` }];
    default:
      return items;
  }
}

/** No tokens yet. */
export const NO_USAGE: Usage = { input: 0, cacheWrite: 0, cacheRead: 0, output: 0 };

export function addUsage(a: Usage, b: Usage): Usage {
  return { input: a.input + b.input, cacheWrite: a.cacheWrite + b.cacheWrite, cacheRead: a.cacheRead + b.cacheRead, output: a.output + b.output };
}

/** A number of tokens in a few characters: 950, 18.6k, 1.3M. */
export function tokens(n: number): string {
  const short = (value: number, unit: string) => `${value.toFixed(1).replace(/\.0$/, "")}${unit}`;
  if (n >= 1_000_000) return short(n / 1_000_000, "M");
  if (n >= 1_000) return short(n / 1_000, "k");
  return String(n);
}

/** What the turn and the session cost so far: the whole prompts, what the
 *  cache served of the turn's, and the answers. Empty before any answer. */
export function usageLine(turn: Usage, session: Usage): string {
  if (session.output === 0 && session.input === 0) return "";
  const prompt = (u: Usage) => u.input + u.cacheWrite + u.cacheRead;
  const cached = turn.cacheRead > 0 ? ` (${tokens(turn.cacheRead)} from the cache)` : "";
  return `This turn ${tokens(prompt(turn))} in${cached}, ${tokens(turn.output)} out. Session ${tokens(prompt(session))} in, ${tokens(session.output)} out.`;
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
    case "manual":
      return `Read the manual: ${text(input, "topic")}`;
    case "reference":
      return text(input, "section") ? `Read the reference: ${text(input, "section")}` : "Read the reference's contents";
    case "build_output":
      return "Read the build output";
    case "read_generated":
      return path ? `Read the generated ${path}` : "Listed the generated files";
    case "build":
      return (input as { run?: boolean } | null)?.run ? "Ran the application" : "Built the application";
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
