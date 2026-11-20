// A report's file (`reports/<Name>.report`, docs/ledger/reports-pdf.md) as the
// report tab designs it: read into its paper, data, bands and parts, and
// written back as the same XML, in the order bands print in. The words are
// the agreed ones: Report, Data, the five bands, the five parts.

/** The bands in the order they print; a report has several group headers and footers, one of each per group. */
export const BANDS = ["PageHeader", "ReportHeader", "GroupHeader", "Detail", "GroupFooter", "ReportFooter", "PageFooter"] as const;
/** The bands a report may have more than one of. */
export const REPEATED: readonly BandKind[] = ["GroupHeader", "GroupFooter"];
export type BandKind = (typeof BANDS)[number];
export const PARTS = ["Text", "Field", "Sum", "Markdown", "PageNumber", "Image", "Line", "Box"] as const;
export type PartKind = (typeof PARTS)[number];

/** The paper sizes a report names, in millimetres, upright. */
export const PAPERS: Record<string, [number, number]> = { A3: [297, 420], A4: [210, 297], A5: [148, 210], Letter: [215.9, 279.4], Legal: [215.9, 355.6] };

export type ReportPart = {
  kind: PartKind;
  at: [number, number];
  size: [number, number];
  /** Its own words: key, bind, of, align, class, in the order written. */
  attrs: Record<string, string>;
};

export type ReportBand = { kind: BandKind; height: number; attrs: Record<string, string>; parts: ReportPart[] };

export type ReportModel = {
  /** name, paper, margin, stylesheet and any other words of Report. */
  attrs: Record<string, string>;
  data: { name: string; type: string }[];
  bands: ReportBand[];
};

type Element = { name: string; attrs: [string, string][]; children: Element[] };

function unescape(text: string): string {
  return text.replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
}

function escape(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** The root element of a small XML text: elements and attributes, comments and the declaration passed over. */
function readXml(text: string): Element {
  let at = 0;
  const stack: Element[] = [{ name: "", attrs: [], children: [] }];
  while (at < text.length) {
    const open = text.indexOf("<", at);
    if (open < 0) break;
    if (text.startsWith("<!--", open)) {
      const end = text.indexOf("-->", open);
      if (end < 0) throw new Error("a comment is not closed");
      at = end + 3;
      continue;
    }
    if (text.startsWith("<?", open)) {
      at = text.indexOf("?>", open) + 2;
      continue;
    }
    const close = text.indexOf(">", open);
    if (close < 0) throw new Error("an element is not closed");
    const inner = text.slice(open + 1, close);
    at = close + 1;
    if (inner.startsWith("/")) {
      const name = inner.slice(1).trim();
      const done = stack.pop();
      if (!done || done.name !== name) throw new Error(`<${done?.name ?? ""}> is closed by </${name}>`);
      continue;
    }
    const selfClosing = inner.endsWith("/");
    const body = selfClosing ? inner.slice(0, -1) : inner;
    const name = body.match(/^[A-Za-z][\w-]*/)?.[0];
    if (!name) throw new Error(`<${body}> names no element`);
    const attrs: [string, string][] = [];
    for (const m of body.slice(name.length).matchAll(/([\w-]+)\s*=\s*"([^"]*)"/g)) attrs.push([m[1], unescape(m[2])]);
    const element: Element = { name, attrs, children: [] };
    stack[stack.length - 1].children.push(element);
    if (!selfClosing) stack.push(element);
  }
  if (stack.length !== 1) throw new Error(`<${stack[stack.length - 1].name}> is not closed`);
  const root = stack[0].children[0];
  if (!root) throw new Error("the report is empty");
  return root;
}

function pair(text: string | undefined, fallback: [number, number]): [number, number] {
  const parts = (text ?? "").split(",").map((n) => Number(n.trim()));
  return parts.length === 2 && parts.every((n) => Number.isFinite(n)) ? [parts[0], parts[1]] : fallback;
}

/** Millimetres of a length as a report writes it: `20mm`, `2cm`, `1in`, `12pt`, or a plain number. */
export function mm(text: string | undefined, fallback: number): number {
  const m = (text ?? "").trim().match(/^(-?[\d.]+)\s*(mm|cm|in|pt)?$/);
  if (!m) return fallback;
  const n = Number(m[1]);
  return m[2] === "cm" ? n * 10 : m[2] === "in" ? n * 25.4 : m[2] === "pt" ? (n * 25.4) / 72 : n;
}

/** A report's text as the designer holds it. */
export function readReport(text: string): ReportModel {
  const root = readXml(text);
  if (root.name !== "Report") throw new Error(`a report starts with <Report>, not <${root.name}>`);
  const model: ReportModel = { attrs: Object.fromEntries(root.attrs), data: [], bands: [] };
  for (const child of root.children) {
    const attrs = Object.fromEntries(child.attrs);
    if (child.name === "Data") {
      model.data.push({ name: attrs.name ?? "", type: attrs.type ?? "" });
      continue;
    }
    if (!(BANDS as readonly string[]).includes(child.name)) throw new Error(`a report has no '${child.name}'`);
    const { height, ...rest } = attrs;
    const parts: ReportPart[] = child.children.map((p) => {
      const { at, size, ...own } = Object.fromEntries(p.attrs);
      if (!(PARTS as readonly string[]).includes(p.name)) throw new Error(`a band has no '${p.name}'`);
      return { kind: p.name as PartKind, at: pair(at, [0, 0]), size: pair(size, [40, 6]), attrs: own };
    });
    model.bands.push({ kind: child.name as BandKind, height: mm(height, 10), attrs: rest, parts });
  }
  return model;
}

function written(n: number): string {
  return String(Math.round(n * 10) / 10);
}

function attributes(attrs: Record<string, string>): string {
  return Object.entries(attrs)
    .filter(([, v]) => v !== undefined && v !== null && v !== "")
    .map(([k, v]) => ` ${k}="${escape(v)}"`)
    .join("");
}

/** The designer's report as its file's text, the bands in the order they print. */
export function writeReport(model: ReportModel): string {
  const lines = [`<Report${attributes(model.attrs)}>`];
  for (const d of model.data) lines.push(`  <Data name="${escape(d.name)}" type="${escape(d.type)}" />`);
  const ordered = [...model.bands].sort((a, b) => BANDS.indexOf(a.kind) - BANDS.indexOf(b.kind));
  for (const band of ordered) {
    const head = `  <${band.kind} height="${written(band.height)}mm"${attributes(band.attrs)}`;
    if (band.parts.length === 0) {
      lines.push(`${head} />`);
      continue;
    }
    lines.push(`${head}>`);
    for (const p of band.parts) lines.push(`    <${p.kind} at="${written(p.at[0])},${written(p.at[1])}" size="${written(p.size[0])},${written(p.size[1])}"${attributes(p.attrs)} />`);
    lines.push(`  </${band.kind}>`);
  }
  lines.push("</Report>", "");
  return lines.join("\n");
}

/** The paper's width and height in millimetres, turned when it says landscape. */
export function paperSize(model: ReportModel): [number, number] {
  const words = (model.attrs.paper ?? "A4").split(/\s+/);
  const landscape = words.includes("landscape");
  const name = words.find((w) => w !== "landscape" && w !== "portrait") ?? "A4";
  const [w, h] = PAPERS[name] ?? PAPERS.A4;
  return landscape ? [h, w] : [w, h];
}

/** What a new part of `kind` starts with, at a place in its band. */
export function newPart(kind: PartKind, at: [number, number]): ReportPart {
  const attrs: Record<string, string> = kind === "Text" ? { key: "" } : kind === "Field" || kind === "Markdown" ? { bind: "" } : kind === "Sum" ? { of: "", align: "right" } : kind === "Image" ? { source: "" } : kind === "Line" || kind === "Box" ? {} : { align: "center" };
  const size: [number, number] = kind === "Markdown" ? [120, 20] : kind === "PageNumber" ? [10, 5] : kind === "Image" ? [40, 20] : kind === "Line" ? [60, 0] : kind === "Box" ? [60, 20] : [40, 6];
  return { kind, at, size, attrs };
}

/** What a part shows in the designer: its key, bind or sum, or its kind. */
export function partLabel(part: ReportPart): string {
  if (part.kind === "PageNumber") return "#";
  if (part.kind === "Line" || part.kind === "Box") return "";
  if (part.kind === "Sum") return `Σ ${part.attrs.of || "?"}`;
  if (part.kind === "Image") return part.attrs.source || (part.attrs.bind ? `[${part.attrs.bind}]` : "Image");
  return part.attrs.key || part.attrs.bind || part.kind;
}

/** A value kept to whole tenths of a millimetre and never below `least`. */
export function snapped(n: number, least = 0): number {
  return Math.max(least, Math.round(n * 10) / 10);
}

/** How a part looks as it prints, from Clarity's own stylesheet, its `body` and its class. */
export type PartLook = { sizePt: number; bold: boolean; italic: boolean; color: string; background: string | null; align: string | null; font: string };

/** The rules of a stylesheet's text by selector: `.title`, `body`. */
export function stylesheetRules(css: string): Record<string, Record<string, string>> {
  const rules: Record<string, Record<string, string>> = {};
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const m of text.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const decls: Record<string, string> = {};
    for (const d of m[2].split(";")) {
      const [k, ...v] = d.split(":");
      if (k && v.length) decls[k.trim()] = v.join(":").trim();
    }
    for (const selector of m[1].split(",")) {
      const name = selector.trim();
      if (name) rules[name] = { ...(rules[name] ?? {}), ...decls };
    }
  }
  return rules;
}

function points(value: string | undefined, body: number): number | null {
  const m = (value ?? "").match(/^([\d.]+)\s*(pt|px|em|%)?$/);
  if (!m) return null;
  const n = Number(m[1]);
  return m[2] === "px" ? n * 0.75 : m[2] === "em" ? n * body : m[2] === "%" ? (n / 100) * body : n;
}

/** A part's look: Clarity's body (10.5pt, its text colour), the stylesheet's `body`, then the part's class. */
export function partLook(rules: Record<string, Record<string, string>>, cls: string | undefined): PartLook {
  const look: PartLook = { sizePt: 10.5, bold: false, italic: false, color: "#1d2330", background: null, align: null, font: "Inter, sans-serif" };
  for (const rule of [rules.body, cls ? rules[`.${cls}`] : undefined]) {
    if (!rule) continue;
    look.sizePt = points(rule["font-size"], look.sizePt) ?? look.sizePt;
    if (rule["font-weight"]) look.bold = rule["font-weight"] === "bold" || Number(rule["font-weight"]) >= 600;
    if (rule["font-style"]) look.italic = rule["font-style"] !== "normal";
    if (rule.color && !rule.color.startsWith("var(")) look.color = rule.color;
    const bg = rule["background-color"] ?? rule.background;
    if (bg && !bg.startsWith("var(")) look.background = bg;
    if (rule["text-align"]) look.align = rule["text-align"];
    if (rule["font-family"]) look.font = rule["font-family"].includes("serif") && !rule["font-family"].includes("sans") ? "serif" : rule["font-family"].includes("mono") ? "monospace" : look.font;
  }
  return look;
}

/** A stylesheet's text with one declaration of a selector set (`.title { font-size: 18pt }`), or taken out when `value` is empty; the rest kept as written. */
export function withDeclaration(css: string, selector: string, property: string, value: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const block = new RegExp(`(^|\\n)(\\s*${escaped}\\s*\\{)([^}]*)(\\})`);
  const m = css.match(block);
  if (!m) {
    if (!value) return css;
    const sep = css.length === 0 || css.endsWith("\n") ? "" : "\n";
    return `${css}${sep}${selector} { ${property}: ${value}; }\n`;
  }
  const decls = m[3]
    .split(";")
    .map((d) => d.trim())
    .filter((d) => d && d.split(":")[0].trim() !== property);
  if (value) decls.push(`${property}: ${value}`);
  const body = decls.length ? ` ${decls.join("; ")}; ` : " ";
  return css.replace(block, `$1$2${body}$4`);
}
