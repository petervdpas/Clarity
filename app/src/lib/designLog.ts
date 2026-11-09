// Trace lines for the window designer, each starting with [design]: in the
// IDE's log file when Logging is on (Clarity settings), nowhere else.
import { invoke } from "@tauri-apps/api/core";

export function designLog(...parts: unknown[]): void {
  const line = parts.map((p) => (typeof p === "string" ? p : JSON.stringify(p))).join(" ");
  invoke("log_line", { line: `[design] ${line}` }).catch(() => {});
}
