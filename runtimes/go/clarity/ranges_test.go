// The slider's and the spin box's range (U1c) against the cases every
// runtime shares (conformance/ranges.json).

package clarity

import (
	"encoding/json"
	"os"
	"testing"
)

func TestARangeMovesAValueAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/ranges.json")
	must(t, err)
	type rangeCase struct {
		Value *string `json:"value"`
		At    string  `json:"at"`
		Min   *string `json:"min"`
		Max   *string `json:"max"`
		Step  string  `json:"step"`
		Key   string  `json:"key"`
		After string  `json:"after"`
	}
	var file struct {
		Keys  []rangeCase `json:"keys"`
		Drag  []rangeCase `json:"drag"`
		Typed []rangeCase `json:"typed"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Keys) == 0 || len(file.Drag) == 0 || len(file.Typed) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range file.Keys {
		got, err := RangeKeyed(c.Value, NumberRange{Min: c.Min, Max: c.Max, Step: c.Step}, c.Key)
		if err != nil || got != c.After {
			t.Errorf("keys %v %s: %q %v, want %q", c.Value, c.Key, got, err, c.After)
		}
	}
	for _, c := range file.Drag {
		got, err := RangeDragged(c.At, NumberRange{Min: c.Min, Max: c.Max, Step: c.Step})
		// In the drag cases the value is what the slider takes.
		if err != nil || got != *c.Value {
			t.Errorf("drag %s: %q %v, want %q", c.At, got, err, *c.Value)
		}
	}
	for _, c := range file.Typed {
		got, err := RangeTyped(*c.Value, NumberRange{Min: c.Min, Max: c.Max})
		if err != nil || got != c.After {
			t.Errorf("typed %s: %q %v, want %q", *c.Value, got, err, c.After)
		}
	}
	if _, err := RangeTyped("twelve", NumberRange{}); err == nil {
		t.Error("text that is no number is no value")
	}
	if _, err := RangeDragged("3", NumberRange{Step: "0"}); err == nil {
		t.Error("a drag needs a step")
	}
}
