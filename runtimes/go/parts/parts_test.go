package parts

import (
	"encoding/json"
	"fmt"
	"reflect"
	"sync"
	"testing"
)

// fake is windows without a display: each child a number of its own.
type fake struct{ next uint64 }

func (f *fake) child(uint64, int, int, int, int) uint64 { f.next++; return f.next }
func (f *fake) window(child uint64) Window              { return Window{Handle: uintptr(child)} }
func (f *fake) move(uint64, int, int, int, int)         {}
func (f *fake) show(uint64, bool)                       {}
func (f *fake) destroy(uint64)                          {}
func (f *fake) flush()                                  {}
func (f *fake) close()                                  {}

var (
	callsMu sync.Mutex
	calls   []string
)

func called(text string) {
	callsMu.Lock()
	defer callsMu.Unlock()
	calls = append(calls, text)
}

func taken() []string {
	callsMu.Lock()
	defer callsMu.Unlock()
	out := calls
	calls = nil
	return out
}

type probe struct{ host Host }

func (p *probe) Resize(width, height int) { called(fmt.Sprintf("resize %dx%d", width, height)) }
func (p *probe) Frame()                   {}
func (p *probe) Set(name string, value json.RawMessage) {
	called(fmt.Sprintf("set %s %s", name, value))
}
func (p *probe) Hear(name string, values []json.RawMessage) {
	text, _ := json.Marshal(values)
	called(fmt.Sprintf("hear %s %s", name, text))
	if name == "ping" {
		p.host.Tell("pong", 42)
	}
}

// As the C part host does (runtimes/parts/tests/parts_test.c).
func TestTheHostMakesPlacesSetsAndTellsItsParts(t *testing.T) {
	openWindows = func() (windows, error) { return &fake{}, nil }
	Register("Probe", func(_ Window, width, height int, host Host) Part {
		called(fmt.Sprintf("create %dx%d", width, height))
		return &probe{host: host}
	})
	if !Present() {
		t.Fatal("no kinds")
	}
	var heardMu sync.Mutex
	var heard []string
	if err := Start(func(window uint64, part, tell string, values []any) {
		text, _ := json.Marshal(values)
		heardMu.Lock()
		heard = append(heard, fmt.Sprintf("%d %s %s %s", window, part, tell, text))
		heardMu.Unlock()
	}); err != nil {
		t.Fatal(err)
	}
	defer Stop()

	// What is given before the part is placed reaches it once it is made, in order.
	Set(1, "arena", "speed", json.RawMessage("5"))
	Hear(1, "arena", "restart", []json.RawMessage{json.RawMessage("1")})
	Place(1, "arena", "Probe", 99, 10, 20, 300, 200, true)
	Wait()
	if got := taken(); !reflect.DeepEqual(got, []string{"create 300x200", "set speed 5", "hear restart [1]"}) {
		t.Errorf("made: %q", got)
	}

	// A new size resizes it; a tell comes back with its window and name.
	Place(1, "arena", "Probe", 99, 10, 20, 400, 250, true)
	Hear(1, "arena", "ping", nil)
	Wait()
	Wait()
	if got := taken(); !reflect.DeepEqual(got, []string{"resize 400x250", "hear ping null"}) {
		t.Errorf("then: %q", got)
	}
	heardMu.Lock()
	if !reflect.DeepEqual(heard, []string{"1 arena pong [42]"}) {
		t.Errorf("told: %q", heard)
	}
	heardMu.Unlock()

	// Closing its window ends it; a kind the program does not have makes nothing.
	Close(1)
	Place(2, "map", "Nowhere", 98, 0, 0, 10, 10, true)
	Wait()
	if got := taken(); len(got) != 0 {
		t.Errorf("after: %q", got)
	}

	if Text(json.RawMessage(`"2.50"`)) != "2.50" || Text(json.RawMessage(`2.5`)) != "2.5" || Read[int64](json.RawMessage(`7`)) != 7 {
		t.Error("reading values")
	}
}
