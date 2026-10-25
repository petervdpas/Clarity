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
