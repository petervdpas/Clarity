// A command-line program's commands for AI agents (docs/ledger/app-mcp.md):
// `<program> mcp` serves them over MCP through Clarity.AppMcp while the app
// is closed. A call becomes the command line the program reads with its
// own parser, runs as if typed, and what it prints is the call's answer;
// its questions go to the agent. Nobody is there to ask for a grant: the
// person's settings agentReadsWhileClosed and agentChangesWhileClosed
// allow, else the call is refused. Carried only when a command has an
// `agent` line (clarity_use_agents in clarity.cmake links the library).

#pragma once

#include <stdbool.h>
#include <stddef.h>

#include "cjson/cJSON.h"
#include "clarity/cli.h"

/// The settings, kept in the person's settings.json, that allow agents
/// while the app is closed.
#define CLARITY_AGENT_READS_SETTING "agentReadsWhileClosed"
#define CLARITY_AGENT_CHANGES_SETTING "agentChangesWhileClosed"

/// What a program serves: its tool list (as clarity-gen writes it), the
/// app's title, the name its settings are kept under, and its version.
typedef struct {
    const char* tools;
    const char* app;
    const char* program;
    const char* version;
} ClarityAgents;

/// Runs a command as the program's dispatch does (clarity/cli.h).
typedef ClarityCliRun ClarityAgentsRun;

/// Where the messages come from and go: `read` gives the next line or NULL
/// at the end, `write` writes one. NULL lines: standard input and output.
typedef struct {
    const char* (*read)(void* context);
    void (*write)(void* context, const char* line);
    void* context;
} ClarityAgentsLines;

/// How the program serves while the app is closed, with what the person's
/// settings `kept` allow, as JSON in the scratch arena.
const char* clarity_agents_options(const ClarityAgents* agents, const cJSON* kept);

/// Forwards standard input and output to the running app of `program` (its
/// running.json beside the settings) until either ends; false when none
/// answers, and the program serves as the closed app.
bool clarity_agents_forward(const char* program);

/// Serves `program`'s commands for agents until the lines end, with the
/// person's settings `kept`; false (with clarity_error) when it cannot.
bool clarity_agents_serve(const ClarityCliProgram* program, const ClarityAgents* agents, const cJSON* kept, const ClarityAgentsLines* lines, ClarityAgentsRun run, void* context);
