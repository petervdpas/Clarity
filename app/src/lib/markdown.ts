// The little Markdown the agent writes, as HTML: paragraphs, headings,
// lists, code blocks, bold, italics and inline code. Everything is escaped
// first, so the text never brings HTML of its own.

const escape = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** Bold, italics and code within a line. */
function inline(text: string): string {
  return escape(text)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*])\*([^*\s][^*]*?)\*/g, "$1<em>$2</em>");
}

const BULLET = /^\s*[-*]\s+(.*)$/;
const NUMBERED = /^\s*\d+[.)]\s+(.*)$/;
const HEADING = /^#{1,6}\s+(.*)$/;

export function markdown(text: string): string {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const out: string[] = [];
  let paragraph: string[] = [];
  let list: { tag: "ul" | "ol"; items: string[] } | null = null;

  const flush = () => {
    if (paragraph.length > 0) out.push(`<p>${paragraph.map(inline).join("<br>")}</p>`);
    paragraph = [];
    if (list) out.push(`<${list.tag}>${list.items.map((i) => `<li>${inline(i)}</li>`).join("")}</${list.tag}>`);
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
    const heading = HEADING.exec(line);
    const bullet = BULLET.exec(line);
    const numbered = bullet ? null : NUMBERED.exec(line);
    if (line.trim() === "") {
      flush();
    } else if (heading) {
      flush();
      out.push(`<h4>${inline(heading[1])}</h4>`);
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
