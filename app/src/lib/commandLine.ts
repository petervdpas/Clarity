// Run and the command-line program (DESIGN 5.20): which programs a stack
// makes, so Run runs the one there is or asks which, and how the stack is
// named when it has no windows.

export type Program = "desktop" | "cli";

/** The programs a stack's targets make: the windowed one, and the command-line one. */
export function programs(targets: readonly string[]): Program[] {
  const out: Program[] = [];
  if (targets.some((t) => t !== "cli")) out.push("desktop");
  if (targets.includes("cli")) out.push("cli");
  return out;
}

/** The stack as the Project tab names it; a stack without windows has no UI. */
export function stackLabel(backend: string | undefined, ui: string | undefined, targets: string[], ids: readonly string[]): string {
  const parts = programs(ids).includes("desktop") ? [backend, ui] : [backend];
  return `${parts.join(", ")}, ${targets.join(" and ")}`;
}

/** `addAll` as the command line writes it: `add-all`, as the checker names commands. */
function hyphenated(name: string): string {
  return name.replace(/_/g, "-").replace(/(?!^)[A-Z]/g, (c) => "-" + c).toLowerCase();
}

/** A command by what is typed after the program's name: `customer add`. */
export function commandLine(group: string | null, name: string): string {
  return group ? `${hyphenated(group)} ${hyphenated(name)}` : hyphenated(name);
}

/** The project's first command, as Run's command line shows it for a hint; empty without commands. */
export function firstCommand(files: readonly { commands: readonly { group?: { name: string } | null; name: { name: string } }[] }[]): string {
  const first = files.flatMap((f) => f.commands)[0];
  return first ? commandLine(first.group?.name ?? null, first.name.name) : "";
}
