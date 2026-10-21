import { test } from "node:test";
import assert from "node:assert/strict";
import { linesOf, rustPieces, segmentsOf } from "./highlight.ts";
import type { Piece } from "./generated/Piece";

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

test("the colored lines are found by their text, so typing elsewhere keeps them", () => {
  const text = "engine Board\n    keep moves: integer = 0";
  const pieces: Piece[] = [
    { line: 1, from: 0, to: 6, kind: "keyword" },
    { line: 2, from: 4, to: 8, kind: "keyword" },
  ];
  const lines = linesOf(text, pieces);
  // A line typed in between: the others keep their colors, the new one is plain.
  const typed = "engine Board\n    state per app\n    keep moves: integer = 0".split("\n").map((l) => lines.get(l) ?? [{ text: l, kind: null }]);
  assert.equal(typed[0][0].kind, "keyword");
  assert.equal(typed[1][0].kind, null);
  assert.equal(typed[2][1].kind, "keyword");
});

test("rust code of an engine has its keywords, texts, numbers, macros and comments", () => {
  const source = 'clarity_engine::function!(play, |(square,): (i64,)| {\n    // A cross.\n    let at = "x";\n    Ok(42)\n});';
  const kinds = (word: string) => {
    const lines = source.split("\n");
    return rustPieces(source)
      .filter((p) => lines[p.line - 1].slice(p.from, p.to) === word)
      .map((p) => p.kind);
  };
  assert.deepEqual(kinds("function!"), ["name"]);
  assert.deepEqual(kinds("let"), ["keyword"]);
  assert.deepEqual(kinds('"x"'), ["text"]);
  assert.deepEqual(kinds("42"), ["number"]);
  assert.deepEqual(kinds("// A cross."), ["comment"]);
  assert.deepEqual(kinds("i64"), ["type"]);
  assert.deepEqual(kinds("Ok"), ["type"]);
});
