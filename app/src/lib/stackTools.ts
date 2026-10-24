// What the stack fields say about the build tools of the chosen stack (P3):
// ready with the tools found, or what to install. Designing, previewing and
// generating need no tools; only Build and Run do.

import type { Backend } from "./generated/Backend";
import type { StackTools } from "./generated/StackTools";
import type { Ui } from "./generated/Ui";

/** What the row says, and the heading of the manual's setup page that tells how to install what is missing. */
export type ToolsNote = { level: "ok" | "warning"; text: string; guide: string | null };

export function toolsNote(all: StackTools[] | null, backend: Backend, ui: Ui): ToolsNote | null {
  const tools = all?.find((t) => t.backend === backend && t.ui === ui);
  if (!tools) return null;
  if (!tools.generated) return { level: "warning", text: "Clarity does not generate this stack yet.", guide: null };
  if (tools.missing.length === 0) return { level: "ok", text: `Ready to build: ${tools.found.join(", ")}`, guide: null };
  const install = tools.missing.join(", and ");
  return {
    level: "warning",
    text: `Install ${install} to build and run here. Designing, previewing and generating work without it; the generated project's README.md says how to build it.`,
    guide: tools.guide,
  };
}
