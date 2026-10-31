package clarity

// A wizard, the steps part (DESIGN 5.15, conformance/steps.json), decided
// here in the runtime: the step it shows, the buttons it has, and where Next
// and Back go. Next runs `on next` on the step shown, then moves one step
// on, unless the handler refused (the step stays) or set the step itself
// (that step wins). Back goes one step back and runs nothing.

// ShownStep is the step shown for step of count steps: the nearest there
// is, the first when there is none.
func ShownStep(step *int64, count int64) int64 {
	shown := int64(1)
	if step != nil {
		shown = *step
	}
	return min(max(shown, 1), max(count, 1))
}

// StepButtons are the buttons of step shown of count: Back from the second
// on, Next before the last, Finish on the last.
func StepButtons(shown, count int64) []string {
	buttons := []string{}
	if shown > 1 {
		buttons = append(buttons, "back")
	}
	if shown < count {
		return append(buttons, "next")
	}
	return append(buttons, "finish")
}

// NextStep is the step after Next on step before, with left the step the
// handler left (nil: it ran none): one on, unless the handler set another.
func NextStep(before int64, left *int64, count int64) int64 {
	after := ShownStep(left, count)
	if left == nil {
		after = before
	}
	if after == before && before < count {
		return before + 1
	}
	return after
}

// BackStep is the step after Back on step shown.
func BackStep(shown int64) int64 {
	return max(shown-1, 1)
}
