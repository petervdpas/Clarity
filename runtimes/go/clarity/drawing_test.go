// A canvas drawn by code, and the angles it is drawn with, against the
// cases every stack's runtime shares (conformance/drawing.json).

package clarity

import (
	"encoding/json"
	"os"
	"testing"

	"github.com/shopspring/decimal"
)

func TestSinAndCosTakeDegreesAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/drawing.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Angles []struct {
			Degrees string `json:"degrees"`
			Sin     string `json:"sin"`
			Cos     string `json:"cos"`
		} `json:"angles"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Angles) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Angles {
		degrees := decimal.RequireFromString(c.Degrees)
		if got := Sin(degrees); !got.Equal(decimal.RequireFromString(c.Sin)) {
			t.Errorf("sin(%s) = %s, want %s", c.Degrees, got, c.Sin)
		}
		if got := Cos(degrees); !got.Equal(decimal.RequireFromString(c.Cos)) {
			t.Errorf("cos(%s) = %s, want %s", c.Degrees, got, c.Cos)
		}
	}
}

func TestADrawStartsAnEmptyDrawingThatTravelsWithThePart(t *testing.T) {
	ui := NewUi(WindowState{Window: "Main"})
	ui.BeginDrawing("dial")
	ui.Draw("dial", map[string]any{"op": "clear", "color": "#101810"})
	ui.Draw("dial", map[string]any{"op": "circle", "x": 50.0, "y": 50.0, "r": Units(decimal.RequireFromString("40.5")), "color": "accent"})
	reply, err := ui.Reply("Main", map[string]any{})
	if err != nil {
		t.Fatal(err)
	}
	got, _ := json.Marshal(reply.State.Parts["dial"])
	if string(got) != `{"drawing":[{"color":"#101810","op":"clear"},{"color":"accent","op":"circle","r":40.5,"x":50,"y":50}]}` {
		t.Errorf("drawing: %s", got)
	}
	again := NewUi(reply.State)
	again.BeginDrawing("dial")
	reply, _ = again.Reply("Main", map[string]any{})
	if got, _ := json.Marshal(reply.State.Parts["dial"]); string(got) != `{"drawing":[]}` {
		t.Errorf("drawn anew: %s", got)
	}
}

func TestACanvasSaysWhereItWasClickedAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/drawing.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Points []struct {
			Part json.RawMessage `json:"part"`
			X    int64           `json:"x"`
			Y    int64           `json:"y"`
		} `json:"points"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Points) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Points {
		var part PartState
		if err := json.Unmarshal(c.Part, &part); err != nil {
			t.Fatal(err)
		}
		ui := NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"board": part}})
		if x, y := ui.X("board"), ui.Y("board"); x != c.X || y != c.Y {
			t.Errorf("%s: %d,%d, want %d,%d", c.Part, x, y, c.X, c.Y)
		}
		// What the pointer holds besides x and y travels back unchanged.
		reply, _ := ui.Reply("Main", map[string]any{})
		got, _ := json.Marshal(reply.State.Parts["board"])
		if normal(got) != normal(c.Part) {
			t.Errorf("travels back as %s, want %s", got, c.Part)
		}
	}
}

// normal is JSON with its keys in order, to compare two texts by what they hold.
func normal(text []byte) string {
	var value any
	_ = json.Unmarshal(text, &value)
	out, _ := json.Marshal(value)
	return string(out)
}
