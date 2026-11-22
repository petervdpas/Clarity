import { test } from "node:test";
import assert from "node:assert/strict";
import { segmentsOf } from "./highlight.ts";
import type { Piece } from "@clarity/ui/generated/Piece.ts";

test("a line is cut into its pieces, the rest of it plain", () => {
  const pieces: Piece[] = [
    { line: 1, from: 4, to: 8, kind: "keyword" },
    { line: 1, from: 9, to: 14, kind: "name" },
  ];
  assert.deepEqual(segmentsOf("    keep moves = 0", pieces), [
    { text: "    ", kind: null },
    { text: "keep", kind: "keyword" },
    { text: " ", kind: null },
    { text: "moves", kind: "name" },
    { text: " = 0", kind: null },
  ]);
});
