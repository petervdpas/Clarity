// Engines beside the screen (DESIGN 5.12, conformance/screen-engines.json)
// as Clarity.UI runs them in the program's own process: one instance per
// canvas, which keeps its memory from frame to frame. Each frame hands over
// the input since the last (clarity_pointer, clarity_key, a hear by its
// function's name), then calls clarity_tick with the milliseconds since the
// last frame and clarity_draw for the canvas's shapes; what the answers
// tell comes back with the frame. A frame that fails ends the engine.

package engine

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"sync"
	"sync/atomic"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
)

// Input is the pointer, a key or a hear on a canvas, as its engine gets it
// with the next frame: one of Pointer, Key or Hear is set.
type Input struct {
	// Pointer is down, move or up, at X and Y in the canvas's units.
	Pointer string
	X, Y    int64
	// Key is down or up, of the key Name by Clarity's name.
	Key  string
	Name string
	// Hear is what a handler told the canvas: the engine's function of that name, with Values.
	Hear   string
	Values []any
}

// Drawn is a frame: the canvas's shapes (conformance/drawing.json) and what
// the engine told while making it.
type Drawn struct {
	Shapes []any  `json:"shapes"`
	Tells  []Tell `json:"tells"`
}

// Hears is which input an engine beside the screen takes.
type Hears struct {
	Pointer bool `json:"pointer"`
	Keys    bool `json:"keys"`
}

// ScreenEngine is an engine beside the screen for one canvas.
type ScreenEngine struct {
	engine *Engine
	hears  Hears
	ended  error
}

// LoadScreen loads bytes as engine name for one canvas; it refuses a module
// that is no engine, imports more than uses or lacks its frame loop. With
// opens, clarity_open runs before the first frame.
func LoadScreen(name string, bytes []byte, uses []string, opens bool, limits Limits) (*ScreenEngine, error) {
	// One instance for the canvas, kept from frame to frame; no window's closing takes it.
	e, err := Load(name, bytes, Declaration{Uses: uses, Limits: limits, State: PerApp})
	if err != nil {
		return nil, err
	}
	exported := e.module.ExportedFunctions()
	for _, needed := range []string{"clarity_tick", "clarity_draw"} {
		if _, ok := exported[needed]; !ok {
			return nil, &Error{What: fmt.Sprintf("engine %s lacks %s, which an engine beside the screen needs", name, needed)}
		}
	}
	if _, ok := exported["clarity_open"]; opens && !ok {
		return nil, &Error{What: fmt.Sprintf("engine %s lacks clarity_open, which its on open needs", name)}
	}
	e.opens = opens
	_, pointer := exported["clarity_pointer"]
	_, keys := exported["clarity_key"]
	return &ScreenEngine{engine: e, hears: Hears{Pointer: pointer, Keys: keys}}, nil
}

// Serve gives the engine services in place of the system's.
func (s *ScreenEngine) Serve(services Services) { s.engine.Serve(services) }

// Hears is the pointer and keys it takes.
func (s *ScreenEngine) Hears() Hears { return s.hears }

// call calls function on the canvas's instance; a refusal ends the engine as any failure does.
func (s *ScreenEngine) call(function string, args []any, told *[]Tell) (any, error) {
	value, err := s.engine.run(0, Watch{}, function, args, told)
	var refused *clarity.RefusedError
	if errors.As(err, &refused) {
		return nil, &Error{What: fmt.Sprintf("engine %s failed in %s: %s", s.engine.name, function, refused.Error())}
	}
	return value, err
}

// Frame is one frame: the input since the last goes first, then ms. Once a
// frame failed, every frame fails alike.
func (s *ScreenEngine) Frame(ms float64, input []Input) (Drawn, error) {
	if s.ended != nil {
		return Drawn{}, s.ended
	}
	drawn, err := s.frame(ms, input)
	if err != nil {
		s.ended = err
		s.Close()
	}
	return drawn, err
}

func (s *ScreenEngine) frame(ms float64, input []Input) (Drawn, error) {
	told := []Tell{}
	for _, given := range input {
		var err error
		switch {
		case given.Pointer != "" && s.hears.Pointer:
			_, err = s.call("clarity_pointer", []any{given.Pointer, given.X, given.Y}, &told)
		case given.Key != "" && s.hears.Keys:
			_, err = s.call("clarity_key", []any{given.Key, given.Name}, &told)
		case given.Hear != "":
			values := given.Values
			if values == nil {
				values = []any{}
			}
			_, err = s.call(given.Hear, values, &told)
		}
		if err != nil {
			return Drawn{}, err
		}
	}
	// Whole milliseconds go as whole numbers, as every renderer sends them.
	var tick any = ms
	if ms == math.Trunc(ms) && math.Abs(ms) < 9e15 {
		tick = int64(ms)
	}
	if _, err := s.call("clarity_tick", []any{tick}, &told); err != nil {
		return Drawn{}, err
	}
	shapes, err := s.call("clarity_draw", nil, &told)
	if err != nil {
		return Drawn{}, err
	}
	list, ok := shapes.([]any)
	if !ok {
		return Drawn{}, &Error{What: fmt.Sprintf("engine %s failed in clarity_draw: it gave no list of shapes", s.engine.name)}
	}
	return Drawn{Shapes: list, Tells: told}, nil
}

// Close ends the engine: its instance goes.
func (s *ScreenEngine) Close() {
	e := s.engine
	e.mu.Lock()
	defer e.mu.Unlock()
	if l, ok := e.kept[0]; ok {
		l.module.Close(context.Background())
		delete(e.kept, 0)
	}
}

// Gone is what a frame of an instance that is gone answers.
const Gone = "The engine ended."

// Instances are the engines beside the screen a program has open, each by
// its number: the screen holds only the number. A frame runs with only its
// own instance held, so the canvases of several windows draw at once.
type Instances struct {
	next atomic.Uint64
	mu   sync.Mutex
	open map[uint64]*held
}

type held struct {
	mu     sync.Mutex
	engine *ScreenEngine
}

// Open keeps engine open: its number.
func (in *Instances) Open(engine *ScreenEngine) uint64 {
	id := in.next.Add(1)
	in.mu.Lock()
	defer in.mu.Unlock()
	if in.open == nil {
		in.open = map[uint64]*held{}
	}
	in.open[id] = &held{engine: engine}
	return id
}

func (in *Instances) get(id uint64) *held {
	in.mu.Lock()
	defer in.mu.Unlock()
	return in.open[id]
}

// Hears is the pointer and keys instance id takes; none when it is gone.
func (in *Instances) Hears(id uint64) Hears {
	h := in.get(id)
	if h == nil {
		return Hears{}
	}
	h.mu.Lock()
	defer h.mu.Unlock()
	return h.engine.Hears()
}

// Frame is one frame of instance id; one that is gone says so.
func (in *Instances) Frame(id uint64, ms float64, input []Input) (Drawn, error) {
	h := in.get(id)
	if h == nil {
		return Drawn{}, errors.New(Gone)
	}
	h.mu.Lock()
	defer h.mu.Unlock()
	return h.engine.Frame(ms, input)
}

// Close ends instance id.
func (in *Instances) Close(id uint64) {
	in.mu.Lock()
	h := in.open[id]
	delete(in.open, id)
	in.mu.Unlock()
	if h != nil {
		h.mu.Lock()
		h.engine.Close()
		h.mu.Unlock()
	}
}

// Count is how many are open.
func (in *Instances) Count() int {
	in.mu.Lock()
	defer in.mu.Unlock()
	return len(in.open)
}

// Screens are a program's engines beside the screen as Clarity.UI asks for
// them (package ui's Engines): each made by its name, its instances by number.
type Screens struct {
	// Make makes a new instance of each engine by its name.
	Make map[string]func() (*ScreenEngine, error)
	open Instances
}

// EngineOpen opens a new instance of engine name; found is false when there is none of that name.
func (s *Screens) EngineOpen(name string) (uint64, bool, error) {
	make, ok := s.Make[name]
	if !ok {
		return 0, false, nil
	}
	engine, err := make()
	if err != nil {
		return 0, true, err
	}
	return s.open.Open(engine), true, nil
}

// EngineHears is whether instance id takes the pointer and keys.
func (s *Screens) EngineHears(id uint64) (bool, bool) {
	hears := s.open.Hears(id)
	return hears.Pointer, hears.Keys
}

// EngineFrame is one frame of instance id, its input as the library sends
// it: [{"pointer":{kind,x,y}} | {"key":{kind,name}} | {"hear":{name,values}}].
func (s *Screens) EngineFrame(id uint64, ms float64, input json.RawMessage) (json.RawMessage, error) {
	var sent []struct {
		Pointer *struct {
			Kind string  `json:"kind"`
			X    float64 `json:"x"`
			Y    float64 `json:"y"`
		} `json:"pointer"`
		Key *struct {
			Kind string `json:"kind"`
			Name string `json:"name"`
		} `json:"key"`
		Hear *struct {
			Name   string `json:"name"`
			Values []any  `json:"values"`
		} `json:"hear"`
	}
	if len(input) > 0 {
		if err := json.Unmarshal(input, &sent); err != nil {
			return nil, &Error{What: "the input of a frame does not read: " + err.Error()}
		}
	}
	given := make([]Input, 0, len(sent))
	for _, i := range sent {
		switch {
		case i.Pointer != nil:
			given = append(given, Input{Pointer: i.Pointer.Kind, X: int64(math.Round(i.Pointer.X)), Y: int64(math.Round(i.Pointer.Y))})
		case i.Key != nil:
			given = append(given, Input{Key: i.Key.Kind, Name: i.Key.Name})
		case i.Hear != nil:
			given = append(given, Input{Hear: i.Hear.Name, Values: i.Hear.Values})
		}
	}
	drawn, err := s.open.Frame(id, ms, given)
	if err != nil {
		return nil, err
	}
	return json.Marshal(drawn)
}

// EngineClose ends instance id.
func (s *Screens) EngineClose(id uint64) { s.open.Close(id) }
