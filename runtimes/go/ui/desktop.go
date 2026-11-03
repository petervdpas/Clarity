package ui

/*
#include <stdlib.h>
#include "callbacks.h"
*/
import "C"

import (
	"runtime"
	"runtime/cgo"
)

// The desktop's windows belong to the main thread on macOS: main runs there,
// locked from the start.
func init() { runtime.LockOSThread() }

// native is the program as the library holds it: a ClarityProgram whose
// callbacks answer through p, held by a handle until the library lets go.
func native(p *Program) (*C.ClarityProgram, error) {
	handle := cgo.NewHandle(p)
	var out *C.ClarityProgram
	if err := failed(C.clarity_go_program_new(C.uintptr_t(handle), &out), "clarity_program_new"); err != nil {
		handle.Delete()
		return nil, err
	}
	return out, nil
}

// Run shows the program's windows in desktop windows until they closed,
// on the main thread; with the environment variable CLARITY_HEADLESS set it
// is driven from standard input instead (DESIGN 1.6). The exit code: 0 or 1.
func Run(desktop Desktop) (int, error) {
	files, err := FilesOf(desktop.Files, desktop.Options.Model, desktop.Options.Title, float64(desktop.Options.Width), float64(desktop.Options.Height))
	if err != nil {
		return 1, err
	}
	defer files.Close()
	program, err := native(NewProgram(desktop))
	if err != nil {
		return 1, err
	}
	defer C.clarity_program_free(program)
	var code C.int32_t
	if err := failed(C.clarity_desktop_run(program, files.c, &code), "clarity_desktop_run"); err != nil {
		return 1, err
	}
	return int(code), nil
}

// Wake has the running desktop read every window's values again and draw,
// from any goroutine; nothing while no desktop runs.
func Wake() { C.clarity_desktop_wake() }

// Driven is a program started as the desktop does, on a screen without a
// window, and driven step by step (DESIGN 1.6): how tests use a window
// model as a person would. Use it on one goroutine, locked to its thread.
type Driven struct {
	c       *C.ClarityDriven
	program *C.ClarityProgram
	files   *Files
}

// StartDriven starts the program on a screen without a window.
func StartDriven(p *Program, files *Files) (*Driven, error) {
	program, err := native(p)
	if err != nil {
		return nil, err
	}
	var c *C.ClarityDriven
	if err := failed(C.clarity_driven_start(program, files.c, &c), "clarity_driven_start"); err != nil {
		C.clarity_program_free(program)
		return nil, err
	}
	return &Driven{c: c, program: program, files: files}, nil
}

// Step does one step (JSON: {"do":"click","part":"save"}) and gives the
// library's answer as JSON; a step that could not be done fails with that
// answer too.
func (d *Driven) Step(step string) (string, error) {
	s, sl := text(step)
	var out *C.char
	status := C.clarity_driven_step(d.c, s, sl, &out)
	answer := taken(out)
	if status != C.CLARITY_OK {
		err := failed(status, "clarity_driven_step")
		return answer, err
	}
	return answer, nil
}

// Close ends the run and lets the program and its files go.
func (d *Driven) Close() {
	if d.c != nil {
		C.clarity_driven_free(d.c)
		C.clarity_program_free(d.program)
		d.files.Close()
		d.c = nil
	}
}
