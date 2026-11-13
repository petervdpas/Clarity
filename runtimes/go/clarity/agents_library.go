//go:build clarityagents

package clarity

// Clarity.AppMcp through cgo (docs/ledger/app-mcp.md): agentsServe serves
// the tool list over standard input and output and runs each call with
// run. Built in the runtime only with the tag clarityagents, for its own
// test; a program that offers commands to agents carries a copy.

/*
#include <stdlib.h>
#include "clarity_app_mcp.h"

int32_t agents_serve(const char* tools, size_t tools_len, const char* options, size_t options_len);
int32_t agents_command_line(ClarityAppMcpCall* call, uint8_t** out, size_t* out_len);
int32_t agents_ask(ClarityAppMcpCall* call, const char* question, size_t question_len, const char* form, size_t form_len, uint8_t** out, size_t* out_len);
void agents_answer(ClarityAppMcpCall* call, const char* text, size_t text_len, int refused);
void agents_free(uint8_t* at, size_t len);
int32_t agents_problem(char* into, size_t size);
int32_t agents_forward(const char* file, size_t file_len);
*/
import "C"

import (
	"encoding/json"
	"errors"
	"unsafe"
)

// agentCall is a call being run; valid only while run runs.
type agentCall struct{ call *C.ClarityAppMcpCall }

// agentsRun runs each call of the one service a program serves.
var agentsRun func(agentCall) (string, bool)

// agentsServe serves tools with options (JSON) until standard input ends.
func agentsServe(tools, options string, run func(agentCall) (string, bool)) error {
	agentsRun = run
	t, o := C.CString(tools), C.CString(options)
	defer C.free(unsafe.Pointer(t))
	defer C.free(unsafe.Pointer(o))
	if C.agents_serve(t, C.size_t(len(tools)), o, C.size_t(len(options))) != C.CLARITY_APP_MCP_OK {
		var why [1024]C.char
		C.agents_problem(&why[0], C.size_t(len(why)))
		return errors.New(C.GoString(&why[0]))
	}
	return nil
}

// agentsForward forwards standard input and output to the running app that
// running.json at file names, until either ends; false when none answers.
func agentsForward(file string) bool {
	f := C.CString(file)
	defer C.free(unsafe.Pointer(f))
	return C.agents_forward(f, C.size_t(len(file))) == 1
}

// taken is what the library gave in out, given back to it.
func taken(out *C.uint8_t, length C.size_t) []byte {
	made := C.GoBytes(unsafe.Pointer(out), C.int(length))
	C.agents_free(out, length)
	return made
}

// CommandLine is the call as the command line the program reads.
func (c agentCall) CommandLine() []string {
	var out *C.uint8_t
	var length C.size_t
	if C.agents_command_line(c.call, &out, &length) != C.CLARITY_APP_MCP_OK {
		return nil
	}
	var line []string
	_ = json.Unmarshal(taken(out, length), &line)
	return line
}

// Ask asks the agent question of the kind form; false when there is no answer.
func (c agentCall) Ask(question, form string) (string, bool) {
	q, f := C.CString(question), C.CString(form)
	defer C.free(unsafe.Pointer(q))
	defer C.free(unsafe.Pointer(f))
	var out *C.uint8_t
	var length C.size_t
	if C.agents_ask(c.call, q, C.size_t(len(question)), f, C.size_t(len(form)), &out, &length) != C.CLARITY_APP_MCP_OK {
		return "", false
	}
	return string(taken(out, length)), true
}

//export clarityAgentRun
func clarityAgentRun(call *C.ClarityAppMcpCall) {
	text, refused := agentsRun(agentCall{call})
	t := C.CString(text)
	defer C.free(unsafe.Pointer(t))
	r := C.int(0)
	if refused {
		r = 1
	}
	C.agents_answer(call, t, C.size_t(len(text)), r)
}
