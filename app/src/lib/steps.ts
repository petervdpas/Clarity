// A wizard, the steps part (DESIGN 5.15, conformance/steps.json), apart from
// the screen: the step it shows, the buttons it has, and what Next does.
// Next runs the handler of `on next` on the step shown, then moves one step
// on, unless the handler refused (the step stays) or set the step itself
// (that step wins). Back goes one step back and runs nothing.

export type StepButton = "back" | "next" | "finish";

/** The step shown when `step` is asked of `count` steps: the nearest there is. */
export function shownStep(step: number | null | undefined, count: number): number {
  return Math.min(Math.max(step ?? 1, 1), Math.max(count, 1));
}

/** The buttons of step `shown` of `count`: Back from the second on, Next before the last, Finish on the last. */
export function stepButtons(shown: number, count: number): StepButton[] {
  const buttons: StepButton[] = [];
  if (shown > 1) buttons.push("back");
  buttons.push(shown < count ? "next" : "finish");
  return buttons;
}

/** Where a wizard keeps its step: the part's state. */
export type StepPlace = { get(): number | null | undefined; set(step: number): void };

/**
 * Next: the step shown is the part's step while `run` runs the handler of
 * `on next` (none: just move on); false from it is a refusal.
 */
export async function goNext(place: StepPlace, count: number, run?: () => Promise<boolean>): Promise<void> {
  const before = shownStep(place.get(), count);
  place.set(before);
  if (run && !(await run())) return;
  if (shownStep(place.get(), count) === before && before < count) place.set(before + 1);
}

/** Back: one step back; nothing runs and what was entered stays. */
export function goBack(place: StepPlace, count: number): void {
  const shown = shownStep(place.get(), count);
  if (shown > 1) place.set(shown - 1);
}
