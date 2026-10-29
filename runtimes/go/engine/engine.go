// Package engine hosts engines (DESIGN 5.11): processing modules compiled
// once to WebAssembly, which every stack hosts alike
// (conformance/engines.json). An engine computes; it imports only the
// services of Clarity it declares (`uses clock, random, log`), so it reaches
// no database, file or network. Each call runs on a fresh instance
// of the module, so an engine keeps no state from one call to the next,
// unless it is declared with `state per app` (one instance serves every
// call) or `state per window` (each open window has an instance of its own,
// dropped when the window closes). Every call runs within the limits of its
// engine: a call that runs too long is
// stopped, and an engine never gets more memory than its limit. An
// application imports this package only when it has an engine.
package engine

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"math/rand/v2"
	"os"
	"slices"
	"sync"
	"time"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

// Error is an engine that could not load, or that failed while it ran.
type Error struct {
	What string
}

func (e *Error) Error() string { return e.What }

// Limits is what one call of an engine may take: how long it may run, and
// how much memory the engine may grow to.
type Limits struct {
	Seconds   uint32 `json:"seconds"`
	Megabytes uint32 `json:"megabytes"`
}

// Services is what the services give: the system's clock, a fresh seed for
// every call and the standard error, unless a test fixes the clock or the
// seed, or keeps the lines of the log.
type Services struct {
	// Clock is the time clock gives, in milliseconds since 1970; nil for the system's clock.
	Clock *int64
	// Seed is the seed random starts every call from; nil for a fresh one each call.
	Seed *uint64
	// Log takes a line of the log, as "engine <Name>: <text>"; nil for the standard error.
	Log func(line string)
	// Language gives the language language gives; nil for the one the screen
	// picked (clarity.Language), "" for the texts as written.
	Language func() string
}

// services are the services an engine may use, each a function of module
// clarity with its parameters and results.
var services = map[string]struct{ params, results []api.ValueType }{
	"clock":    {nil, []api.ValueType{api.ValueTypeI64}},
	"random":   {nil, []api.ValueType{api.ValueTypeI64}},
	"log":      {[]api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, nil},
	"language": {[]api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{api.ValueTypeI32}},
	"progress": {[]api.ValueType{api.ValueTypeI64, api.ValueTypeI64}, nil},
}

// Watch is what the screen sees of a long call and how the person stops it
// (`uses progress`, conformance/progress.json): every report of how far it
// is goes to Progress, and once Stop is closed the call ends, failing with
// "it was stopped".
type Watch struct {
	// Progress is told each report, done of total; nil drops them.
	Progress func(done, total int64)
	// Stop is closed to stop the call; nil never stops it.
	Stop <-chan struct{}
}

// errStopped is the cause of a call's context when the person stopped it.
var errStopped = errors.New("it was stopped")

// State is whether an engine keeps an instance from call to call.
type State int

const (
	// NoState runs every call on a fresh instance.
	NoState State = iota
	// PerApp keeps one instance for every call of the application (`state per app`).
	PerApp
	// PerWindow keeps an instance for each open window (`state per window`).
	PerWindow
)

// Declaration is an engine as its declaration gives it: its functions, the
// services it uses, the limits of a call, whether it keeps state and
// whether it has `on open`.
type Declaration struct {
	Functions []string
	Uses      []string
	Limits    Limits
	State     State
	// Opens runs the engine's clarity_open on every new instance (`on open`).
	Opens bool
}

// Engine is an engine, its module checked and compiled once.
type Engine struct {
	name     string
	runtime  wazero.Runtime
	module   wazero.CompiledModule
	limits   Limits
	state    State
	opens    bool
	services Services
	// kept holds the instances an engine with state keeps between calls, by
	// window (one, at 0, for PerApp).
	mu   sync.Mutex
	kept map[uint64]*live
}

// live is an instance with what a call needs of it.
type live struct {
	module      api.Module
	memory      api.Memory
	alloc, free api.Function
}

// call is the state of the services during one call.
type call struct {
	progress func(done, total int64)
	name     string
	clock    *int64
	random   uint64
	log      func(string)
	language func() string
}

type callKey struct{}

// splitmix is the next number of splitmix64, the random sequence every host gives alike.
func splitmix(state *uint64) int64 {
	*state += 0x9E3779B97F4A7C15
	z := *state
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	return int64(z ^ (z >> 31))
}

// serve adds module clarity, the services, to runtime.
func serve(ctx context.Context, runtime wazero.Runtime) error {
	state := func(ctx context.Context) *call { return ctx.Value(callKey{}).(*call) }
	_, err := runtime.NewHostModuleBuilder("clarity").
		NewFunctionBuilder().WithFunc(func(ctx context.Context) int64 {
		if c := state(ctx); c.clock != nil {
			return *c.clock
		}
		return time.Now().UnixMilli()
	}).Export("clock").
		NewFunctionBuilder().WithFunc(func(ctx context.Context) int64 { return splitmix(&state(ctx).random) }).Export("random").
		NewFunctionBuilder().WithFunc(func(ctx context.Context, m api.Module, at, length uint32) {
		text, ok := m.Memory().Read(at, length)
		if !ok {
			panic("the text to log is outside the engine's memory")
		}
		c := state(ctx)
		c.log("engine " + c.name + ": " + string(text))
	}).Export("log").
		// The language as it is at the call, written into the engine's room.
		NewFunctionBuilder().WithFunc(func(ctx context.Context, m api.Module, at, room uint32) uint32 {
		language := []byte(state(ctx).language())
		written := language
		if uint32(len(written)) > room {
			written = written[:room]
		}
		if !m.Memory().Write(at, written) {
			panic("the room for the language is outside the engine's memory")
		}
		return uint32(len(language))
	}).Export("language").
		NewFunctionBuilder().WithFunc(func(ctx context.Context, done, total int64) {
		if p := state(ctx).progress; p != nil {
			p(done, total)
		}
	}).Export("progress").
		Instantiate(ctx)
	return err
}

// Load loads bytes as engine name with the functions it is declared with,
// the services it uses and the limits of its calls; it refuses a module
// that is no engine, imports anything but those services, or lacks one of
// the functions.
func Load(name string, bytes []byte, declaration Declaration) (*Engine, error) {
	functions, uses, limits := declaration.Functions, declaration.Uses, declaration.Limits
	refused := func(why string) error { return &Error{What: "engine " + name + " " + why} }
	if len(bytes) < 4 || string(bytes[:4]) != "\x00asm" {
		return nil, refused("is no WebAssembly module")
	}
	ctx := context.Background()
	// A call stops when its context does; memory grows in pages of 64 KiB.
	config := wazero.NewRuntimeConfig().WithCloseOnContextDone(true).WithMemoryLimitPages(limits.Megabytes * 16)
	runtime := wazero.NewRuntimeWithConfig(ctx, config)
	module, err := runtime.CompileModule(ctx, bytes)
	if err != nil {
		return nil, refused("is no WebAssembly module: " + err.Error())
	}
	for _, f := range module.ImportedFunctions() {
		from, what, _ := f.Import()
		if from != "clarity" || !slices.Contains(uses, what) {
			return nil, refused(fmt.Sprintf("imports %s.%s; an engine imports only the services it uses", from, what))
		}
		if s, ok := services[what]; !ok || !slices.Equal(f.ParamTypes(), s.params) || !slices.Equal(f.ResultTypes(), s.results) {
			return nil, refused(fmt.Sprintf("imports clarity.%s with another type than Clarity gives it", what))
		}
	}
	for _, m := range module.ImportedMemories() {
		from, what, _ := m.Import()
		return nil, refused(fmt.Sprintf("imports %s.%s; an engine imports only the services it uses", from, what))
	}
	if err := serve(ctx, runtime); err != nil {
		return nil, refused("cannot be given its services: " + err.Error())
	}
	exported := module.ExportedFunctions()
	for _, needed := range []string{"clarity_alloc", "clarity_free"} {
		if _, ok := exported[needed]; !ok {
			return nil, refused("lacks " + needed + " of the engine interface")
		}
	}
	if _, ok := module.ExportedMemories()["memory"]; !ok {
		return nil, refused("lacks the memory of the engine interface")
	}
	for _, f := range functions {
		if _, ok := exported[f]; !ok {
			return nil, refused("has no function " + f)
		}
	}
	if _, ok := exported["clarity_open"]; declaration.Opens && !ok {
		return nil, refused("lacks clarity_open, which its on open needs")
	}
	e := &Engine{name: name, runtime: runtime, module: module, limits: limits, state: declaration.State, opens: declaration.Opens, kept: map[uint64]*live{}}
	if e.state == PerWindow {
		// A window that closes takes its instance along.
		clarity.OnWindowClosed(e.CloseWindow)
	}
	return e, nil
}

// Serve gives the engine services in place of the system's.
func (e *Engine) Serve(s Services) {
	e.services = s
}

// Call calls function with args: its value as JSON gives it, the engine's
// refusal in its own words, or what went wrong running it.
func (e *Engine) Call(function string, args ...any) (any, error) {
	return e.CallIn(0, function, args...)
}

// CloseWindow drops the instance window had, when it closes.
func (e *Engine) CloseWindow(window uint64) {
	if e.state != PerWindow {
		return
	}
	e.mu.Lock()
	defer e.mu.Unlock()
	if l, ok := e.kept[window]; ok {
		l.module.Close(context.Background())
		delete(e.kept, window)
	}
}

// CallIn calls function in open window window: for an engine with state
// per window, on that window's own instance.
func (e *Engine) CallIn(window uint64, function string, args ...any) (any, error) {
	return e.CallWatched(window, Watch{}, function, args...)
}

// CallWatched calls function in open window window as CallIn does, telling
// watch how far it is and stopping when it says so.
func (e *Engine) CallWatched(window uint64, watch Watch, function string, args ...any) (any, error) {
	state := &call{name: e.name, clock: e.services.Clock, random: rand.Uint64(), log: e.services.Log, progress: watch.Progress}
	if e.services.Seed != nil {
		state.random = *e.services.Seed
	}
	if state.log == nil {
		state.log = func(line string) { fmt.Fprintln(os.Stderr, line) }
	}
	state.language = e.services.Language
	if state.language == nil {
		state.language = clarity.Language
	}
	timed, cancel := context.WithTimeout(context.WithValue(context.Background(), callKey{}, state), time.Duration(e.limits.Seconds)*time.Second)
	defer cancel()
	// Stop ends the call as its time limit does, with a cause of its own.
	ctx, stop := context.WithCancelCause(timed)
	defer stop(nil)
	if watch.Stop != nil {
		select {
		case <-watch.Stop:
			return nil, &Error{What: fmt.Sprintf("engine %s failed in %s: %s", e.name, function, errStopped)}
		default:
		}
		go func() {
			select {
			case <-watch.Stop:
				stop(errStopped)
			case <-ctx.Done():
			}
		}()
	}
	failed := func(why string) error {
		if errors.Is(context.Cause(ctx), errStopped) {
			why = errStopped.Error()
		} else if errors.Is(ctx.Err(), context.DeadlineExceeded) {
			unit := "seconds"
			if e.limits.Seconds == 1 {
				unit = "second"
			}
			why = fmt.Sprintf("it ran longer than %d %s", e.limits.Seconds, unit)
		}
		return &Error{What: fmt.Sprintf("engine %s failed in %s: %s", e.name, function, why)}
	}
	// An engine with state keeps its instance, unless a call fails or runs
	// past its limit: then its memory may be half changed, so the next call
	// starts a new one.
	keeps := e.state != NoState
	key := window
	if e.state == PerApp {
		key = 0
	}
	if keeps {
		e.mu.Lock()
		defer e.mu.Unlock()
	}
	l := e.kept[key]
	delete(e.kept, key)
	if l == nil {
		var err error
		if l, err = e.instance(ctx); err != nil {
			return nil, failed(err.Error())
		}
	}
	output, err := invoke(ctx, l, function, args)
	if err != nil {
		l.module.Close(context.Background())
		return nil, failed(err.Error())
	}
	// The engine answered, even with a refusal, so its memory is whole.
	if keeps {
		e.kept[key] = l
	} else {
		l.module.Close(context.Background())
	}
	return answer(output, failed)
}

// instance is a new instance of the module, opened when the engine has `on open`.
func (e *Engine) instance(ctx context.Context) (*live, error) {
	module, err := e.runtime.InstantiateModule(ctx, e.module, wazero.NewModuleConfig().WithName(""))
	if err != nil {
		return nil, err
	}
	l := &live{module: module, memory: module.Memory(), alloc: module.ExportedFunction("clarity_alloc"), free: module.ExportedFunction("clarity_free")}
	if l.memory == nil || l.alloc == nil || l.free == nil {
		module.Close(context.Background())
		return nil, errors.New("the engine interface is incomplete")
	}
	if e.opens {
		output, err := invoke(ctx, l, "clarity_open", nil)
		if err != nil {
			module.Close(context.Background())
			return nil, err
		}
		if _, err := answer(output, func(why string) error { return errors.New(why) }); err != nil {
			module.Close(context.Background())
			var refused *clarity.RefusedError
			if errors.As(err, &refused) {
				return nil, errors.New("its on open refused: " + refused.Error())
			}
			return nil, errors.New("its on open gave no answer")
		}
	}
	return l, nil
}

// invoke writes the arguments into the instance, runs function and reads its answer.
func invoke(ctx context.Context, l *live, function string, args []any) ([]byte, error) {
	run := l.module.ExportedFunction(function)
	if run == nil {
		return nil, errors.New("there is no function " + function)
	}
	if args == nil {
		args = []any{}
	}
	input, err := json.Marshal(args)
	if err != nil {
		return nil, err
	}
	room, err := l.alloc.Call(ctx, uint64(len(input)))
	if err != nil {
		return nil, err
	}
	at := uint32(room[0])
	if !l.memory.Write(at, input) {
		return nil, errors.New("the arguments do not fit its memory")
	}
	result, err := run.Call(ctx, uint64(at), uint64(len(input)))
	if err != nil {
		return nil, err
	}
	place, size := uint32(result[0]>>32), uint32(result[0])
	output, ok := l.memory.Read(place, size)
	if !ok {
		return nil, errors.New("its answer is outside its memory")
	}
	output = append([]byte(nil), output...)
	if _, err := l.free.Call(ctx, uint64(place), uint64(size)); err != nil {
		return nil, err
	}
	return output, nil
}

// answer is the value in an engine's answer, or its refusal in its own words.
func answer(output []byte, failed func(string) error) (any, error) {
	// By key, so a value that is null is a value too.
	var answer map[string]json.RawMessage
	if err := json.Unmarshal(output, &answer); err != nil {
		return nil, failed("its answer is no JSON: " + err.Error())
	}
	if raw, ok := answer["error"]; ok {
		var why string
		if err := json.Unmarshal(raw, &why); err != nil {
			return nil, failed("its error is no text")
		}
		// An engine refuses by the key of its text (its says), or in its own words.
		return nil, clarity.Refuse(clarity.KeyText(why))
	}
	raw, ok := answer["value"]
	if !ok {
		return nil, failed("its answer has no value")
	}
	var value any
	if err := json.Unmarshal(raw, &value); err != nil {
		return nil, failed(err.Error())
	}
	return value, nil
}

// As gives what an engine gave as a value of code of its declared type.
func As[T any](value any, err error) (T, error) {
	var out T
	if err != nil {
		return out, err
	}
	text, err := json.Marshal(value)
	if err == nil {
		err = json.Unmarshal(text, &out)
	}
	if err != nil {
		return out, &Error{What: "an engine gave a value of another type: " + err.Error()}
	}
	return out, nil
}
