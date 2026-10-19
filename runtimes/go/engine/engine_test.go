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
	Refused []struct {
		Bytes     string   `json:"bytes"`
		Functions []string `json:"functions"`
		Message   string   `json:"message"`
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
	e, err := engine.Load("Test", wasm, c.Functions)
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
		_, err := engine.Load("Test", bytes, r.Functions)
		if err == nil || !strings.Contains(err.Error(), r.Message) || !strings.HasPrefix(err.Error(), "engine Test ") {
			t.Errorf("want %q, got %v", r.Message, err)
		}
	}
}

func TestAValueComesBackAsTheTypeCodeDeclares(t *testing.T) {
	c, wasm := load(t)
	e, _ := engine.Load("Test", wasm, c.Functions)
	sum, err := engine.As[int64](e.Call("add", 2, 40))
	if err != nil || sum != 42 {
		t.Fatalf("%v %v", sum, err)
	}
}
