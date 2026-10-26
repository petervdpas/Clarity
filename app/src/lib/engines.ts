// The engines a project declares (DESIGN 5.11), for the project tree and the
// module view: each with its module and the errors found in its declaration,
// such as a file that is missing or lacks a declared function.

import type { EngineDecl } from "./generated/EngineDecl";
import type { EngineFunction } from "./generated/EngineFunction";
import type { Event as PartEvent } from "./generated/Event";
import type { Handler } from "./generated/Handler";
import type { Project } from "./generated/Project";
import type { TellHandler } from "./generated/TellHandler";
import { typeText } from "./windowData.ts";

export type ProjectEngine = {
  engine: EngineDecl;
  /** The module that declares it. */
  module: string;
  /** The errors inside its declaration. */
  problems: string[];
};

export function projectEngines(project: Project): ProjectEngine[] {
  return project.files.flatMap((file) =>
    file.engines.map((engine) => ({
      engine,
      module: file.module?.name ?? "",
      problems: project.diagnostics
        .filter((d) => d.severity === "error" && d.path === file.path && d.span.start >= engine.span.start && d.span.start < engine.span.end)
        .map((d) => d.message),
    })),
  );
}

/** What the project tree says beside an engine: what it is written in, and its functions. */
export function engineNote(engine: EngineDecl): string {
  const kind = engine.body ? "Clarity" : "WebAssembly";
  const count = engine.functions.length;
  return `${kind}, ${count === 1 ? "1 function" : `${count} functions`}`;
}

/** A function of an engine as it is declared: `total(lines: list<Line>) -> money`. */
export function signature(f: EngineFunction): string {
  const params = f.params.map((p) => `${p.name.name}: ${typeText(p.ty.kind)}`).join(", ");
  return `${f.name.name}(${params})` + (f.returns ? ` -> ${typeText(f.returns.kind)}` : "");
}

/** The services an engine may use, in the order Clarity lists them (clarity_spec::engines::SERVICES). */
export const SERVICES = ["clock", "random", "log", "language"] as const;

/** An engine's declaration as its inspector edits it. */
export type EngineForm = {
  name: string;
  file: string;
  uses: string[];
  /** The texts it gives, translated like the project's own. */
  says: string[];
  state: "none" | "app" | "window";
  opens: boolean;
  /** How long one call may run; null keeps the default. */
  seconds: number | null;
  /** How much memory it may grow to; null keeps the default. */
  megabytes: number | null;
  functions: { name: string; params: { name: string; type: string }[]; returns: string }[];
};

export function formOf(decl: EngineDecl): EngineForm {
  return {
    name: decl.name.name,
    file: decl.file,
    uses: decl.uses.map((u) => u.name),
    says: decl.says,
    state: decl.state,
    opens: decl.opens,
    seconds: decl.seconds,
    megabytes: decl.megabytes,
    functions: decl.functions.map((f) => ({
      name: f.name.name,
      params: f.params.map((p) => ({ name: p.name.name, type: typeText(p.ty.kind) })),
      returns: f.returns ? typeText(f.returns.kind) : "",
    })),
  };
}

const STATE_LINES = { none: null, app: "state per app", window: "state per window" } as const;

/** The declaration the form says, as Clarity text, saying only what is not the default. */
export function engineText(form: EngineForm): string {
  const lines = [`engine ${form.name} from ${JSON.stringify(form.file)}`];
  const uses = SERVICES.filter((s) => form.uses.includes(s));
  if (uses.length > 0) lines.push(`uses ${uses.join(", ")}`);
  if (form.says.length > 0) lines.push(`says ${form.says.map((t) => JSON.stringify(t)).join(", ")}`);
  const state = STATE_LINES[form.state];
  if (state) lines.push(state);
  if (form.opens) lines.push("on open");
  const limits = [form.seconds !== null ? `${form.seconds} seconds` : null, form.megabytes !== null ? `${form.megabytes} MB` : null].filter((l) => l !== null);
  if (limits.length > 0) lines.push(`limit ${limits.join(", ")}`);
  for (const f of form.functions) {
    const params = f.params.map((p) => `${p.name}: ${p.type}`).join(", ");
    lines.push(`function ${f.name}(${params})` + (f.returns ? ` -> ${f.returns}` : ""));
  }
  return lines.map((l, i) => (i === 0 ? l : "    " + l)).join("\n") + "\n";
}

/** The lines above the code of an engine written in Clarity that the inspector sets. */
export type EngineLine = "uses" | "state";

/**
 * The text of an engine written in Clarity with its `uses` or `state` line
 * set to `line`, or taken away for null. The line goes where the other
 * settings are: after the `engine` line, before what it keeps and its
 * functions.
 */
export function withEngineLine(text: string, kind: EngineLine, line: string | null): string {
  const lines = text.split("\n");
  const own = lines.findIndex((l, i) => i > 0 && l.trim().startsWith(kind + " "));
  if (own >= 0) {
    if (line === null) lines.splice(own, 1);
    else lines[own] = "    " + line;
    return lines.join("\n");
  }
  if (line === null) return text;
  // After the settings already there: uses first, then state.
  let at = 1;
  while (at < lines.length && /^\s+(uses|says|state)\b/.test(lines[at]) && !(kind === "uses" && lines[at].trim().startsWith("state"))) at++;
  lines.splice(at, 0, "    " + line);
  return lines.join("\n");
}

/** The line (from 1) where an engine's Rust source writes the function `name`, as clarity_project::engines finds it. */
export function functionLine(source: string, name: string): number | null {
  const lines = source.split("\n");
  for (let i = 0; i < lines.length; i++) {
    for (const match of lines[i].matchAll(/function!\(\s*([A-Za-z0-9_]+)/g)) {
      if (match[1] === name) return i + 1;
    }
  }
  return null;
}

/** The values of a function as typed in one cell: `square: integer, level: Level`. */
export function paramsOf(text: string): { name: string; type: string }[] {
  return text
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part !== "")
    .map((part) => {
      const colon = part.indexOf(":");
      return colon < 0 ? { name: part, type: "" } : { name: part.slice(0, colon).trim(), type: part.slice(colon + 1).trim() };
    });
}

export function paramsText(params: { name: string; type: string }[]): string {
  return params.map((p) => `${p.name}: ${p.type}`).join(", ");
}

/** `base`, or `base2`, `base3`... whichever `taken` does not have. */
export function freeName(taken: string[], base: string): string {
  if (!taken.includes(base)) return base;
  let n = 2;
  while (taken.includes(base + n)) n++;
  return base + n;
}

/** The engines a canvas can run (DESIGN 5.12): those declared beside the screen. */
export function besideEngines(project: Project): EngineDecl[] {
  return project.files.flatMap((file) => file.engines).filter((engine) => engine.beside);
}

type Running = { runs: string | null; handlers: Handler[]; tells: TellHandler[] };

/**
 * A canvas set to run `engine`, or none. The engine draws it, so its draw
 * handler is unwired (the handler stays in the code), and only tells the
 * engine has stay wired.
 */
export function runEngine<P extends Running>(canvas: P, engine: EngineDecl | null): P {
  if (engine === null) return { ...canvas, runs: null, tells: [] };
  const tells = new Set(engine.tells.map((t) => t.name.name));
  return {
    ...canvas,
    runs: engine.name.name,
    handlers: canvas.handlers.filter((h) => h.event !== "draw"),
    tells: canvas.tells.filter((t) => tells.has(t.tell)),
  };
}

/** A canvas's events: none to draw it while an engine does. */
export function canvasEvents(events: PartEvent[], runs: string | null): PartEvent[] {
  return runs === null ? events : events.filter((e) => e !== "draw");
}
