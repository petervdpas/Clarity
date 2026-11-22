// Code in colors, for the code editors. Clarity's pieces come from the
// backend (clarity_spec::highlight, the lexer's reading); the Rust of an
// engine has a small tokenizer here. The editor paints each line by its
// text, so a line typed since the last answer shows plain until the next.

import type { Piece } from "@clarity/ui/generated/Piece.ts";
import type { PieceKind } from "@clarity/ui/generated/PieceKind.ts";

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

