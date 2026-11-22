// The range of a slider or a spin box as the designer shows it. Clarity.UI
// moves a value within it (conformance/ranges.json).

import type { Literal } from "../generated/Literal";

/** A range as `clarity_spec::widgets::number_range` gives it: null is no end. */
export type Range = { min: string | null; max: string | null; step: string };

/** The range of a slider or a spin box, as `clarity_spec::widgets::number_range` gives it: as written, else the bound field's own ends; a slider without either runs from 0 to 100. */
export function numberRange(
  p: { kind: string; min: string | null; max: string | null; step: string | null },
  modifiers?: { min: Literal | null; max: Literal | null },
): Range {
  const own = (l: Literal | null | undefined) => (l?.kind === "number" ? l.value : null);
  let min = p.min ?? own(modifiers?.min);
  let max = p.max ?? own(modifiers?.max);
  if (p.kind === "slider") {
    min ??= "0";
    max ??= "100";
  }
  return { min, max, step: p.step ?? "1" };
}
