// Code in colors, for the code editors. Clarity's pieces come from the
// backend (clarity_spec::highlight, the lexer's reading); the Rust of an
// engine has a small tokenizer here. The editor paints each line by its
// text, so a line typed since the last answer shows plain until the next.

import type { Piece } from "./generated/Piece";
import type { PieceKind } from "./generated/PieceKind";

/** A run of a line, colored by its kind, or plain. */
export type Segment = { text: string; kind: PieceKind | null };

/** `line` cut into runs by the pieces that belong to it. */
export function segmentsOf(line: string, pieces: Piece[]): Segment[] {
  const out: Segment[] = [];
  let at = 0;
  for (const p of [...pieces].sort((a, b) => a.from - b.from)) {
    if (p.from < at || p.to > line.length) continue;
    if (p.from > at) out.push({ text: line.slice(at, p.from), kind: null });
    out.push({ text: line.slice(p.from, p.to), kind: p.kind });
    at = p.to;
  }
  if (at < line.length || out.length === 0) out.push({ text: line.slice(at), kind: null });
  return out;
}

/** The runs of each line of `text`, by the line's own text. */
export function linesOf(text: string, pieces: Piece[]): Map<string, Segment[]> {
  const byLine = new Map<number, Piece[]>();
  for (const p of pieces) byLine.set(p.line, [...(byLine.get(p.line) ?? []), p]);
  const out = new Map<string, Segment[]>();
  text.split("\n").forEach((line, i) => out.set(line, segmentsOf(line, byLine.get(i + 1) ?? [])));
  return out;
}

const RUST_KEYWORDS = new Set(
  "as break const continue crate else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while async await dyn".split(" "),
);
const RUST_TYPES = new Set("i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char str String Vec Option Result".split(" "));

/** The pieces of Rust source: keywords, types, texts, numbers, macros and comments. */
export function rustPieces(source: string): Piece[] {
  const pieces: Piece[] = [];
  source.split("\n").forEach((text, index) => {
    const line = index + 1;
    let at = 0;
    while (at < text.length) {
      const rest = text.slice(at);
      const comment = rest.startsWith("//");
      if (comment) {
        pieces.push({ line, from: at, to: text.length, kind: "comment" });
        break;
      }
      if (rest[0] === '"') {
        let end = at + 1;
        while (end < text.length && text[end] !== '"') end += text[end] === "\\" ? 2 : 1;
        pieces.push({ line, from: at, to: Math.min(end + 1, text.length), kind: "text" });
        at = end + 1;
        continue;
      }
      const number = /^\d[\d_]*(\.\d+)?/.exec(rest);
      if (number) {
        pieces.push({ line, from: at, to: at + number[0].length, kind: "number" });
        at += number[0].length;
        continue;
      }
      const word = /^[A-Za-z_][A-Za-z0-9_]*!?/.exec(rest);
      if (word) {
        const w = word[0];
        const kind: PieceKind | null = w.endsWith("!")
          ? "name"
          : RUST_KEYWORDS.has(w)
            ? "keyword"
            : RUST_TYPES.has(w) || /^[A-Z]/.test(w)
              ? "type"
              : null;
        if (kind) pieces.push({ line, from: at, to: at + w.length, kind });
        at += w.length;
        continue;
      }
      at++;
    }
  });
  return pieces;
}
