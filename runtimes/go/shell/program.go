// The program over the wire (DESIGN 7.3 and 7.9): what Clarity.UI asks a
// generated program, on the desktop through package ui and in a browser
// over HTTP (serve.go), answered with the service and the program's own
// facts, engines and long calls.

package shell

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
)

// Engines are the engines beside the screen of a program (DESIGN 5.12),
// each instance by its number; package engine's Screens is one.
type Engines interface {
	// EngineOpen opens a new instance of engine name: its number; found is
	// false when the program has no engine of that name.
	EngineOpen(name string) (id uint64, found bool, err error)
	// EngineHears is whether instance id takes the pointer and keys.
	EngineHears(id uint64) (pointer, keys bool)
	// EngineFrame is one frame of instance id: the input since the last as
	// the library sends it, then ms; its shapes and tells as JSON.
	EngineFrame(id uint64, ms float64, input json.RawMessage) (json.RawMessage, error)
	EngineClose(id uint64)
}

// ProgramOptions are what a generated program answers Clarity.UI with:
// the service's options, which answer the windows' commands, and the
// program's own facts, long calls and engines.
type ProgramOptions struct {
	Options Options
	// Facts are what the program knows of its fields and types, as JSON
	// (the runtime's fields facts), which the parts read beside the values; nil for none.
	Facts []byte
	// Watches is whether an engine says how far a long call is: the library
	// then runs handlers off its own thread and shows the bar with Stop.
	Watches bool
	// Engines are the engines beside the screen; nil for none.
	Engines Engines
}

// watch is a long call's watch: how far it is, and its Stop.
type watch struct {
	id         uint64
	done, all  int64
	told       bool
	stop       chan struct{}
	stopClosed bool
}

// Program answers the library's commands (clarity_ui.h) for a generated
// application: the windows' commands go to the service, and the library's
// own (started, watches, facts, engines) are answered here.
type Program struct {
	service *Service
	options ProgramOptions

	mu sync.Mutex
	// watching is the watch of each thread a handler runs on, by thread.
	watching map[uint64]*watch
	// early are the watches stopped before they started; ended is the
	// highest watch that ended, so a Stop of one that ended stops nothing.
	early map[uint64]bool
	ended uint64

	// pages are the browser tabs a server answers, by the id each gives
	// its requests (serve.go); one unasked for quiet is forgotten.
	pagesMu sync.Mutex
	pages   map[string]*servedPage
	quiet   time.Duration
}

// NewProgram is the program of options, with its service.
func NewProgram(options ProgramOptions) *Program {
	return &Program{service: New(options.Options), options: options, watching: map[uint64]*watch{}, early: map[uint64]bool{}, pages: map[string]*servedPage{}, quiet: Quiet}
}

// Service is the program's service, which answers the windows' commands.
func (p *Program) Service() *Service { return p.service }

// forwarded are the commands the service answers as the wire has them.
var forwarded = map[string]bool{
	"start_app": true, "connect": true, "open_window": true, "close_window": true, "return_to": true,
	"dispatch_event": true, "edit_event": true, "undo_edit": true, "reorder_event": true, "item_event": true,
	"range_event": true, "canvas_event": true, "step_event": true, "view_event": true, "answer_event": true,
	"ref_options": true, "field_values": true, "find_options": true, "read_settings": true, "keep_settings": true,
	"set_language": true, "signin": true, "signout": true,
}

// value is the answer {"value": v}.
func value(v any) []byte {
	out, err := json.Marshal(Answer{Value: v})
	if err != nil {
		return refused(err.Error())
	}
	return out
}

// refused is the answer {"error": why}.
func refused(why string) []byte {
	out, _ := json.Marshal(map[string]string{"error": why})
	return out
}

func wired(a Answer) []byte {
	out, err := json.Marshal(a)
	if err != nil {
		return refused(err.Error())
	}
	return out
}

// numbered is the id a watch or engine command names.
func numbered(args []byte) uint64 {
	var given struct {
		ID uint64 `json:"id"`
	}
	_ = json.Unmarshal(args, &given)
	return given.ID
}

// Answer answers command with its arguments as JSON, asked from thread:
// {"value": ...} or {"error": ...}; nil takes the program's default (the
// library's own for a command it does not know).
func (p *Program) Answer(command string, args []byte, thread uint64) []byte {
	return p.answerIn(OnTheDesktop(context.Background()), command, args, thread)
}

// answerIn is Answer asked in ctx: served, ctx names the page that asks.
func (p *Program) answerIn(ctx context.Context, command string, args []byte, thread uint64) (out []byte) {
	// Fail safe: a command that panics is an error the screen shows, and the program goes on.
	defer func() {
		if r := recover(); r != nil {
			fmt.Fprintf(os.Stderr, "%s failed: %v\n", command, r)
			out = refused(fmt.Sprint(r))
		}
	}()
	if len(args) == 0 {
		args = []byte("{}")
	}
	switch command {
	case "started":
		return p.started(ctx)
	case "splash_status":
		return value(clarity.SplashStatus())
	case "watches", "start_watch", "watch_progress", "stop_watch", "end_watch":
		return p.watch(command, numbered(args), thread)
	case "facts":
		if p.options.Facts == nil {
			return nil
		}
		return value(json.RawMessage(p.options.Facts))
	case "engine_open", "engine_hears", "engine_frame", "engine_close":
		// Served while people sign in, an engine runs only for a call with a session.
		if !p.service.signedInOrOpen(ctx) {
			return wired(failed(clarity.NotAllowed()))
		}
		return p.engine(command, args)
	case "field_bytes":
		// A record's file or picture, as base64.
		return wired(p.service.Invoke(ctx, "file_data", args))
	case "store_file":
		// A store's file as base64; the web wire gives it as a data URL.
		answered := p.service.Invoke(ctx, "store_file", args)
		if url, ok := answered.Value.(string); ok {
			if _, data, found := strings.Cut(url, ","); found {
				answered.Value = data
			}
		}
		return wired(answered)
	case "dispatch_event", "answer_event":
		if w := p.watchOf(thread); w != nil {
			return wired(p.service.InvokeWatched(ctx, command, args, clarity.Watch{Stop: w.stop, Progress: p.progress(w)}))
		}
	}
	if !forwarded[command] {
		return nil
	}
	return wired(p.service.Invoke(ctx, command, args))
}

// started is the window the application opens with, and why the databases did not open.
func (p *Program) started(ctx context.Context) []byte {
	answered := p.service.Invoke(ctx, "start", nil)
	if answered.Error != nil {
		return wired(answered)
	}
	start, _ := answered.Value.(map[string]any)
	started := map[string]any{"main": start["main"], "unconnected": start["unconnected"]}
	if signin, ok := start["signin"]; ok {
		started["signin"] = signin
	}
	return value(started)
}

// -- Long calls (clarity_ui.h): a handler runs on the library's worker
// thread between start_watch and end_watch; how far it is and Stop come
// from the screen's thread meanwhile.

func (p *Program) watchOf(thread uint64) *watch {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.watching[thread]
}

func (p *Program) progress(w *watch) func(done, total int64) {
	return func(done, total int64) {
		p.mu.Lock()
		defer p.mu.Unlock()
		w.done, w.all, w.told = done, total, true
	}
}

func (p *Program) byID(id uint64) *watch {
	for _, w := range p.watching {
		if w.id == id {
			return w
		}
	}
	return nil
}

func (w *watch) halt() {
	if !w.stopClosed {
		close(w.stop)
		w.stopClosed = true
	}
}

func (p *Program) watch(command string, id, thread uint64) []byte {
	if command == "watches" {
		return value(p.options.Watches)
	}
	p.mu.Lock()
	defer p.mu.Unlock()
	switch command {
	case "start_watch":
		w := &watch{id: id, stop: make(chan struct{})}
		p.watching[thread] = w
		// A Stop before the handler's own watch started holds once it starts.
		if p.early[id] {
			w.halt()
			delete(p.early, id)
		}
	case "watch_progress":
		if w := p.byID(id); w != nil && w.told {
			return value([]int64{w.done, w.all})
		}
	case "stop_watch":
		if id <= p.ended && p.ended > 0 {
			break
		}
		if w := p.byID(id); w != nil {
			w.halt()
		} else {
			p.early[id] = true
		}
	case "end_watch":
		// The handler ended: its watch is forgotten; this may come twice.
		for at, w := range p.watching {
			if w.id == id {
				delete(p.watching, at)
			}
		}
		p.ended = max(p.ended, id)
		delete(p.early, id)
	}
	return value(nil)
}

// -- Engines beside the screen

func (p *Program) engine(command string, args []byte) []byte {
	engines := p.options.Engines
	if engines == nil {
		return nil
	}
	switch command {
	case "engine_open":
		var given struct {
			Name string `json:"name"`
		}
		_ = json.Unmarshal(args, &given)
		id, found, err := engines.EngineOpen(given.Name)
		if err != nil {
			return refused(err.Error())
		}
		if !found {
			return value(nil)
		}
		return value(id)
	case "engine_hears":
		pointer, keys := engines.EngineHears(numbered(args))
		return value(map[string]bool{"pointer": pointer, "keys": keys})
	case "engine_frame":
		var given struct {
			ID    uint64          `json:"id"`
			Ms    float64         `json:"ms"`
			Input json.RawMessage `json:"input"`
		}
		if err := json.Unmarshal(args, &given); err != nil {
			return refused(err.Error())
		}
		drawn, err := engines.EngineFrame(given.ID, given.Ms, given.Input)
		if err != nil {
			return refused(err.Error())
		}
		return value(drawn)
	default:
		engines.EngineClose(numbered(args))
		return value(nil)
	}
}
