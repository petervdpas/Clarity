// Package engine hosts engines (DESIGN 5.11): processing modules compiled
// once to WebAssembly, which every stack hosts alike
// (conformance/engines.json). An engine computes; it imports nothing, so it
// reaches no database, file or network. Each call runs on a fresh instance
// of the module, so an engine keeps no state from one call to the next. An
// application imports this package only when it has an engine.
package engine

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
	"github.com/tetratelabs/wazero"
)

// Error is an engine that could not load, or that failed while it ran.
type Error struct {
	What string
}

func (e *Error) Error() string { return e.What }

// Engine is an engine, its module checked and compiled once.
type Engine struct {
	name    string
	runtime wazero.Runtime
	module  wazero.CompiledModule
}

// Load loads bytes as engine name with the functions it is declared with;
// it refuses a module that is no engine, imports anything, or lacks one of
// them.
func Load(name string, bytes []byte, functions []string) (*Engine, error) {
	refused := func(why string) error { return &Error{What: "engine " + name + " " + why} }
	if len(bytes) < 4 || string(bytes[:4]) != "\x00asm" {
		return nil, refused("is no WebAssembly module")
	}
	ctx := context.Background()
	runtime := wazero.NewRuntime(ctx)
	module, err := runtime.CompileModule(ctx, bytes)
	if err != nil {
		return nil, refused("is no WebAssembly module: " + err.Error())
	}
	for _, f := range module.ImportedFunctions() {
		from, what, _ := f.Import()
		return nil, refused(fmt.Sprintf("imports %s.%s; an engine imports nothing", from, what))
	}
	for _, m := range module.ImportedMemories() {
		from, what, _ := m.Import()
		return nil, refused(fmt.Sprintf("imports %s.%s; an engine imports nothing", from, what))
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
	return &Engine{name: name, runtime: runtime, module: module}, nil
}

// Call calls function with args: its value as JSON gives it, the engine's
// refusal in its own words, or what went wrong running it.
func (e *Engine) Call(function string, args ...any) (any, error) {
	failed := func(why string) error {
		return &Error{What: fmt.Sprintf("engine %s failed in %s: %s", e.name, function, why)}
	}
	ctx := context.Background()
	instance, err := e.runtime.InstantiateModule(ctx, e.module, wazero.NewModuleConfig().WithName(""))
	if err != nil {
		return nil, failed(err.Error())
	}
	defer instance.Close(ctx)
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
