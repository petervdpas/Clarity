// The little Markdown the agent and the manual write, as HTML: paragraphs,
// headings, lists, tables, code blocks, bold, italics, inline code and
// links. Everything is escaped first, so the text never brings HTML of its
// own. An answer of the agent shows a link's text only; a page of the
// manual (`page`) keeps links to other pages and headings for the window to
// follow, and gives its headings anchors.

/** How a text is shown: as a page of the manual, or as an answer. */
export type MarkdownOptions = { page?: boolean };

const escape = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** A link a page may follow: another page of the manual, or a heading. */
const FOLLOWED = /^(\.\.\/)?[A-Za-z0-9._-]+\.md(#[A-Za-z0-9-]+)?$|^#[A-Za-z0-9-]+$/;

/** A heading's anchor, as Markdown links write it: lower case, without
 *  marks, words joined by hyphens. */
export const anchor = (heading: string) =>
  heading
    .toLowerCase()
    .replace(/[^a-z0-9 -]/g, "")
    .trim()
    .replace(/\s+/g, "-");

/** Bold, italics, code and links within a line. */
function inline(text: string, page = false): string {
  return escape(text)
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, label: string, href: string) =>
      page && FOLLOWED.test(href.replace(/&amp;/g, "&")) ? `<a href="#" data-link="${href}">${label}</a>` : label,
    )
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*])\*([^*\s][^*]*?)\*/g, "$1<em>$2</em>");
}

const BULLET = /^\s*[-*]\s+(.*)$/;
const NUMBERED = /^\s*\d+[.)]\s+(.*)$/;
const HEADING = /^#{1,6}\s+(.*)$/;

/** A table row's cells. */
const cells = (line: string) =>
  line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());

export function markdown(text: string, options: MarkdownOptions = {}): string {
  const page = options.page ?? false;
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const out: string[] = [];
  let paragraph: string[] = [];
  let list: { tag: "ul" | "ol"; items: string[] } | null = null;

  const flush = () => {
    // An answer keeps its line breaks; a page's lines flow into one paragraph.
    if (paragraph.length > 0) out.push(`<p>${paragraph.map((l) => inline(l, page)).join(page ? " " : "<br>")}</p>`);
    paragraph = [];
    if (list) out.push(`<${list.tag}>${list.items.map((i) => `<li>${inline(i, page)}</li>`).join("")}</${list.tag}>`);
    list = null;
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim().startsWith("```")) {
      flush();
      const code: string[] = [];
      while (++i < lines.length && !lines[i].trim().startsWith("```")) code.push(lines[i]);
      out.push(`<pre><code>${escape(code.join("\n"))}</code></pre>`);
      continue;
    }
    // A table: a header row, a row of dashes, then its rows.
    if (line.trim().startsWith("|") && /^\s*\|?[\s:|-]+\|?\s*$/.test(lines[i + 1] ?? "") && (lines[i + 1] ?? "").includes("-")) {
      flush();
      const head = cells(line).map((c) => `<th>${inline(c, page)}</th>`).join("");
      const rows: string[] = [];
      i++;
      while (i + 1 < lines.length && lines[i + 1].trim().startsWith("|")) {
        rows.push(`<tr>${cells(lines[++i]).map((c) => `<td>${inline(c, page)}</td>`).join("")}</tr>`);
      }
      out.push(`<table><thead><tr>${head}</tr></thead><tbody>${rows.join("")}</tbody></table>`);
      continue;
    }
    const heading = HEADING.exec(line);
    const bullet = BULLET.exec(line);
    const numbered = bullet ? null : NUMBERED.exec(line);
    if (line.trim() === "") {
      flush();
    } else if (heading) {
      flush();
      if (page) {
        const level = Math.min(line.indexOf(" "), 3);
        out.push(`<h${level} id="${anchor(heading[1])}">${inline(heading[1], page)}</h${level}>`);
      } else {
        out.push(`<h4>${inline(heading[1])}</h4>`);
      }
    } else if (bullet || numbered) {
      const tag = bullet ? "ul" : "ol";
      if (paragraph.length > 0 || (list && list.tag !== tag)) flush();
      list ??= { tag, items: [] };
      list.items.push((bullet ?? numbered)![1]);
    } else if (list && /^\s+/.test(line)) {
      // An indented line goes on with the item above it.
      list.items[list.items.length - 1] += ` ${line.trim()}`;
    } else {
      if (list) flush();
      paragraph.push(line);
    }
  }
  flush();
  return out.join("");
}
