// Reordering (DESIGN 5.30) against the cases every runtime shares
// (conformance/reorder.json).

package clarity

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

func TestARowMovesAsEveryRuntimeMovesIt(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/reorder.json")
	must(t, err)
	type reorderCase struct {
		Name    string           `json:"name"`
		Rows    []map[string]any `json:"rows"`
		Move    Move             `json:"move"`
		Order   []int            `json:"order"`
		Parents []any            `json:"parents"`
		Numbers []int64          `json:"numbers"`
		Refused bool             `json:"refused"`
	}
	var file struct {
		Lists []reorderCase `json:"lists"`
		Trees []reorderCase `json:"trees"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Lists) == 0 || len(file.Trees) == 0 {
		t.Fatal("no cases")
	}
	run := func(cases []reorderCase, tree bool) {
		for _, c := range cases {
			rows := make([]TreeRow, len(c.Rows))
			for i, r := range c.Rows {
				rows[i] = TreeRow{ID: r["id"], Parent: r["parent"]}
			}
			got, moved := Reorder(rows, c.Move, tree)
			if c.Refused {
				if moved {
					t.Errorf("%s: moved %+v, but it is refused", c.Name, got)
				}
				continue
			}
			if !moved {
				t.Errorf("%s: refused", c.Name)
				continue
			}
			if !reflect.DeepEqual(got.Order, c.Order) || !reflect.DeepEqual(got.Numbers, c.Numbers) {
				t.Errorf("%s: order %v numbers %v, want %v %v", c.Name, got.Order, got.Numbers, c.Order, c.Numbers)
			}
			if tree && !reflect.DeepEqual(asJSON(got.Parents), asJSON(c.Parents)) {
				t.Errorf("%s: parents %v, want %v", c.Name, got.Parents, c.Parents)
			}
		}
	}
	run(file.Lists, false)
	run(file.Trees, true)
}
