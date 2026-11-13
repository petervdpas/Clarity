// A command-line program's commands for AI agents (docs/ledger/app-mcp.md):
// `<program> mcp` serves them over MCP through Clarity.AppMcp while the app
// is closed. A call becomes the command line the program reads with its
// own parser, runs as if typed, and what it prints is the call's answer;
// its questions go to the agent. Nobody is there to ask for a grant: the
// person's settings agentReadsWhileClosed and agentChangesWhileClosed
// allow, else the call is refused. Carried only when a command has an
// `agent` line (clarity_use_agents in clarity.cmake links the library).

#pragma once

#include <functional>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/cli.hpp"

namespace clarity::agents {

/// The settings, kept in the person's settings.json, that allow agents
/// while the app is closed.
inline constexpr const char* reads_setting = "agentReadsWhileClosed";
inline constexpr const char* changes_setting = "agentChangesWhileClosed";

/// Clarity's words of an app for agents, in English (conformance/app-mcp.json).
extern const std::vector<std::pair<std::string, std::string>>& words;

/// What a program serves: its tool list (as clarity-gen writes it), the
/// app's title, the name its settings are kept under, and its version.
struct Agents {
    std::string tools;
    std::string app;
    std::string program;
    std::string version;
};

/// Runs a command as the program's dispatch does: its exit code.
using Run = std::function<int(cli::Session&, size_t, const std::vector<std::optional<std::string>>&)>;

/// Where the messages come from and go: `read` gives the next line or
/// nothing at the end. Left out: standard input and output.
struct Lines {
    std::function<std::optional<std::string>()> read;
    std::function<void(const std::string&)> write;
};

/// How the program serves while the app is closed, with what the person's
/// settings `kept` allow, as JSON.
std::string options(const Agents& agents, const nlohmann::json& kept);

/// Serves `program`'s commands for agents until the lines end, with the
/// person's settings `kept`. Throws an InvalidError when it cannot.
void serve(const cli::Program& program, const Agents& agents, const nlohmann::json& kept, const std::optional<Lines>& lines, const Run& run);

/// Runs a command-line program whose commands agents may call: `mcp` as the
/// only word serves them over standard input and output; anything else is
/// the command line as cli::run reads it.
int run_program(const cli::Program& program, const Agents& agents, int argc, char** argv, const Run& run);

}  // namespace clarity::agents
