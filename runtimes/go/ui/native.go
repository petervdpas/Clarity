// Package ui runs a generated Go application on Clarity.UI (DESIGN 7.9):
// one library, written in Rust, that draws the windows, lays them out,
// takes the person's input and runs the desktop host, called through its C
// interface (clarity_ui.h) with cgo. The program keeps its window model in
// Go: the library asks it by command (the web wire, DESIGN 7.3), and
// package shell answers. Custom parts written in Go reach the library as a
// table of callbacks (custom.go).
//
// The library comes from the folder the environment variable CLARITY_UI
// names, with include/clarity_ui.h and lib/ holding the library for this
// system. A build gives cgo both, as Clarity's own build does:
//
//	CGO_CFLAGS=-I$CLARITY_UI/include
//	CGO_LDFLAGS="-L$CLARITY_UI/lib -Wl,-rpath,\$ORIGIN -Wl,-rpath,$CLARITY_UI/lib"
//
// so the program finds the shared library beside it, else in that folder
// (on macOS @executable_path in place of $ORIGIN; on Windows the DLL goes
// beside the program).
package ui

/*
#cgo LDFLAGS: -lclarity_ui
#include <stdlib.h>
#include "callbacks.h"
*/
import "C"

import (
	"errors"
	"fmt"
	"unsafe"

	"github.com/petervdpas/Clarity/runtimes/go/shell"
)

// Version is the library's version, the clarity-ui crate's.
func Version() string { return C.GoString(C.clarity_ui_version()) }

// Interface is the version of the C interface the library has; it matches
// the header's CLARITY_UI_INTERFACE when header and library belong together.
func Interface() (library, header uint32) {
	return uint32(C.clarity_ui_interface()), uint32(C.CLARITY_UI_INTERFACE)
}

// failed is the error of a call that did not return CLARITY_OK, with the
// library's message for this thread.
func failed(status C.ClarityStatus, call string) error {
	if status == C.CLARITY_OK {
		return nil
	}
	message := C.clarity_last_error()
	defer C.clarity_string_free(message)
	if message == nil {
		return fmt.Errorf("%s failed (%d)", call, int(status))
	}
	return errors.New(call + ": " + C.GoString(message))
}

// text is Go text as the library takes it: a pointer and a length, with no
// copy (the library copies what it keeps).
func text(s string) (*C.char, C.size_t) {
	if s == "" {
		return nil, 0
	}
	return (*C.char)(unsafe.Pointer(unsafe.StringData(s))), C.size_t(len(s))
}

// bytesOf is a byte slice as the library takes it.
func bytesOf(b []byte) (*C.uint8_t, C.size_t) {
	if len(b) == 0 {
		return nil, 0
	}
	return (*C.uint8_t)(unsafe.Pointer(&b[0])), C.size_t(len(b))
}

// given is text the library gave a callback: a pointer and a length.
func given(p *C.char, n C.size_t) string {
	if p == nil || n == 0 {
		return ""
	}
	return C.GoStringN(p, C.int(n))
}

// The agent's questions go through the library while it runs a command for it.
func init() {
	shell.AgentAsk = func(question, form string) (string, bool) {
		q, f := C.CString(question), C.CString(form)
		defer C.free(unsafe.Pointer(q))
		defer C.free(unsafe.Pointer(f))
		var out *C.char
		if C.clarity_agent_ask(q, C.size_t(len(question)), f, C.size_t(len(form)), &out) != 0 || out == nil {
			return "", false
		}
		return taken(out), true
	}
}

// taken is text the library gave back, freed once read.
func taken(p *C.char) string {
	if p == nil {
		return ""
	}
	defer C.clarity_string_free(p)
	return C.GoString(p)
}

// answerWith sets what a callback answers.
func answerWith(answer *C.ClarityAnswer, out []byte) {
	if answer == nil || out == nil {
		return
	}
	data, n := bytesOf(out)
	C.clarity_answer_set(answer, data, n)
}

// thread is the number of the thread that calls.
func thread() uint64 { return uint64(C.clarity_go_thread()) }
