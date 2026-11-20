// DevTools' page logic (DevToolsSettings.svelte, ReleasePlanning.svelte).

type JobLike = { state: string } | undefined;
type CheckLike = { job: string; title: string; last: { passed: boolean; current?: boolean } | null };

/** How a check stands: its build in this run of Clarity, else how it last ended. */
export function checkState(check: CheckLike, job: JobLike): "waiting" | "running" | "passed" | "failed" | "notRun" {
  if (job?.state === "waiting" || job?.state === "running" || job?.state === "passed") return job.state;
  if (job) return "failed";
  // A result of another tree (the commit or an uncommitted file changed since) counts as not run.
  if (check.last && check.last.current !== false) return check.last.passed ? "passed" : "failed";
  return "notRun";
}

/** The checks as one outcome: what runs now, what failed, what has not run, and that in a line. */
export function verdict(checks: CheckLike[], jobs: Record<string, JobLike>) {
  const states = checks.map((c) => ({ title: c.title, job: c.job, state: checkState(c, jobs[c.job]) }));
  const running = states.find((s) => s.state === "running") ?? null;
  const waiting = states.filter((s) => s.state === "waiting").length;
  const passed = states.filter((s) => s.state === "passed").length;
  const failed = states.filter((s) => s.state === "failed");
  const notRun = states.filter((s) => s.state === "notRun");
  const total = checks.length;
  let text: string;
  if (running) text = `Running ${passed + failed.length + 1} of ${total}: ${running.title}`;
  else if (waiting) text = `${waiting} waiting to run`;
  else if (passed === total) text = `All ${total} checks passed`;
  else if (notRun.length === total) text = `Not run yet: ${total} checks`;
  else if (!notRun.length) text = `${failed.length} of ${total} failed`;
  else text = `${passed} passed, ${failed.length} failed, ${notRun.length} not run`;
  const held = total - passed;
  // Whether the agent may write a plan (MCP write_release_plan refuses while one is held back).
  const planText = held === 0 && total > 0 ? "A plan can be written" : `Held back: ${held} ${held === 1 ? "check" : "checks"} not passed on this tree`;
  return { running, failed, notRun, passed, total, allPassed: total > 0 && passed === total, text, planText };
}

/** What a release's changes since its tag come to, for review: code first, the rest only named. */
export function changeText(row: { code: string[]; manifest: string[]; tests: string[]; docs: string[] }): string {
  if (row.code.length) return row.code.length === 1 ? "1 code file" : `${row.code.length} code files`;
  const kinds = [row.manifest.length && "manifest", row.tests.length && "tests", row.docs.length && "docs"].filter(Boolean) as string[];
  if (!kinds.length) return "no changes";
  return kinds.length === 1 ? `${kinds[0]} only` : `${kinds.slice(0, -1).join(", ")} and ${kinds[kinds.length - 1]}, no code`;
}

/** How a build stands, in words. */
export function stateText(job: { state: string } | undefined): string {
  switch (job?.state) {
    case undefined:
      return "not run";
    case "waiting":
      return "waiting";
    case "running":
      return "building";
    case "passed":
      return "passed";
    default:
      return "failed";
  }
}

/** A size as people read it: `39.2 GB`, `448 MB`, `12 KB`. */
export function bytesText(bytes: number): string {
  const units = ["bytes", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${bytes} bytes` : `${value >= 10 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`;
}

/** The Disk and Release planning tabs' keys: no piece has them. */
export const DISK_TAB = "disk";
export const RELEASE_TAB = "releasePlanning";

/** The tab an agent's ask works in, which Clarity settings open at: a build in its piece's tab, a removal in Disk. */
export function askedTab(ask: { action: string; key: string }): string {
  return ask.action === "remove" ? DISK_TAB : ask.key;
}

/** The tabs in their order: Disk and Release planning first, then the pieces marked with a signal, then the others, each in the order given. */
export function tabOrder(pieces: { key: string; signal: string | null }[]): string[] {
  return [DISK_TAB, RELEASE_TAB, ...pieces.filter((p) => p.signal).map((p) => p.key), ...pieces.filter((p) => !p.signal).map((p) => p.key)];
}

/** The tab `step` places from `current` in `order`, round from the last to the first and back. */
export function cycle(order: string[], current: string, step: number): string {
  if (!order.length) return current;
  const at = Math.max(0, order.indexOf(current));
  return order[(((at + step) % order.length) + order.length) % order.length];
}

/** The piece of Clarity.UI's browser files, which the design and preview draw with. */
export const BROWSER_PIECE = "clarityUiBrowser";

/** Whether every canvas loads the browser files again: they were swapped, or a build of them just passed. */
export function reloadsBrowserFiles(change: { swapped: string } | { job: { key: string; state: string; ended: number | null }; before?: { ended: number | null } }): boolean {
  if ("swapped" in change) return change.swapped === BROWSER_PIECE;
  return change.job.key === `piece:${BROWSER_PIECE}` && change.job.state === "passed" && change.before?.ended !== change.job.ended;
}
