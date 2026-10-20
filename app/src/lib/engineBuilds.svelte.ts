// What the last build of each engine said: whether it built, and the
// compiler's messages at their lines. The engine's editor marks them in its
// code, and the Problems list shows them with the project's own.

import type { Diagnostic } from "./generated/Diagnostic";
import type { EngineBuilt } from "./generated/EngineBuilt";
import type { EngineProblem } from "./generated/EngineProblem";

export const engineBuilds = $state<{ byEngine: Record<string, EngineBuilt>; revision: number }>({ byEngine: {}, revision: 0 });

/** Keeps what an engine's build said, replacing what the build before said. */
export function recordBuilt(built: EngineBuilt) {
  engineBuilds.byEngine[built.name] = built;
  engineBuilds.revision++;
}

/** Every engine's compiler messages, errors first. */
export function engineProblems(): { engine: string; problem: EngineProblem }[] {
  return Object.values(engineBuilds.byEngine)
    .flatMap((b) => b.problems.map((problem) => ({ engine: b.name, problem })))
    .sort((a, b) => (a.problem.severity === b.problem.severity ? 0 : a.problem.severity === "error" ? -1 : 1));
}

/** A compiler message as the code editor marks problems. */
export function asDiagnostic(p: EngineProblem): Diagnostic {
  return { severity: p.severity, message: p.message, path: p.file, span: { start: 0, end: 0, line: p.line, col: p.column } };
}
