// The engines a project declares (DESIGN 5.11), for the project tree and the
// module view: each with its module and the errors found in its declaration,
// such as a file that is missing or lacks a declared function.

import type { EngineDecl } from "./generated/EngineDecl";
import type { EngineFunction } from "./generated/EngineFunction";
import type { Project } from "./generated/Project";
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

/** A function of an engine as it is declared: `total(lines: list<Line>) -> money`. */
export function signature(f: EngineFunction): string {
  const params = f.params.map((p) => `${p.name.name}: ${typeText(p.ty.kind)}`).join(", ");
  return `${f.name.name}(${params})` + (f.returns ? ` -> ${typeText(f.returns.kind)}` : "");
}

/** The services an engine may use, in the order Clarity lists them (clarity_spec::engines::SERVICES). */
export const SERVICES = ["clock", "random", "log"] as const;

/** An engine's declaration as its inspector edits it. */
export type EngineForm = {
  name: string;
  file: string;
  uses: string[];
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
