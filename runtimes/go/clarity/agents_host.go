//go:build clarityagents

package clarity

// The C side of Clarity.AppMcp's host (docs/ledger/app-mcp.md): the
// program's callbacks into Go, and the calls into the library. Built in the
// runtime only with the tag clarityagents, for its own test; a program that
// offers commands to agents carries a copy.

/*
#cgo LDFLAGS: -lclarity_app_mcp
#include <stdlib.h>
#include "clarity_app_mcp.h"

// The Go function agents_library.go exports.
extern void clarityAgentRun(ClarityAppMcpCall* call);

static void agents_run(void* user, ClarityAppMcpCall* call, const uint8_t* tool, size_t tool_len, const uint8_t* arguments, size_t arguments_len) {
	clarityAgentRun(call);
}

// While the app is closed nobody is there to ask.
static int32_t agents_grant(void* user, const uint8_t* grant, size_t grant_len) { return CLARITY_APP_MCP_REFUSE; }

int32_t agents_serve(const char* tools, size_t tools_len, const char* options, size_t options_len) {
	ClarityAppMcpHost host = { NULL, NULL, NULL, agents_run, agents_grant };
	return clarity_app_mcp_api()->serve((const uint8_t*)tools, tools_len, (const uint8_t*)options, options_len, &host);
}

int32_t agents_command_line(ClarityAppMcpCall* call, uint8_t** out, size_t* out_len) { return clarity_app_mcp_api()->command_line(call, out, out_len); }

int32_t agents_ask(ClarityAppMcpCall* call, const char* question, size_t question_len, const char* form, size_t form_len, uint8_t** out, size_t* out_len) {
	return clarity_app_mcp_api()->ask_cli(call, (const uint8_t*)question, question_len, (const uint8_t*)form, form_len, out, out_len);
}

void agents_answer(ClarityAppMcpCall* call, const char* text, size_t text_len, int refused) {
	if (refused) {
		clarity_app_mcp_api()->refused(call, (const uint8_t*)text, text_len);
	} else {
		clarity_app_mcp_api()->printed(call, (const uint8_t*)text, text_len);
	}
}

void agents_free(uint8_t* at, size_t len) { clarity_app_mcp_api()->free(at, len); }

int32_t agents_problem(char* into, size_t size) { return clarity_app_mcp_api()->problem(into, size); }

int32_t agents_forward(const char* file, size_t file_len) { return clarity_app_mcp_api()->forward((const uint8_t*)file, file_len); }
*/
import "C"
