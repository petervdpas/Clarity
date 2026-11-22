package clarity

import (
	"encoding/json"
	"os"
	"testing"
)

// A date compared with an empty one, as every stack compares them
// (conformance/compare-empty.json): generated Go orders dates through
// CompareDates and CompareDateTimes.
func TestAnEmptyDateIsSmallestAsEveryStackComparesIt(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/compare-empty.json")
	must(t, err)
	var file struct {
		Cases []map[string]any `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	for _, c := range file.Cases {
		var order int
		if c["kind"] == "date" {
			date := func(v any) *Date {
				if v == nil {
					return nil
				}
				d, err := ParseDate(v.(string))
				must(t, err)
				return &d
			}
			order = CompareDates(date(c["a"]), date(c["b"]))
		} else {
			moment := func(v any) *DateTime {
				if v == nil {
					return nil
				}
				d, err := ParseDateTime(v.(string))
				must(t, err)
				return &d
			}
			order = CompareDateTimes(moment(c["a"]), moment(c["b"]))
		}
		got := map[string]bool{"<": order < 0, "<=": order <= 0, ">": order > 0, ">=": order >= 0}
		for _, op := range []string{"<", "<=", ">", ">="} {
			if got[op] != c[op].(bool) {
				t.Errorf("%v %s %v: %v, want %v", c["a"], op, c["b"], got[op], c[op])
			}
		}
	}
}
