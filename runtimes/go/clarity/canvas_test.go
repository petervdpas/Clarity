package clarity

// Canvas modes (DESIGN 5.31), as every runtime does them
// (conformance/canvas-modes.json).

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

type canvasCases struct {
	SVG []struct {
		Name   string           `json:"name"`
		Units  [2]int           `json:"units"`
		Shapes []map[string]any `json:"shapes"`
		Text   string           `json:"text"`
	} `json:"svg"`
	Read []struct {
		Name   string           `json:"name"`
		Text   string           `json:"text"`
		Units  *[2]int          `json:"units"`
		Shapes []map[string]any `json:"shapes"`
	} `json:"read"`
	Actions []struct {
		Name     string                     `json:"name"`
		Canvas   CanvasArgs                 `json:"-"`
		Data     map[string]any             `json:"data"`
		Action   map[string]json.RawMessage `json:"action"`
		After    map[string]any             `json:"after"`
		Selected json.RawMessage            `json:"selected"`
		Refused  bool                       `json:"refused"`
	} `json:"actions"`
}

func loadCanvasCases(t *testing.T) (canvasCases, []CanvasArgs) {
	text, err := os.ReadFile("../../../conformance/canvas-modes.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases canvasCases
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	var raw struct {
		Actions []CanvasArgs `json:"actions"`
	}
	if err := json.Unmarshal(text, &raw); err != nil {
		t.Fatal(err)
	}
	return cases, raw.Actions
}

func TestDrawingSVG(t *testing.T) {
	cases, _ := loadCanvasCases(t)
	for _, c := range cases.SVG {
		shapes := asJSON(c.Shapes).([]any)
		if got := DrawingText(c.Units, shapes); got != c.Text {
			t.Errorf("%s: wrote\n%s\nwant\n%s", c.Name, got, c.Text)
		}
		units, read := DrawingShapes(c.Text, [2]int{1000, 1000})
		if units != c.Units || !reflect.DeepEqual(asJSON(read), asJSON(shapes)) {
			t.Errorf("%s: read %v %v, want %v %v", c.Name, units, asJSON(read), c.Units, shapes)
		}
	}
	for _, c := range cases.Read {
		units, read := DrawingShapes(c.Text, [2]int{1000, 1000})
		if c.Units != nil && units != *c.Units {
			t.Errorf("%s: units %v, want %v", c.Name, units, *c.Units)
		}
		if !reflect.DeepEqual(asJSON(read), asJSON(c.Shapes)) {
			t.Errorf("%s: read %v, want %v", c.Name, read, c.Shapes)
		}
	}
}

func TestCanvasActions(t *testing.T) {
	cases, args := loadCanvasCases(t)
	for i, c := range cases.Actions {
		state := NewWindowState("", c.Data)
		changed, err := CanvasEvent(state, args[i], c.Action)
		if err != nil {
			t.Errorf("%s: %v", c.Name, err)
			continue
		}
		if changed == c.Refused {
			t.Errorf("%s: changed %v, refused %v", c.Name, changed, c.Refused)
			continue
		}
		if c.Refused {
			if !reflect.DeepEqual(asJSON(state.Data), asJSON(c.Data)) {
				t.Errorf("%s: a refused action changed the data", c.Name)
			}
			continue
		}
		if !reflect.DeepEqual(asJSON(state.Data), asJSON(c.After)) {
			t.Errorf("%s: data %v, want %v", c.Name, asJSON(state.Data), c.After)
		}
		var want *int
		_ = json.Unmarshal(c.Selected, &want)
		got, has := state.Selected[args[i].Part]
		if (want == nil) == has || (want != nil && got != *want) {
			t.Errorf("%s: selected %v %v, want %v", c.Name, got, has, want)
		}
	}
}
