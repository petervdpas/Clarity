// A foldable panel or form (conformance/folding.json): folded as the person
// or a handler left it, which the part's state carries, or else as designed.

export function isFolded(design: boolean, part: { folded?: boolean | null } | undefined): boolean {
  return part?.folded ?? design;
}
