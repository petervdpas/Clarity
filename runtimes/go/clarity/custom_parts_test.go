package clarity

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

// sameJSON says whether a value is the JSON the shared file gives.
func sameJSON(t *testing.T, got any, want json.RawMessage) bool {
	t.Helper()
	text, err := json.Marshal(got)
	if err != nil {
		t.Fatal(err)
	}
	var a, b any
	if err := json.Unmarshal(text, &a); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(want, &b); err != nil {
		t.Fatal(err)
	}
	return reflect.DeepEqual(a, b)
}

func TestCustomPartsAreAsEveryRuntimeSays(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/custom-parts.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Read []struct {
			Part  json.RawMessage `json:"part"`
			Name  string          `json:"name"`
			Value json.RawMessage `json:"value"`
		} `json:"read"`
		Set []struct {
			Sets []json.RawMessage `json:"sets"`
			Part json.RawMessage   `json:"part"`
		} `json:"set"`
		Hear []struct {
			Hears [][]json.RawMessage `json:"hears"`
			Reply json.RawMessage     `json:"reply"`
		} `json:"hear"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Read) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Read {
		var part PartState
		if err := json.Unmarshal(c.Part, &part); err != nil {
			t.Fatal(err)
		}
		ui := NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"arena": part}})
		value, _ := ui.PartValue("arena", c.Name)
		if !sameJSON(t, value, c.Value) {
			t.Errorf("read %s of %s: %v, want %s", c.Name, c.Part, value, c.Value)
		}
	}
	for _, c := range file.Set {
		ui := NewUi(WindowState{Window: "Main"})
		for _, raw := range c.Sets {
			var set []any
			if err := json.Unmarshal(raw, &set); err != nil {
				t.Fatal(err)
			}
			ui.SetPartValue("arena", set[0].(string), set[1])
		}
		reply, _ := ui.Reply("Main", map[string]any{})
		if !sameJSON(t, reply.State.Parts["arena"], c.Part) {
			t.Errorf("set: %+v, want %s", reply.State.Parts["arena"], c.Part)
		}
	}
	for _, c := range file.Hear {
		ui := NewUi(WindowState{Window: "Main"})
		for _, hear := range c.Hears {
			var part, name string
			var values []any
			_ = json.Unmarshal(hear[0], &part)
			_ = json.Unmarshal(hear[1], &name)
			_ = json.Unmarshal(hear[2], &values)
			ui.Hear(part, name, values...)
		}
		reply, _ := ui.Reply("Main", map[string]any{})
		text, _ := json.Marshal(reply)
		var whole map[string]json.RawMessage
		_ = json.Unmarshal(text, &whole)
		got, ok := whole["hears"]
		if !ok {
			got = json.RawMessage("null")
		}
		var a, b any
		_ = json.Unmarshal(got, &a)
		_ = json.Unmarshal(c.Reply, &b)
		if !reflect.DeepEqual(a, b) {
			t.Errorf("hears: %s, want %s", got, c.Reply)
		}
	}
}

func TestAPartValueReadsAsItsTypeOrItsFallback(t *testing.T) {
	ui := NewUi(WindowState{Window: "Main", Parts: map[string]PartState{"arena": {Values: map[string]any{"speed": float64(7)}}}})
	speed, err := PartValueAs[int64](ui, "arena", "speed", "3")
	if err != nil || speed != 7 {
		t.Errorf("speed %d %v, want 7", speed, err)
	}
	title, err := PartValueAs[string](ui, "arena", "title", `"Level one"`)
	if err != nil || title != "Level one" {
		t.Errorf("title %q %v", title, err)
	}
}
