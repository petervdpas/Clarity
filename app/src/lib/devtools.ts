// DevTools' page logic (DevToolsSettings.svelte, ReleasePlanning.svelte).

/** The plan's file name in releases/, after the local time: `2026-11-05-0907`. */
export function planStamp(at: Date): string {
  const two = (n: number) => String(n).padStart(2, "0");
  return `${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())}-${two(at.getHours())}${two(at.getMinutes())}`;
}

/** A release with files changed since its tag starts in the plan. */
export function includedAtFirst(row: { changed: string[] }): boolean {
  return row.changed.length > 0;
}

/** The titles of the checks whose build has not passed. */
export function openChecks(checks: { job: string; title: string }[], jobs: Record<string, { state: string } | undefined>): string[] {
  return checks.filter((c) => jobs[c.job]?.state !== "passed").map((c) => c.title);
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
