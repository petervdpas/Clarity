// A report's file (`reports/<Name>.report`, docs/ledger/reports-pdf.md) as
// the report tab designs it. The model is the spec's
// (clarity_spec::report_design): the designer reads and writes it through
// the backend (reportBackend.ts). Here is what the designer draws with: the
// bands and parts it offers, the paper, a part's label and look.

import type { ReportBand as SpecBand } from "@clarity/ui/generated/ReportBand.ts";
import type { ReportDesign } from "@clarity/ui/generated/ReportDesign.ts";
import type { ReportPart as SpecPart } from "@clarity/ui/generated/ReportPart.ts";

/** The bands in the order they print; a report has several group headers and footers, one of each per group. */
export const BANDS = ["PageHeader", "ReportHeader", "GroupHeader", "Detail", "GroupFooter", "ReportFooter", "PageFooter"] as const;
/** The bands a report may have more than one of. */
export const REPEATED: readonly BandKind[] = ["GroupHeader", "GroupFooter"];
export type BandKind = (typeof BANDS)[number];
export const PARTS = ["Text", "Field", "Sum", "Markdown", "PageNumber", "Image", "Line", "Box"] as const;
export type PartKind = (typeof PARTS)[number];

/** The paper sizes a report names, in millimetres, upright. */
export const PAPERS: Record<string, [number, number]> = { A3: [297, 420], A4: [210, 297], A5: [148, 210], Letter: [215.9, 279.4], Legal: [215.9, 355.6] };

/** A part on a band, as the spec's model has it, of a kind the designer offers. */
export type ReportPart = Omit<SpecPart, "kind"> & { kind: PartKind };

/** A band, as the spec's model has it, of a kind the designer offers. */
export type ReportBand = Omit<SpecBand, "kind" | "parts"> & { kind: BandKind; parts: ReportPart[] };

/** A report as the spec's model has it: its words (name, paper, margin, stylesheet), its data, its bands. */
export type ReportModel = Omit<ReportDesign, "bands"> & { bands: ReportBand[] };

/** Millimetres of a length as a report writes it: `20mm`, `2cm`, `1in`, `12pt`, or a plain number. */
export function mm(text: string | undefined, fallback: number): number {
  const m = (text ?? "").trim().match(/^(-?[\d.]+)\s*(mm|cm|in|pt)?$/);
  if (!m) return fallback;
  const n = Number(m[1]);
  return m[2] === "cm" ? n * 10 : m[2] === "in" ? n * 25.4 : m[2] === "pt" ? (n * 25.4) / 72 : n;
}

/** The paper's width and height in millimetres, turned when it says landscape. */
export function paperSize(model: ReportModel): [number, number] {
  const words = (model.attrs.paper ?? "A4").split(/\s+/);
  const landscape = words.includes("landscape");
  const name = words.find((w) => w !== "landscape" && w !== "portrait") ?? "A4";
  const [w, h] = PAPERS[name] ?? PAPERS.A4;
  return landscape ? [h, w] : [w, h];
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
