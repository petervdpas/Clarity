// Package engine hosts engines (DESIGN 5.11): processing modules compiled
// once to WebAssembly, which every stack hosts alike
// (conformance/engines.json). An engine computes; it imports only the
// services of Clarity it declares (`uses clock, random, log`), so it reaches
// no database, file or network. Each call runs on a fresh instance
// of the module, so an engine keeps no state from one call to the next, and
// runs within the limits of its engine: a call that runs too long is
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
}

// services are the services an engine may use, each a function of module
// clarity with its parameters and results.
var services = map[string]struct{ params, results []api.ValueType }{
	"clock":  {nil, []api.ValueType{api.ValueTypeI64}},
	"random": {nil, []api.ValueType{api.ValueTypeI64}},
	"log":    {[]api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, nil},
}

// Engine is an engine, its module checked and compiled once.
type Engine struct {
	name     string
	runtime  wazero.Runtime
	module   wazero.CompiledModule
	limits   Limits
	services Services
}

// call is the state of the services during one call.
type call struct {
	name   string
	clock  *int64
	random uint64
	log    func(string)
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
		Instantiate(ctx)
	return err
}

// Load loads bytes as engine name with the functions it is declared with,
// the services it uses and the limits of its calls; it refuses a module
// that is no engine, imports anything but those services, or lacks one of
// the functions.
func Load(name string, bytes []byte, functions []string, uses []string, limits Limits) (*Engine, error) {
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
	return &Engine{name: name, runtime: runtime, module: module, limits: limits}, nil
}

// Serve gives the engine services in place of the system's.
func (e *Engine) Serve(s Services) {
	e.services = s
}

// Call calls function with args: its value as JSON gives it, the engine's
// refusal in its own words, or what went wrong running it.
func (e *Engine) Call(function string, args ...any) (any, error) {
	state := &call{name: e.name, clock: e.services.Clock, random: rand.Uint64(), log: e.services.Log}
	if e.services.Seed != nil {
		state.random = *e.services.Seed
	}
	if state.log == nil {
		state.log = func(line string) { fmt.Fprintln(os.Stderr, line) }
	}
	ctx, cancel := context.WithTimeout(context.WithValue(context.Background(), callKey{}, state), time.Duration(e.limits.Seconds)*time.Second)
	defer cancel()
	failed := func(why string) error {
		if errors.Is(ctx.Err(), context.DeadlineExceeded) {
			unit := "seconds"
			if e.limits.Seconds == 1 {
				unit = "second"
			}
			why = fmt.Sprintf("it ran longer than %d %s", e.limits.Seconds, unit)
		}
		return &Error{What: fmt.Sprintf("engine %s failed in %s: %s", e.name, function, why)}
	}
	instance, err := e.runtime.InstantiateModule(ctx, e.module, wazero.NewModuleConfig().WithName(""))
	if err != nil {
		return nil, failed(err.Error())
	}
	defer instance.Close(context.Background())
	memory := instance.Memory()
	alloc, free, run := instance.ExportedFunction("clarity_alloc"), instance.ExportedFunction("clarity_free"), instance.ExportedFunction(function)
	if memory == nil || alloc == nil || free == nil || run == nil {
		return nil, failed("the engine interface is incomplete")
	}
	if args == nil {
		args = []any{}
	}
	input, err := json.Marshal(args)
	if err != nil {
		return nil, failed(err.Error())
	}
	room, err := alloc.Call(ctx, uint64(len(input)))
	if err != nil {
		return nil, failed(err.Error())
	}
	at := uint32(room[0])
	if !memory.Write(at, input) {
		return nil, failed("the arguments do not fit its memory")
	}
	result, err := run.Call(ctx, uint64(at), uint64(len(input)))
	if err != nil {
		return nil, failed(err.Error())
	}
	place, size := uint32(result[0]>>32), uint32(result[0])
	output, ok := memory.Read(place, size)
	if !ok {
		return nil, failed("its answer is outside its memory")
	}
	output = append([]byte(nil), output...)
	if _, err := free.Call(ctx, uint64(place), uint64(size)); err != nil {
		return nil, failed(err.Error())
	}
	var answer struct {
		Value *json.RawMessage `json:"value"`
		Error *string          `json:"error"`
	}
	if err := json.Unmarshal(output, &answer); err != nil {
		return nil, failed("its answer is no JSON: " + err.Error())
	}
	if answer.Error != nil {
		return nil, clarity.Refuse(clarity.PlainText(*answer.Error))
	}
	if answer.Value == nil {
		return nil, failed("its answer has no value")
	}
	var value any
	if err := json.Unmarshal(*answer.Value, &value); err != nil {
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
