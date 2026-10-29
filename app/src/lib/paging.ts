// The paging form of a web API (DESIGN 5.23, paging): how its lists are
// read a page at a time, as the project file's `paging` line writes it.

import type { Paging } from "./generated/Paging";
import type { PagingKind } from "./generated/PagingKind";

/** The kinds in the order the form offers them, with their labels. */
export const PAGING_KINDS: { id: PagingKind; label: string }[] = [
  { id: "page", label: "Page numbers" },
  { id: "offset", label: "Offset and limit" },
  { id: "cursor", label: "Cursor" },
  { id: "link", label: "Link header" },
];

/** Each kind as it starts, matching the project file's examples. */
const STARTS: Record<PagingKind, Paging> = {
  page: { kind: "page", param: "page", size_param: "per_page", size: 100, next: "", items: "" },
  offset: { kind: "offset", param: "offset", size_param: "limit", size: 100, next: "", items: "" },
  cursor: { kind: "cursor", param: "after", size_param: "", size: 0, next: "meta.next", items: "" },
  link: { kind: "link", param: "", size_param: "", size: 0, next: "", items: "" },
};

/** What each kind's form shows: the label of its parameter, and whether it has a size or a next cursor. */
const FIELDS: Record<PagingKind, { param: string | null; size: boolean; next: boolean }> = {
  page: { param: "Page parameter", size: true, next: false },
  offset: { param: "Offset parameter", size: true, next: false },
  cursor: { param: "Cursor parameter", size: false, next: true },
  link: { param: null, size: false, next: false },
};

export function pagingFields(kind: PagingKind) {
  return FIELDS[kind];
}

/**
 * The paging after a change of kind: the kind's start, keeping where the
 * items are and a size both kinds have. No kind means the API does not page.
 */
export function withKind(current: Paging | null, kind: PagingKind | ""): Paging | null {
  if (kind === "") return null;
  if (current?.kind === kind) return current;
  const start = STARTS[kind];
  const size = current && FIELDS[kind].size && FIELDS[current.kind].size ? current.size : start.size;
  return { ...start, size, items: current?.items ?? "" };
}
