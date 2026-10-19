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
