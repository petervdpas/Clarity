// The Libraries section of Clarity settings, as words: where a build takes
// each library or package set from now (LibraryRow.from), one sentence per
// source, written out whole.

import type { LibraryRow } from "@clarity/ui/generated/LibraryRow.ts";
import type { UsedFrom } from "@clarity/ui/generated/UsedFrom.ts";

/** The sentence of each source; `{version}` and `{variable}` are filled from the row. */
export const USED_FROM: Record<UsedFrom, string> = {
  chosen: "The folder chosen here.",
  environment: "The folder {variable} names where Clarity was started; it counts until a folder is chosen here.",
  carried: "Clarity's own for {version}, carried by Clarity.",
  repository: "This repository's build, as a Clarity run from its source uses.",
  fetched: "Clarity's own for {version}, fetched before.",
  toFetch: "Clarity's own for {version}, fetched on the first build that needs it.",
  unavailable: "Clarity's own for {version} is not made for this system; choose a folder.",
  published: "Clarity's own for {version}, which the stack's own tool fetches on the first build.",
  inProgram: "Copied into each program's source, from this Clarity.",
};

/** Where builds take `row` from now, as people read it. */
export function usedText(row: LibraryRow): string {
  return USED_FROM[row.from].replace("{version}", row.version ?? "").replace("{variable}", row.variable ?? "");
}

/** Whether any folder is chosen, which a factory reset forgets. */
export function anyChosen(rows: LibraryRow[]): boolean {
  return rows.some((r) => r.chosen !== null);
}
