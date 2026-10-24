// Package parts runs the custom parts written in Go (DESIGN 5.19 and 6.5):
// a part is a Go package of the project, which implements the interface
// Clarity writes from its declaration; the program registers each kind of
// part, and this host runs them. It makes a native child window for each
// part where the screen says the part is, calls its Frame about 60 times a
// second, and passes values, hears and tells. Everything runs on one
// goroutine locked to its own thread: a part is made there and lives there.
package parts

import (
	"encoding/json"
	"fmt"
	"os"
	"runtime"
	"sync"
	"time"
	"unsafe"
)

// frame is how long between frames: about 60 a second.
const frame = time.Second / 60

// Window is the native window a part draws in: on Linux an X11 window, on
// the host's own X11 connection; on Windows a Win32 child window of the
// shell's. Handle is the one every system has; Display and Screen are X11's.
type Window struct {
	// The X11 window on Linux, the HWND on Windows.
	Handle  uintptr
	Display unsafe.Pointer
	Screen  int
}

// Part is a part as the host runs it: the program's glue around the part's own type.
type Part interface {
	Resize(width, height int)
	Frame()
	// Set gives a property by name (value, enabled and the declared ones).
	Set(name string, value json.RawMessage)
	// Hear gives a hear by name with its values.
	Hear(name string, values []json.RawMessage)
}

// Make makes a kind of part in its window, at its size, telling through its host.
type Make func(window Window, width, height int, host Host) Part

// Host is where a part tells what happened.
type Host struct {
	window uint64
	part   string
	asks   chan<- ask
}

// Tell tells tell with its values: its handler runs with them.
func (h Host) Tell(tell string, values ...any) {
	if values == nil {
		values = []any{}
	}
	a := ask{kind: tellAsk, window: h.window, part: h.part, name: tell, told: values}
	// In order with what the host was asked; never blocking the host's own goroutine.
	select {
	case h.asks <- a:
	default:
		go func() { h.asks <- a }()
	}
}

// Told is what a part told: its window's number, the part, the tell and its values.
type Told func(window uint64, part, tell string, values []any)

// windows are the native windows of the host: X11 on Linux.
type windows interface {
	child(parent uint64, x, y, width, height int) uint64
	window(child uint64) Window
	move(child uint64, x, y, width, height int)
	show(child uint64, shown bool)
	destroy(child uint64)
	flush()
	close()
}

// openWindows opens the system's windows; the tests give their own.
var openWindows = openPlatform

type askKind int

const (
	placeAsk askKind = iota
	setAsk
	hearAsk
	tellAsk
	closeAsk
	waitAsk
	stopAsk
)

type ask struct {
	kind                askKind
	window              uint64
	part, kind_, name   string
	parent              uint64
	x, y, width, height int
	shown               bool
	value               json.RawMessage
	values              []json.RawMessage
	told                []any
	done                chan struct{}
}

var (
	mu      sync.Mutex
	kinds   = map[string]Make{}
	asks    chan ask
	stopped chan struct{}
)

// Register tells the host about a kind of part by its declared name.
func Register(kind string, make Make) {
	mu.Lock()
	defer mu.Unlock()
	kinds[kind] = make
}

// Present says whether the program has parts.
func Present() bool {
	mu.Lock()
	defer mu.Unlock()
	return len(kinds) > 0
}

// Start starts the host; told is called on its goroutine.
func Start(told Told) error {
	mu.Lock()
	defer mu.Unlock()
	if asks != nil {
		return nil
	}
	ready := make(chan error)
	asks = make(chan ask, 256)
	stopped = make(chan struct{})
	go run(asks, told, ready)
	if err := <-ready; err != nil {
		asks = nil
		return err
	}
	return nil
}

func send(a ask) {
	mu.Lock()
	c := asks
	mu.Unlock()
	if c != nil {
		c <- a
	}
}

// Place says where a part is, in the native window parent, and whether it shows.
func Place(window uint64, part, kind string, parent uint64, x, y, width, height int, shown bool) {
	send(ask{kind: placeAsk, window: window, part: part, kind_: kind, parent: parent, x: x, y: y, width: width, height: height, shown: shown})
}

// Set sets a property of a part.
func Set(window uint64, part, name string, value json.RawMessage) {
	send(ask{kind: setAsk, window: window, part: part, name: name, value: value})
}

// Hear gives a part a hear with its values.
func Hear(window uint64, part, hear string, values []json.RawMessage) {
	send(ask{kind: hearAsk, window: window, part: part, name: hear, values: values})
}

// Close ends every part of a window that closed.
func Close(window uint64) { send(ask{kind: closeAsk, window: window}) }

// Wait waits until everything asked before has been done. For tests.
func Wait() {
	done := make(chan struct{})
	send(ask{kind: waitAsk, done: done})
	select {
	case <-done:
	case <-time.After(5 * time.Second):
	}
}

// Stop ends every part and the host.
func Stop() {
	mu.Lock()
	c, s := asks, stopped
	asks = nil
	mu.Unlock()
	if c != nil {
		c <- ask{kind: stopAsk}
		<-s
	}
}

// Read is a value given to a part as type T; T's empty value when it does not fit.
func Read[T any](value json.RawMessage) T {
	var out T
	if len(value) > 0 {
		_ = json.Unmarshal(value, &out)
	}
	return out
}

// Text is a value given to a part as text in its plain form: a number as written.
func Text(value json.RawMessage) string {
	var text string
	if json.Unmarshal(value, &text) == nil {
		return text
	}
	if string(value) == "null" {
		return ""
	}
	return string(value)
}

// At is the value at i of a hear's values; none past the last.
func At(values []json.RawMessage, i int) json.RawMessage {
	if i < len(values) {
		return values[i]
	}
	return nil
}

type key struct {
	window uint64
	part   string
}

type placed struct {
	made                Part
	child               uint64
	x, y, width, height int
	shown               bool
	waiting             []ask
}

func run(c chan ask, told Told, ready chan<- error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	w, err := openWindows()
	if err != nil {
		ready <- err
		return
	}
	ready <- nil
	tracing := os.Getenv("CLARITY_PARTS_TRACE") != ""
	parts := map[key]*placed{}
	ticker := time.NewTicker(frame)
	defer ticker.Stop()
	for {
		select {
		case a := <-c:
			switch a.kind {
			case stopAsk:
				for _, p := range parts {
					if p.child != 0 {
						w.destroy(p.child)
					}
				}
				w.close()
				close(stopped)
				return
			case waitAsk:
				close(a.done)
			case tellAsk:
				told(a.window, a.part, a.name, a.told)
			case placeAsk:
				if tracing {
					state := "hidden"
					if a.shown {
						state = "shown"
					}
					fmt.Fprintf(os.Stderr, "clarity parts: place %s (%s) of window %d in %#x at %d,%d %dx%d %s\n", a.part, a.kind_, a.window, a.parent, a.x, a.y, a.width, a.height, state)
				}
				mu.Lock()
				make := kinds[a.kind_]
				mu.Unlock()
				if make == nil {
					continue
				}
				k := key{a.window, a.part}
				p := parts[k]
				if p == nil {
					p = &placed{}
					parts[k] = p
				}
				if p.made == nil {
					p.child = w.child(a.parent, a.x, a.y, a.width, a.height)
					w.show(p.child, a.shown)
					p.x, p.y, p.width, p.height, p.shown = a.x, a.y, a.width, a.height, a.shown
					p.made = make(w.window(p.child), max(a.width, 1), max(a.height, 1), Host{window: a.window, part: a.part, asks: c})
					for _, given := range p.waiting {
						give(p.made, given)
					}
					p.waiting = nil
					continue
				}
				resized := a.width != p.width || a.height != p.height
				if resized || a.x != p.x || a.y != p.y {
					w.move(p.child, a.x, a.y, a.width, a.height)
				}
				if a.shown != p.shown {
					w.show(p.child, a.shown)
				}
				p.x, p.y, p.width, p.height, p.shown = a.x, a.y, a.width, a.height, a.shown
				if resized {
					p.made.Resize(max(a.width, 1), max(a.height, 1))
				}
			case setAsk, hearAsk:
				k := key{a.window, a.part}
				p := parts[k]
				if p == nil {
					p = &placed{}
					parts[k] = p
				}
				if p.made == nil {
					p.waiting = append(p.waiting, a)
				} else {
					give(p.made, a)
				}
			case closeAsk:
				for k, p := range parts {
					if k.window == a.window {
						if p.child != 0 {
							w.destroy(p.child)
						}
						delete(parts, k)
					}
				}
			}
		case <-ticker.C:
			for _, p := range parts {
				if p.made != nil && p.shown {
					p.made.Frame()
				}
			}
		}
		w.flush()
	}
}

func give(p Part, a ask) {
	if a.kind == setAsk {
		p.Set(a.name, a.value)
	} else {
		p.Hear(a.name, a.values)
	}
}
