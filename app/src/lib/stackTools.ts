// What the stack fields say about the build tools of the chosen stack (P3):
// ready with the tools found, or what to install. Designing, previewing and
// generating need no tools; only Build and Run do.

import type { Backend } from "./generated/Backend";
import type { StackTools } from "./generated/StackTools";
import type { Target } from "./generated/Target";
import type { Ui } from "./generated/Ui";

/**
 * What the row says; the heading of the manual's setup page that tells how
 * to install what is missing; and whether Clarity installs it itself, with
 * the system asking the person's permission first (`elevated`).
 */
export type ToolsNote = { level: "ok" | "warning"; text: string; guide: string | null; install: { elevated: boolean } | null };

export function toolsNote(all: StackTools[] | null, backend: Backend, ui: Ui, targets: Target[]): ToolsNote | null {
  const tools = all?.find((t) => t.backend === backend && t.ui === ui);
  if (!tools) return null;
  if (!tools.generated) return { level: "warning", text: "Clarity does not generate this stack yet.", guide: null, install: null };
  // A program served to a browser may need more (DESIGN 7.6).
  const web = targets.includes("web");
  const missing = [...tools.missing, ...(web ? tools.web_missing : [])];
  const found = [...tools.found, ...(web ? tools.web_found : [])];
  if (missing.length === 0) return { level: "ok", text: `Ready to build: ${found.join(", ")}`, guide: null, install: null };
  const install = missing.join(", and ");
  // Clarity installs what the web target needs when that is all that is missing.
  const installs = web && tools.missing.length === 0 && tools.web_installable;
  return {
    level: "warning",
    text: `Install ${install} to build and run here. Designing, previewing and generating work without it; the generated project's README.md says how to build it.`,
    guide: tools.guide,
    install: installs ? { elevated: tools.web_elevated } : null,
  };
}
