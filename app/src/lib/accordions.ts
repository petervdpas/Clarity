// An accordion apart from the screen (conformance/accordions.json): the
// section shown open, and what a press on a header makes of it.

/** The section shown open when `open` is asked of `count`: it, when it is there; else none (0). */
export function shownSection(open: number | null | undefined, count: number): number {
  const asked = open ?? 1;
  return asked >= 1 && asked <= count ? asked : 0;
}

/** The open section after a press on the header of `press`: it, or none when it was the open one. */
export function pressedSection(open: number | null | undefined, press: number, count: number): number {
  return shownSection(open, count) === press ? 0 : press;
}
