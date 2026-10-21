// Engines hosted by the Go runtime, against the cases every host shares
// (conformance/engines.json).

package engine_test

import (
	"encoding/hex"
	"encoding/json"
	"os"
	"reflect"
	"strings"
	"testing"

	"github.com/petervdpas/Clarity/runtimes/go/engine"
)

type cases struct {
	Engine    string   `json:"engine"`
	Functions []string `json:"functions"`
	Calls     []struct {
		Function string         `json:"function"`
		Args     []any          `json:"args"`
		Gives    map[string]any `json:"gives"`
	} `json:"calls"`
	Limits   engine.Limits `json:"limits"`
	Uses     []string      `json:"uses"`
	Services struct {
		Clock    int64  `json:"clock"`
		Seed     uint64 `json:"seed"`
		Language string `json:"language"`
	} `json:"services"`
	Served []struct {
		Function string         `json:"function"`
		Args     []any          `json:"args"`
		Gives    map[string]any `json:"gives"`
		Log      []string       `json:"log"`
	} `json:"served"`
	Limited []struct {
		Function string         `json:"function"`
		Args     []any          `json:"args"`
		Gives    map[string]any `json:"gives"`
		Fails    string         `json:"fails"`
	} `json:"limited"`
	Kept struct {
		State string `json:"state"`
		Open  bool   `json:"open"`
		Calls []struct {
			Function string         `json:"function"`
			Args     []any          `json:"args"`
			Gives    map[string]any `json:"gives"`
			Fails    string         `json:"fails"`
		} `json:"calls"`
	} `json:"kept"`
	Windows struct {
		State string `json:"state"`
		Open  bool   `json:"open"`
		Steps []struct {
			Window   uint64         `json:"window"`
			Close    uint64         `json:"close"`
			Function string         `json:"function"`
			Gives    map[string]any `json:"gives"`
		} `json:"steps"`
	} `json:"windows"`
	Refused []struct {
		Bytes     string    `json:"bytes"`
		Functions []string  `json:"functions"`
		Uses      *[]string `json:"uses"`
		Open      bool      `json:"open"`
		Message   string    `json:"message"`
	} `json:"refused"`
}

func load(t *testing.T) (cases, []byte) {
	text, err := os.ReadFile("../../../conformance/engines.json")
	if err != nil {
		t.Fatal(err)
	}
	var c cases
	if err := json.Unmarshal(text, &c); err != nil {
		t.Fatal(err)
	}
	wasm, err := os.ReadFile("../../../conformance/" + c.Engine)
	if err != nil {
		t.Fatal(err)
	}
	return c, wasm
}

func TestEveryCallGivesWhatEveryHostGives(t *testing.T) {
	c, wasm := load(t)
	e, err := engine.Load("Test", wasm, engine.Declaration{Functions: c.Functions, Uses: c.Uses, Limits: c.Limits})
	if err != nil {
		t.Fatal(err)
	}
	for _, call := range c.Calls {
		value, err := e.Call(call.Function, call.Args...)
		got := map[string]any{"value": value}
		if err != nil {
			got = map[string]any{"error": err.Error()}
		}
		// Both sides as JSON, so numbers compare alike.
		want, _ := json.Marshal(call.Gives)
		have, _ := json.Marshal(got)
		var w, h any
		_ = json.Unmarshal(want, &w)
		_ = json.Unmarshal(have, &h)
		if !reflect.DeepEqual(w, h) {
			t.Errorf("%s(%v) = %s, want %s", call.Function, call.Args, have, want)
		}
	}
}

func TestAModuleThatIsNoEngineIsRefusedWhenItLoads(t *testing.T) {
	c, wasm := load(t)
	for _, r := range c.Refused {
		bytes := wasm
		if r.Bytes != "" {
			bytes, _ = hex.DecodeString(r.Bytes)
		}
		uses := c.Uses
		if r.Uses != nil {
			uses = *r.Uses
		}
		_, err := engine.Load("Test", bytes, engine.Declaration{Functions: r.Functions, Uses: uses, Limits: c.Limits, Opens: r.Open})
		if err == nil || !strings.Contains(err.Error(), r.Message) || !strings.HasPrefix(err.Error(), "engine Test ") {
			t.Errorf("want %q, got %v", r.Message, err)
		}
	}
}

func TestAValueComesBackAsTheTypeCodeDeclares(t *testing.T) {
	c, wasm := load(t)
	e, _ := engine.Load("Test", wasm, engine.Declaration{Functions: c.Functions, Uses: c.Uses, Limits: c.Limits})
	sum, err := engine.As[int64](e.Call("add", 2, 40))
	if err != nil || sum != 42 {
		t.Fatalf("%v %v", sum, err)
	}
}

func TestACallKeepsToTheLimitsOfItsEngine(t *testing.T) {
	c, wasm := load(t)
	e, err := engine.Load("Test", wasm, engine.Declaration{Functions: c.Functions, Uses: c.Uses, Limits: c.Limits})
	if err != nil {
		t.Fatal(err)
	}
	for _, call := range c.Limited {
		value, err := e.Call(call.Function, call.Args...)
		if call.Fails != "" {
			if err == nil || !strings.HasPrefix(err.Error(), call.Fails) {
				t.Errorf("%s(%v): want an error starting %q, got %v, %v", call.Function, call.Args, call.Fails, value, err)
			}
			continue
		}
		have, _ := json.Marshal(map[string]any{"value": value})
		want, _ := json.Marshal(call.Gives)
		if err != nil || string(have) != string(want) {
			t.Errorf("%s(%v) = %s, %v, want %s", call.Function, call.Args, have, err, want)
		}
	}
}

func TestAnEngineUsesTheServicesItDeclares(t *testing.T) {
	c, wasm := load(t)
	e, err := engine.Load("Test", wasm, engine.Declaration{Functions: c.Functions, Uses: c.Uses, Limits: c.Limits})
	if err != nil {
		t.Fatal(err)
	}
	var lines []string
	clock, seed := c.Services.Clock, c.Services.Seed
	language := c.Services.Language
	e.Serve(engine.Services{Clock: &clock, Seed: &seed, Log: func(line string) { lines = append(lines, line) }, Language: func() string { return language }})
	for _, call := range c.Served {
		lines = nil
		value, err := e.Call(call.Function, call.Args...)
		have, _ := json.Marshal(map[string]any{"value": value})
		want, _ := json.Marshal(call.Gives)
		if err != nil || string(have) != string(want) {
			t.Errorf("%s(%v) = %s, %v, want %s", call.Function, call.Args, have, err, want)
		}
		if !reflect.DeepEqual(lines, call.Log) && (len(lines) > 0 || len(call.Log) > 0) {
			t.Errorf("%s logged %q, want %q", call.Function, lines, call.Log)
		}
	}
}

func TestAnEngineWithStateKeepsItsInstanceFromCallToCall(t *testing.T) {
	c, wasm := load(t)
	if c.Kept.State != "app" {
		t.Fatalf("state %q", c.Kept.State)
	}
	e, err := engine.Load("Test", wasm, engine.Declaration{Functions: c.Functions, Uses: c.Uses, Limits: c.Limits, State: engine.PerApp, Opens: c.Kept.Open})
	if err != nil {
		t.Fatal(err)
	}
	for _, call := range c.Kept.Calls {
		value, err := e.Call(call.Function, call.Args...)
		if call.Fails != "" {
			if err == nil || !strings.HasPrefix(err.Error(), call.Fails) {
				t.Errorf("%s: want an error starting %q, got %v, %v", call.Function, call.Fails, value, err)
			}
			continue
		}
		got := map[string]any{"value": value}
		if err != nil {
			got = map[string]any{"error": err.Error()}
		}
		have, _ := json.Marshal(got)
		want, _ := json.Marshal(call.Gives)
		if string(have) != string(want) {
			t.Errorf("%s = %s, want %s", call.Function, have, want)
		}
	}
}

func TestAnEngineWithStatePerWindowKeepsAnInstanceForEachWindow(t *testing.T) {
	c, wasm := load(t)
	if c.Windows.State != "window" {
		t.Fatalf("state %q", c.Windows.State)
	}
	e, err := engine.Load("Test", wasm, engine.Declaration{Functions: c.Functions, Uses: c.Uses, Limits: c.Limits, State: engine.PerWindow, Opens: c.Windows.Open})
	if err != nil {
		t.Fatal(err)
	}
	for _, step := range c.Windows.Steps {
		if step.Close != 0 {
			e.CloseWindow(step.Close)
			continue
		}
		value, err := e.CallIn(step.Window, step.Function)
		have, _ := json.Marshal(map[string]any{"value": value})
		want, _ := json.Marshal(step.Gives)
		if err != nil || string(have) != string(want) {
			t.Errorf("%s in window %d = %s, %v, want %s", step.Function, step.Window, have, err, want)
		}
	}
}
