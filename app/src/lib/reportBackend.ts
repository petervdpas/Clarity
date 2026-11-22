// The report tab's designer reads and writes its report through the backend,
// on the spec's model (clarity_spec::report_design), as Clarity's report
// tools do. The design travels as JSON text, so its words keep their order.

import { invoke } from "@clarity/ui/host/backend.ts";
import type { PartKind, ReportModel, ReportPart } from "./reportDesign";

/** A report's text as the designer holds it; refused with why it cannot be read. */
export async function readReport(text: string): Promise<ReportModel> {
  return JSON.parse(await invoke<string>("report_read", { text })) as ReportModel;
}

/** The designer's report as its file's text, the bands in the order they print. */
export function writeReport(model: ReportModel): Promise<string> {
  return invoke<string>("report_write", { design: JSON.stringify(model) });
}

/** What a new part of `kind` starts with, at a place in its band. */
export async function newPart(kind: PartKind, at: [number, number]): Promise<ReportPart> {
  return JSON.parse(await invoke<string>("report_new_part", { kind, at })) as ReportPart;
}

/** A stylesheet's text with one declaration of a selector set, or taken out when `value` is empty; the rest kept as written. */
export function withDeclaration(css: string, selector: string, property: string, value: string): Promise<string> {
  return invoke<string>("report_with_declaration", { css, selector, property, value });
}

/** The rules of a stylesheet by selector (`.title`, `body`), each with its declarations. */
export async function stylesheetRules(css: string): Promise<Record<string, Record<string, string>>> {
  const rules = await invoke<[string, [string, string][]][]>("report_rules", { css });
  return Object.fromEntries(rules.map(([selector, declarations]) => [selector, Object.fromEntries(declarations)]));
}
