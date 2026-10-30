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
			Part  json.RawMessage `json:"part"`
			X     int64           `json:"x"`
			Y     int64           `json:"y"`
			FromX int64           `json:"fromX"`
			FromY int64           `json:"fromY"`
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
		if x, y := ui.FromX("board"), ui.FromY("board"); x != c.FromX || y != c.FromY {
			t.Errorf("%s: from %d,%d, want %d,%d", c.Part, x, y, c.FromX, c.FromY)
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

func TestAFoldablePartIsFoldedAsEveryRuntimeSays(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/folding.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []struct {
			Design bool            `json:"design"`
			Part   json.RawMessage `json:"part"`
			Folded bool            `json:"folded"`
		} `json:"cases"`
		Set []struct {
			Folded bool            `json:"folded"`
			Part   json.RawMessage `json:"part"`
		} `json:"set"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Cases) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Cases {
		var part PartState
		if err := json.Unmarshal(c.Part, &part); err != nil {
			t.Fatal(err)
		}
		ui := NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"details": part}})
		if got := ui.Folded("details", c.Design); got != c.Folded {
			t.Errorf("%s with design %v: %v, want %v", c.Part, c.Design, got, c.Folded)
		}
	}
	for _, c := range file.Set {
		ui := NewUi(WindowState{Window: "Main"})
		ui.SetFolded("details", c.Folded)
		reply, _ := ui.Reply("Main", map[string]any{})
		got, _ := json.Marshal(reply.State.Parts["details"])
		if normal(got) != normal(c.Part) {
			t.Errorf("set %v sends %s, want %s", c.Folded, got, c.Part)
		}
	}
}

func TestASidebarIsCollapsedAndMaximizedAsEveryRuntimeSays(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/sidebars.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Collapsed []struct {
			Design    bool            `json:"design"`
			Part      json.RawMessage `json:"part"`
			Collapsed bool            `json:"collapsed"`
		} `json:"collapsed"`
		Maximized []struct {
			Part      json.RawMessage `json:"part"`
			Maximized bool            `json:"maximized"`
		} `json:"maximized"`
		Set []struct {
			Collapsed *bool           `json:"collapsed"`
			Maximized *bool           `json:"maximized"`
			Part      json.RawMessage `json:"part"`
		} `json:"set"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Collapsed) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	uiWith := func(raw json.RawMessage) *Ui {
		var part PartState
		if err := json.Unmarshal(raw, &part); err != nil {
			t.Fatal(err)
		}
		return NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"nav": part}})
	}
	for _, c := range file.Collapsed {
		if got := uiWith(c.Part).Collapsed("nav", c.Design); got != c.Collapsed {
			t.Errorf("%s with design %v: %v, want %v", c.Part, c.Design, got, c.Collapsed)
		}
	}
	for _, c := range file.Maximized {
		if got := uiWith(c.Part).Maximized("nav"); got != c.Maximized {
			t.Errorf("%s: %v, want %v", c.Part, got, c.Maximized)
		}
	}
	for _, c := range file.Set {
		ui := NewUi(WindowState{Window: "Main"})
		if c.Collapsed != nil {
			ui.SetCollapsed("nav", *c.Collapsed)
		}
		if c.Maximized != nil {
			ui.SetMaximized("nav", *c.Maximized)
		}
		reply, _ := ui.Reply("Main", map[string]any{})
		got, _ := json.Marshal(reply.State.Parts["nav"])
		if normal(got) != normal(c.Part) {
			t.Errorf("sends %s, want %s", got, c.Part)
		}
	}
}

func TestAWizardsStepIsAsEveryRuntimeSays(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/steps.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Step []struct {
			Part json.RawMessage `json:"part"`
			Step int64           `json:"step"`
		} `json:"step"`
		Set []struct {
			Step int64           `json:"step"`
			Part json.RawMessage `json:"part"`
		} `json:"set"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Step) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Step {
		var part PartState
		if err := json.Unmarshal(c.Part, &part); err != nil {
			t.Fatal(err)
		}
		ui := NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"wizard": part}})
		if got := ui.Step("wizard"); got != c.Step {
			t.Errorf("%s: %d, want %d", c.Part, got, c.Step)
		}
	}
	for _, c := range file.Set {
		ui := NewUi(WindowState{Window: "Main"})
		ui.SetStep("wizard", c.Step)
		reply, _ := ui.Reply("Main", map[string]any{})
		got, _ := json.Marshal(reply.State.Parts["wizard"])
		if normal(got) != normal(c.Part) {
			t.Errorf("sends %s, want %s", got, c.Part)
		}
	}
}

func TestAnAccordionsOpenSectionIsAsEveryRuntimeSays(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/accordions.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Open []struct {
			Part json.RawMessage `json:"part"`
			Open int64           `json:"open"`
		} `json:"open"`
		Set []struct {
			Open int64           `json:"open"`
			Part json.RawMessage `json:"part"`
		} `json:"set"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Open) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Open {
		var part PartState
		if err := json.Unmarshal(c.Part, &part); err != nil {
			t.Fatal(err)
		}
		ui := NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"faq": part}})
		if got := ui.OpenSection("faq"); got != c.Open {
			t.Errorf("%s: %d, want %d", c.Part, got, c.Open)
		}
	}
	for _, c := range file.Set {
		ui := NewUi(WindowState{Window: "Main"})
		ui.SetOpenSection("faq", c.Open)
		reply, _ := ui.Reply("Main", map[string]any{})
		got, _ := json.Marshal(reply.State.Parts["faq"])
		if normal(got) != normal(c.Part) {
			t.Errorf("sends %s, want %s", got, c.Part)
		}
	}
}

func TestARibbonsOpenTabIsAsEveryRuntimeSays(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/ribbons.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Tab []struct {
			Part json.RawMessage `json:"part"`
			Tab  int64           `json:"tab"`
		} `json:"tab"`
		Set []struct {
			Tab  int64           `json:"tab"`
			Part json.RawMessage `json:"part"`
		} `json:"set"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Tab) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Tab {
		var part PartState
		if err := json.Unmarshal(c.Part, &part); err != nil {
			t.Fatal(err)
		}
		ui := NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"mainRibbon": part}})
		if got := ui.Tab("mainRibbon"); got != c.Tab {
			t.Errorf("%s: %d, want %d", c.Part, got, c.Tab)
		}
	}
	for _, c := range file.Set {
		ui := NewUi(WindowState{Window: "Main"})
		ui.SetTab("mainRibbon", c.Tab)
		reply, _ := ui.Reply("Main", map[string]any{})
		got, _ := json.Marshal(reply.State.Parts["mainRibbon"])
		if normal(got) != normal(c.Part) {
			t.Errorf("sends %s, want %s", got, c.Part)
		}
	}
}
