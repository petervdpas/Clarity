// The Markdown of the markdown part, read into blocks (headings, paragraphs,
// list items, quotes, scene breaks, code) and their spans of text, the way every renderer reads
// it (conformance/markdown.json says what each case gives). Nothing here is
// HTML: the parts render the blocks themselves.

export type Span = { text: string; bold?: true; italic?: true; code?: true; link?: string };

export type Block =
  | { kind: "heading"; level: number; spans: Span[] }
  | { kind: "paragraph"; spans: Span[] }
  | { kind: "item"; depth: number; ordered: boolean; number?: number; spans: Span[] }
  | { kind: "quote"; depth: number; spans: Span[] }
  | { kind: "rule" }
  | { kind: "code"; text: string };

/** The look a span has, as it is built. */
type Look = { bold?: true; italic?: true; link?: string };

const HEADING = /^(#{1,6})\s+(.*)$/;
const ITEM = /^(\s*)([-*+]|(\d{1,9})[.)])\s+(.*)$/;
const PUNCTUATION = "\\`*_[]()#+-.!{}<>";
const FENCE = "```";

/** Addresses a link may open; with any other, only the link's text shows. */
function opens(address: string): boolean {
  return address.startsWith("http://") || address.startsWith("https://") || address.startsWith("mailto:") || address.startsWith("clarity:");
}

const isWordChar = (c: string | undefined) => c !== undefined && /[\p{L}\p{N}]/u.test(c);

/** Where the single `mark` that closes an emphasis from `from` on is, skipping doubled marks. */
function closingSingle(s: string, mark: string, from: number): number {
  for (let j = from; j < s.length; j++) {
    if (s[j] !== mark) continue;
    if (s[j + 1] === mark) {
      j++;
      continue;
    }
    if (mark === "_" && isWordChar(s[j + 1])) continue;
    return j;
  }
  return -1;
}

/** The spans of a block's text, with the look of what holds them. */
function inline(s: string, look: Look, out: Span[]) {
  let text = "";
  const flush = () => {
    if (text) out.push({ text, ...look });
    text = "";
  };
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    const double = s.slice(i, i + 2);
    const triple = s.slice(i, i + 3);
    const tripleEnd = (triple === "***" || triple === "___") && s[i + 3] !== undefined && s[i + 3] !== " " && !(c === "_" && isWordChar(s[i - 1])) ? s.indexOf(triple, i + 3) : -1;
    if (c === "\\" && i + 1 < s.length && PUNCTUATION.includes(s[i + 1])) {
      text += s[i + 1];
      i += 2;
    } else if (tripleEnd >= 0) {
      // Bold and italic at once.
      flush();
      inline(s.slice(i + 3, tripleEnd), { ...look, bold: true, italic: true }, out);
      i = tripleEnd + 3;
    } else if (c === "`") {
      const end = s.indexOf("`", i + 1);
      if (end < 0) {
        text += c;
        i++;
      } else {
        flush();
        out.push({ text: s.slice(i + 1, end), ...look, code: true });
        i = end + 1;
      }
    } else if ((double === "**" || double === "__") && s[i + 2] !== undefined && s[i + 2] !== " " && !(c === "_" && isWordChar(s[i - 1]))) {
      const end = s.indexOf(double, i + 2);
      if (end < 0) {
        text += double;
        i += 2;
      } else {
        flush();
        inline(s.slice(i + 2, end), { ...look, bold: true }, out);
        i = end + 2;
      }
    } else if ((c === "*" || c === "_") && s[i + 1] !== undefined && s[i + 1] !== " " && !(c === "_" && isWordChar(s[i - 1]))) {
      const end = closingSingle(s, c, i + 1);
      if (end < 0) {
        text += c;
        i++;
      } else {
        flush();
        inline(s.slice(i + 1, end), { ...look, italic: true }, out);
        i = end + 1;
      }
    } else if (c === "[") {
      const close = s.indexOf("]", i + 1);
      const end = close >= 0 && s[close + 1] === "(" ? s.indexOf(")", close + 2) : -1;
      if (end < 0) {
        text += c;
        i++;
      } else {
        flush();
        const address = s.slice(close + 2, end).trim();
        inline(s.slice(i + 1, close), opens(address) ? { ...look, link: address } : look, out);
        i = end + 1;
      }
    } else {
      text += c;
      i++;
    }
  }
  flush();
}

const sameLook = (a: Span, b: Span) => a.bold === b.bold && a.italic === b.italic && a.code === b.code && a.link === b.link;

/** The spans of a text: split where the look changes, merged where it does not. */
export function spans(text: string): Span[] {
  const raw: Span[] = [];
  inline(text, {}, raw);
  const merged: Span[] = [];
  for (const span of raw) {
    const last = merged.at(-1);
    if (last && sameLook(last, span)) last.text += span.text;
    else merged.push({ ...span });
  }
  return merged;
}

/** A scene break: three or more of one of `*`, `-` and `_`, alone on the line (spaces between allowed). */
function isRule(trimmed: string): boolean {
  const marks = trimmed.replace(/ /g, "");
  return marks.length >= 3 && "*-_".includes(marks[0]) && [...marks].every((m) => m === marks[0]);
}

/** A quote line: its depth (at most 3) and its text after one space; null for another line. */
function quote(trimmed: string): { depth: number; text: string } | null {
  if (!trimmed.startsWith(">")) return null;
  let depth = 0;
  let rest = trimmed;
  while (rest.startsWith(">")) {
    depth++;
    rest = rest.slice(1);
    if (rest.startsWith(" >")) rest = rest.slice(1);
  }
  return { depth: Math.min(depth, 3), text: (rest.startsWith(" ") ? rest.slice(1) : rest).trimEnd() };
}

/** Whether a line ends in a hard line break: two spaces, or a backslash. */
const breaks = (line: string) => line.endsWith("  ") || line.trimEnd().endsWith("\\");

/** The blocks of a text in Markdown. */
export function markdownBlocks(markdown: string): Block[] {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const blocks: Block[] = [];
  // The paragraph, item or quote that following lines join, with its text so far.
  let open: { block: Block & { spans: Span[] }; text: string } | null = null;
  // Whether the last line ended in a hard line break.
  let broken = false;
  const close = () => {
    if (open) open.block.spans = spans(open.text);
    open = null;
  };
  // Joins `more` to the open block: with a line break after a line that asked for one, else a space.
  const join = (more: string, wasBroken: boolean) => {
    if (!open) return;
    if (wasBroken) {
      if (open.text.endsWith("\\")) open.text = open.text.slice(0, -1);
      open.text += "\n" + more;
    } else {
      open.text += " " + more;
    }
  };
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (trimmed.startsWith(FENCE)) {
      close();
      const code: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith(FENCE)) code.push(lines[i++]);
      blocks.push({ kind: "code", text: code.join("\n") });
      continue;
    }
    const wasBroken = broken;
    broken = breaks(line);
    if (!trimmed) {
      close();
      continue;
    }
    const heading = HEADING.exec(trimmed);
    if (heading) {
      close();
      blocks.push({ kind: "heading", level: heading[1].length, spans: spans(heading[2].trim()) });
      continue;
    }
    if (isRule(trimmed)) {
      close();
      blocks.push({ kind: "rule" });
      continue;
    }
    const quoted = quote(trimmed);
    if (quoted) {
      const current = open as { block: Block } | null;
      const same = current !== null && current.block.kind === "quote" && current.block.depth === quoted.depth;
      if (!quoted.text) {
        // A `>` alone ends the quote's paragraph.
        close();
      } else if (same) {
        join(quoted.text, wasBroken);
      } else {
        close();
        const block: Block & { spans: Span[] } = { kind: "quote", depth: quoted.depth, spans: [] };
        blocks.push(block);
        open = { block, text: quoted.text };
      }
      continue;
    }
    const item = ITEM.exec(line);
    if (item) {
      close();
      const depth = Math.min(Math.floor(item[1].length / 2), 3);
      const block: Block & { spans: Span[] } = item[3] !== undefined
        ? { kind: "item", depth, ordered: true, number: Number(item[3]), spans: [] }
        : { kind: "item", depth, ordered: false, spans: [] };
      blocks.push(block);
      open = { block, text: item[4].trim() };
      continue;
    }
    if (open) {
      join(trimmed, wasBroken);
    } else {
      const block: Block & { spans: Span[] } = { kind: "paragraph", spans: [] };
      blocks.push(block);
      open = { block, text: trimmed };
    }
  }
  close();
  return blocks;
}

