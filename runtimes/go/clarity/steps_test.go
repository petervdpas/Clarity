// A wizard's walk through its steps (DESIGN 5.15) against the cases every
// runtime shares (conformance/steps.json, `walk`).

package clarity

import (
	"encoding/json"
	"fmt"
	"os"
	"reflect"
	"slices"
	"testing"
)

func TestAWizardWalksItsStepsAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/steps.json")
	must(t, err)
	var file struct {
		Walk []struct {
			Name    string `json:"name"`
			Steps   int64  `json:"steps"`
			Start   *int64 `json:"start"`
			Handler *struct {
				Refuses []int64 `json:"refuses"`
				Jumps   *struct {
					From int64 `json:"from"`
					To   int64 `json:"to"`
				} `json:"jumps"`
			} `json:"handler"`
			Do     []string `json:"do"`
			Expect struct {
				Shown   int64    `json:"shown"`
				Buttons []string `json:"buttons"`
				Ran     []string `json:"ran"`
			} `json:"expect"`
		} `json:"walk"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Walk) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range file.Walk {
		step := c.Start
		ran := []string{}
		for _, button := range c.Do {
			shown := ShownStep(step, c.Steps)
			switch button {
			case "back":
				back := BackStep(shown)
				step = &back
			case "finish":
				ran = append(ran, fmt.Sprintf("finish %d", shown))
			case "next":
				// The handler runs on the step shown, and leaves the step as it set it.
				left := shown
				if h := c.Handler; h != nil {
					ran = append(ran, fmt.Sprintf("next %d", shown))
					if slices.Contains(h.Refuses, shown) {
						step = &shown
						continue
					}
					if h.Jumps != nil && h.Jumps.From == shown {
						left = h.Jumps.To
					}
				}
				next := NextStep(shown, &left, c.Steps)
				step = &next
			}
		}
		shown := ShownStep(step, c.Steps)
		if shown != c.Expect.Shown || !reflect.DeepEqual(StepButtons(shown, c.Steps), c.Expect.Buttons) || !reflect.DeepEqual(ran, c.Expect.Ran) {
			t.Errorf("%s: shown %d, buttons %v, ran %v", c.Name, shown, StepButtons(shown, c.Steps), ran)
		}
	}
}
