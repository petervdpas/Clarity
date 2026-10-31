package clarity

import (
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"testing"
)

// The edit bus against the cases every runtime shares (conformance/edit-bus.json).
func TestTheEditBusKeepsTheRecordAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/edit-bus.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []struct {
			Name  string                       `json:"name"`
			Start map[string]any               `json:"start"`
			Steps []map[string]json.RawMessage `json:"steps"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	type made struct {
		Part  string         `json:"part"`
		Scope string         `json:"scope"`
		Data  map[string]any `json:"data"`
		Steps []int64        `json:"steps"`
	}
	for _, c := range file.Cases {
		bus := &EditBus{}
		bus.Reset(c.Start)
		data := c.Start
		refuse := false
		for i, s := range c.Steps {
			var m made
			switch {
			case s["edit"] != nil:
				_ = json.Unmarshal(s["edit"], &m)
				bus.Edit(m.Part, m.Scope, m.Data, m.Steps)
				data = m.Data
			case s["drag"] != nil:
				_ = json.Unmarshal(s["drag"], &m)
				bus.Drag(m.Part, m.Scope, m.Data, m.Steps)
				data = m.Data
			case s["reply"] != nil:
				_ = json.Unmarshal(s["reply"], &m)
				bus.Reset(m.Data)
				data = m.Data
			default:
				var scope string
				back := s["undo"] != nil
				if back {
					_ = json.Unmarshal(s["undo"], &scope)
				} else {
					_ = json.Unmarshal(s["redo"], &scope)
				}
				refuse = s["refused"] != nil
				var taken []int64
				take := func(steps []int64, _ bool) error {
					if refuse {
						return errors.New("changed since")
					}
					taken = steps
					return nil
				}
				after, found, err := bus.Undo(scope, data, back, take)
				switch {
				case s["nothing"] != nil:
					if found {
						t.Errorf("%s, step %d: something to take back", c.Name, i+1)
					}
				case refuse:
					if err == nil {
						t.Errorf("%s, step %d: not refused", c.Name, i+1)
					}
				default:
					var want struct {
						Data  map[string]any `json:"data"`
						Steps []int64        `json:"steps"`
					}
					want.Data = map[string]any{}
					_ = json.Unmarshal(s["data"], &want.Data)
					_ = json.Unmarshal(s["steps"], &want.Steps)
					if !found || err != nil {
						t.Errorf("%s, step %d: nothing taken back (%v)", c.Name, i+1, err)
						continue
					}
					if !reflect.DeepEqual(after, want.Data) {
						t.Errorf("%s, step %d: data %v, want %v", c.Name, i+1, after, want.Data)
					}
					if len(taken) != len(want.Steps) || (len(taken) > 0 && !reflect.DeepEqual(taken, want.Steps)) {
						t.Errorf("%s, step %d: steps %v, want %v", c.Name, i+1, taken, want.Steps)
					}
					data = after
				}
			}
		}
	}
}

// The baseline is the window's data as the last reply or edit left it: what
// the runtime puts back when it takes a change back.
func TestTheEditBusGivesItsBaseline(t *testing.T) {
	bus := &EditBus{}
	if bus.Baseline() != nil {
		t.Fatal("a bus without a reply has no baseline")
	}
	bus.Reset(map[string]any{"name": "Ada"})
	kept := bus.Baseline()
	kept["name"] = "changed"
	bus.Edit("nameBox", "form", map[string]any{"name": "Ann"}, nil)
	if got := bus.Baseline(); got["name"] != "Ann" {
		t.Fatal("the edit moved the baseline on, and a copy changed nothing", got)
	}
}

// A table's edit keeps to its editable columns: rows the same in number and
// order, other fields as they were.
func TestATableEditKeepsToItsEditableColumns(t *testing.T) {
	before := []any{map[string]any{"id": 1.0, "title": "Milk", "hours": 9.0}}
	cases := []struct {
		after any
		fits  bool
	}{
		{[]any{map[string]any{"id": 1, "title": "Milk", "hours": 10}}, true},
		{[]any{map[string]any{"id": 1, "title": "Tea", "hours": 9}}, false},
		{[]any{map[string]any{"id": 1, "title": "Milk", "hours": 9, "note": "x"}}, false},
		{[]any{map[string]any{"id": 1, "title": "Milk", "hours": 9}, map[string]any{"id": 2}}, false},
		{[]any{}, false},
	}
	for _, c := range cases {
		if got := TableEditFits(before, c.after, []string{"hours"}); got != c.fits {
			t.Errorf("%v: %v", c.after, got)
		}
	}
}
