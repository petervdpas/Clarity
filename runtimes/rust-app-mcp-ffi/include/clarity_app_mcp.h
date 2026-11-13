/*
 * clarity_app_mcp.h: the C interface of Clarity.AppMcp (docs/ledger/app-mcp.md).
 * The library (libclarity_app_mcp.so, clarity_app_mcp.dll, libclarity_app_mcp.dylib)
 * is the MCP service of a program Clarity makes: it offers the app's commands
 * for agents as tools, asks the person for each grant through the program, and
 * runs a call through the program (conformance/app-mcp.json).
 *
 * Conventions
 * - Texts are UTF-8 as a pointer and a length, not ended by a zero.
 * - `serve` takes the tool list clarity-gen writes and the options as JSON:
 *   {"version", "open", "closedReads", "closedChanges",
 *    "texts": {"refused", "closed"}}, each left out keeping its default;
 *   the texts come from the program's translations. It returns when the
 *   lines end.
 * - The host's `read` points `line` at the next line and gives 0, or gives 1
 *   at the end; the program keeps the line until its next read. A null `read`
 *   or `write` is standard input or output.
 * - `run` is called with the tool's name and its arguments as JSON; inside it
 *   the program answers with `printed` or `refused`, and a command that asks
 *   calls `ask`. `grant` gets the grant as JSON ({"tool", "description",
 *   "kind": "read"|"change", "arguments", "changes"}) and gives
 *   CLARITY_APP_MCP_ONCE, _SESSION or _REFUSE.
 * - What the library allocates into `*out` and `*out_len` the program gives
 *   back with `free`. A failing call gives a code; `problem` copies what it
 *   was about. No panic crosses the interface.
 */

#ifndef CLARITY_APP_MCP_H
#define CLARITY_APP_MCP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_APP_MCP_API __declspec(dllimport)
#else
#define CLARITY_APP_MCP_API
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_APP_MCP_INTERFACE 1

#define CLARITY_APP_MCP_OK 0
/* The tool list, the options or a text given is not what it should be. */
#define CLARITY_APP_MCP_BAD_INPUT 1
/* No running.json, or not a whole one. */
#define CLARITY_APP_MCP_NOT_FOUND 2
/* A fault inside Clarity.AppMcp. */
#define CLARITY_APP_MCP_FAILED 3

/* What `ask` gives: the answer in *out as JSON, or why there is none. */
#define CLARITY_APP_MCP_GIVEN 0
#define CLARITY_APP_MCP_DECLINED 1
#define CLARITY_APP_MCP_CANCELLED 2
#define CLARITY_APP_MCP_UNSUPPORTED 3

/* The person's answer to a grant. */
#define CLARITY_APP_MCP_ONCE 0
#define CLARITY_APP_MCP_SESSION 1
#define CLARITY_APP_MCP_REFUSE 2

/* A call being run; valid only inside `run`. */
typedef struct ClarityAppMcpCall ClarityAppMcpCall;

typedef struct ClarityAppMcpHost {
    void* user;
    int32_t (*read)(void* user, const uint8_t** line, size_t* line_len);
    void (*write)(void* user, const uint8_t* line, size_t line_len);
    void (*run)(void* user, ClarityAppMcpCall* call, const uint8_t* tool, size_t tool_len,
                const uint8_t* arguments, size_t arguments_len);
    int32_t (*grant)(void* user, const uint8_t* grant, size_t grant_len);
} ClarityAppMcpHost;

typedef struct ClarityAppMcpApi {
    uint32_t interface;
    int32_t (*serve)(const uint8_t* tool_list, size_t tool_list_len, const uint8_t* options,
                     size_t options_len, const ClarityAppMcpHost* host);
    void (*printed)(ClarityAppMcpCall* call, const uint8_t* text, size_t text_len);
    void (*refused)(ClarityAppMcpCall* call, const uint8_t* text, size_t text_len);
    int32_t (*ask)(ClarityAppMcpCall* call, const uint8_t* message, size_t message_len,
                   const uint8_t* schema, size_t schema_len, uint8_t** out, size_t* out_len);
    /* Writes running.json at `file` for this process with a new token; gives it as JSON. */
    int32_t (*running_write)(const uint8_t* file, size_t file_len, const uint8_t* app, size_t app_len,
                             const uint8_t* address, size_t address_len, uint8_t** out, size_t* out_len);
    int32_t (*running_read)(const uint8_t* file, size_t file_len, uint8_t** out, size_t* out_len);
    /* Removes running.json when it is this process's own. */
    void (*running_remove)(const uint8_t* file, size_t file_len);
    void (*free)(uint8_t* at, size_t len);
    int32_t (*problem)(char* into, size_t size);
    /* Inside `run`: the call as the command line the program reads with its own
     * parser, a JSON array of texts (the command's words, `--name=value`
     * options, `--`, the arguments). */
    int32_t (*command_line)(ClarityAppMcpCall* call, uint8_t** out, size_t* out_len);
    /* Inside `run`: asks the agent a command's question; `form` is JSON as
     * conformance/command-line.json writes it. The answer, as the terminal
     * gives it, goes to *out; CLARITY_APP_MCP_NOT_FOUND when there is none. */
    int32_t (*ask_cli)(ClarityAppMcpCall* call, const uint8_t* question, size_t question_len,
                       const uint8_t* form, size_t form_len, uint8_t** out, size_t* out_len);
    /* Forwards standard input and output to the running app that running.json
     * at `file` names (beside the app's settings), until either ends: 1 when an
     * app answered, 0 when none does; the program then serves as the closed app. */
    int32_t (*forward)(const uint8_t* file, size_t file_len);
} ClarityAppMcpApi;

/* The table, the same for the library's whole life. */
CLARITY_APP_MCP_API const ClarityAppMcpApi* clarity_app_mcp_api(void);

#ifdef __cplusplus
}
#endif

#endif
