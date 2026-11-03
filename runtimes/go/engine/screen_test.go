// Engines beside the screen on Go, as Clarity.UI runs them in the program's
// process: one instance per canvas, against the cases every host shares
// (conformance/screen-engines.json).

package engine

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

type screenFrame struct {
	Input []map[string]any `json:"input"`
	Tick  float64          `json:"tick"`
	Tells []Tell           `json:"tells"`
	Draws []any            `json:"draws"`
}

type screenCases struct {
	Uses   []string `json:"uses"`
	Frames []struct {
		Name  string    `json:"name"`
		Opens bool      `json:"opens"`
		Ticks []float64 `json:"ticks"`
		Draws [][]any   `json:"draws"`
	} `json:"frames"`
	Fails []struct {
		Name    string           `json:"name"`
		Seconds uint32           `json:"seconds"`
		Ticks   []float64        `json:"ticks"`
		Input   []map[string]any `json:"input"`
		Tick    float64          `json:"tick"`
		Fails   string           `json:"fails"`
	} `json:"fails"`
	Input []struct {
		Name   string        `json:"name"`
		Opens  bool          `json:"opens"`
		Frames []screenFrame `json:"frames"`
	} `json:"input"`
	Tells []struct {
		Name   string        `json:"name"`
		Opens  bool          `json:"opens"`
		Frames []screenFrame `json:"frames"`
	} `json:"tells"`
	Logs []struct {
		Opens bool             `json:"opens"`
		Input []map[string]any `json:"input"`
		Lines []string         `json:"lines"`
	} `json:"logs"`
	Written struct {
		Engines []string      `json:"engines"`
		Name    string        `json:"name"`
		Frames  []screenFrame `json:"frames"`
	} `json:"written"`
}

func loadScreenCases(t *testing.T) (screenCases, []byte) {
	t.Helper()
	var cases screenCases
	text, err := os.ReadFile("../../../conformance/screen-engines.json")
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	screen, err := os.ReadFile("../../../conformance/engines/screen.wasm")
	if err != nil {
		t.Fatal(err)
	}
	return cases, screen
}

// inputOf is an input as the conformance file writes it.
func inputOf(given []map[string]any) []Input {
	var out []Input
	for _, i := range given {
		switch {
		case i["pointer"] != nil:
			out = append(out, Input{Pointer: i["pointer"].(string), X: int64(i["x"].(float64)), Y: int64(i["y"].(float64))})
		case i["hear"] != nil:
			values, _ := i["values"].([]any)
			out = append(out, Input{Hear: i["hear"].(string), Values: values})
		default:
			out = append(out, Input{Key: i["key"].(string), Name: i["name"].(string)})
		}
	}
	return out
}

// same compares through JSON, as the file writes numbers.
func same(t *testing.T, got, want any, what string) {
	t.Helper()
	a, _ := json.Marshal(got)
	b, _ := json.Marshal(want)
	var x, y any
	_ = json.Unmarshal(a, &x)
	_ = json.Unmarshal(b, &y)
	if !reflect.DeepEqual(x, y) {
		t.Fatalf("%s:\n got %s\nwant %s", what, a, b)
	}
}

func TestEachFrameGivesTheCanvasItsShapes(t *testing.T) {
	cases, screen := loadScreenCases(t)
	for _, run := range cases.Frames {
		e, err := LoadScreen("Screen", screen, cases.Uses, run.Opens, Limits{Seconds: 5, Megabytes: 256})
		if err != nil {
			t.Fatal(err)
		}
		for i, tick := range run.Ticks {
			drawn, err := e.Frame(tick, nil)
			if err != nil {
				t.Fatal(err)
			}
			same(t, drawn.Shapes, run.Draws[i], run.Name)
		}
		e.Close()
	}
}

func TestAFrameThatFailsOrRunsTooLongEndsTheEngine(t *testing.T) {
	cases, screen := loadScreenCases(t)
	for _, fails := range cases.Fails {
		e, err := LoadScreen("Screen", screen, cases.Uses, true, Limits{Seconds: fails.Seconds, Megabytes: 256})
		if err != nil {
			t.Fatal(err)
		}
		for _, tick := range fails.Ticks {
			if _, err := e.Frame(tick, nil); err != nil {
				t.Fatal(err)
			}
		}
		_, err = e.Frame(fails.Tick, inputOf(fails.Input))
		if err == nil || err.Error() != fails.Fails {
			t.Fatalf("%s: got %v, want %s", fails.Name, err, fails.Fails)
		}
		// Ended, it draws no more.
		if _, again := e.Frame(16, nil); again == nil || again.Error() != fails.Fails {
			t.Fatalf("%s: an ended engine drew again: %v", fails.Name, again)
		}
	}
}

func TestPointerKeysAndHearsReachTheEngineBeforeItsTickAndItTells(t *testing.T) {
	cases, screen := loadScreenCases(t)
	runs := append(cases.Input, cases.Tells...)
	for _, run := range runs {
		e, err := LoadScreen("Screen", screen, cases.Uses, run.Opens, Limits{Seconds: 5, Megabytes: 256})
		if err != nil {
			t.Fatal(err)
		}
		if e.Hears() != (Hears{Pointer: true, Keys: true}) {
			t.Fatalf("hears %+v", e.Hears())
		}
		for i, frame := range run.Frames {
			drawn, err := e.Frame(frame.Tick, inputOf(frame.Input))
			if err != nil {
				t.Fatal(err)
			}
			same(t, drawn.Shapes, frame.Draws, run.Name)
			if frame.Tells != nil {
				same(t, drawn.Tells, frame.Tells, run.Name)
			}
			_ = i
		}
	}
}

func TestAnEngineBesideTheScreenWrittenInClarityRunsCompiledOrInterpreted(t *testing.T) {
	cases, _ := loadScreenCases(t)
	for _, file := range cases.Written.Engines {
		bytes, err := os.ReadFile("../../../conformance/engines/" + file)
		if err != nil {
			t.Fatal(err)
		}
		e, err := LoadScreen(cases.Written.Name, bytes, nil, false, Limits{Seconds: 5, Megabytes: 256})
		if err != nil {
			t.Fatal(err)
		}
		for _, frame := range cases.Written.Frames {
			drawn, err := e.Frame(frame.Tick, inputOf(frame.Input))
			if err != nil {
				t.Fatal(err)
			}
			same(t, drawn.Tells, frame.Tells, file)
			same(t, drawn.Shapes, frame.Draws, file)
		}
	}
}

func TestAnEngineBesideTheScreenLogsAndExportsItsFrameLoop(t *testing.T) {
	cases, screen := loadScreenCases(t)
	for _, run := range cases.Logs {
		var lines []string
		e, err := LoadScreen("Screen", screen, cases.Uses, run.Opens, Limits{Seconds: 5, Megabytes: 256})
		if err != nil {
			t.Fatal(err)
		}
		e.Serve(Services{Log: func(line string) { lines = append(lines, line) }})
		if _, err := e.Frame(0, inputOf(run.Input)); err != nil {
			t.Fatal(err)
		}
		same(t, lines, run.Lines, "logs")
	}
	test, err := os.ReadFile("../../../conformance/engines/test.wasm")
	if err != nil {
		t.Fatal(err)
	}
	_, err = LoadScreen("Test", test, []string{"clock", "random", "log", "language"}, false, Limits{Seconds: 5, Megabytes: 256})
	if err == nil || err.Error() != "engine Test lacks clarity_tick, which an engine beside the screen needs" {
		t.Fatalf("got %v", err)
	}
}

func TestInstancesGoByNumberAndAGoneOneSaysSo(t *testing.T) {
	cases, screen := loadScreenCases(t)
	var open Instances
	e, err := LoadScreen("Screen", screen, cases.Uses, true, Limits{Seconds: 5, Megabytes: 256})
	if err != nil {
		t.Fatal(err)
	}
	id := open.Open(e)
	if open.Hears(id) != (Hears{Pointer: true, Keys: true}) || open.Count() != 1 {
		t.Fatal("the open instance is not there")
	}
	if _, err := open.Frame(id, 0, nil); err != nil {
		t.Fatal(err)
	}
	open.Close(id)
	if _, err := open.Frame(id, 16, nil); err == nil || err.Error() != Gone || open.Count() != 0 {
		t.Fatalf("a closed instance drew: %v", err)
	}
}
