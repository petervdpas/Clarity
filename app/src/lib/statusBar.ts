// What the status bar at the foot of Clarity says: what is open, how the
// build is doing, and counts. The bar itself is StatusBar.svelte.

import type { Selected } from "./selection";

/** The kinds of things that open, as the bar names them. */
const KIND_NAMES: Record<Selected["kind"], string> = {
  window: "Window",
  app: "App",
  entity: "Entity",
  enum: "Enum",
  engine: "Engine",
  part: "Part",
  library: "Library",
  theme: "Theme",
  language: "Language",
  resource: "Resource",
  diagram: "Diagram",
  module: "Module",
  settings: "Project settings",
  api: "Web API",
  readme: "README",
};

/** What is open, like `Window Main`. */
export function openLabel(open: Selected | null): string {
  if (!open) return "Nothing open";
  const kind = KIND_NAMES[open.kind];
  return open.name ? `${kind} ${open.name}` : kind;
}

/** `1 error`, `3 errors`. */
export function counted(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

export type BuildTone = "idle" | "busy" | "ok" | "failed";

/** The build as the bar shows it, from the state the Output panel keeps. */
export function buildStatus(state: {
  building: boolean;
  commandRunning: boolean;
  running: boolean;
  done: { ok: boolean; message: string } | null;
}): { text: string; tone: BuildTone } {
  if (state.commandRunning) return { text: "Running the command line", tone: "busy" };
  if (state.building) return { text: "Building...", tone: "busy" };
  if (state.running) return { text: "Running", tone: "busy" };
  if (state.done && !state.done.ok) return { text: `Build failed: ${state.done.message}`, tone: "failed" };
  if (state.done && state.done.message !== "running") return { text: state.done.message, tone: "ok" };
  return { text: "Ready", tone: "idle" };
}

/** How long an outside agent's last call stays in the bar without a new one. */
export const AGENT_CALL_SHOWN_SECONDS = 120;

/** An outside agent's call, as the activity list has it (AgentActivity). */
export type AgentCall = {
  time: number;
  client: string;
  tool: string;
  path: string | null;
  ok: boolean;
  said: string;
};

/** The last call an outside agent made through `clarity mcp`, as the bar shows it at `now`
 *  (seconds since 1970); none when there is none or it is older than two minutes. */
export function agentCall(call: AgentCall | null, now: number): { text: string; ok: boolean; title: string } | null {
  if (!call || now - call.time > AGENT_CALL_SHOWN_SECONDS) return null;
  const where = call.path ? ` (${call.path})` : "";
  // A program may start without giving its name.
  const client = call.client || "Agent";
  return {
    text: `${client}: ${call.tool}`,
    ok: call.ok,
    title: `${client} called ${call.tool}${where}: ${call.said} Opens the agent activity.`,
  };
}
