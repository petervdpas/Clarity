package clarity

// A command-line program's commands for AI agents (docs/ledger/app-mcp.md):
// `<program> mcp` serves them over MCP through Clarity.AppMcp while the app
// is closed. The program's own cgo files (agents_library.go and
// agents_host.go, generated only when a command has an `agent` line) call
// the library; this module stays pure Go. A call becomes the command line
// the program reads with its own parser, runs as if typed, and what it
// prints is the call's answer; its questions go to the agent. Nobody is
// there to ask for a grant: the person's settings agentReadsWhileClosed and
// agentChangesWhileClosed allow, else the call is refused.

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// The settings, kept in the person's settings.json, that allow agents while
// the app is closed.
const (
	AgentReadsSetting   = "agentReadsWhileClosed"
	AgentChangesSetting = "agentChangesWhileClosed"
)

// AgentWords are Clarity's words of a program whose commands agents may
// call, in English (conformance/app-mcp.json): what its MCP service says of
// a refused call, and its help's line for `mcp`.
var AgentWords = []struct{ Key, English string }{
	{Key: "agentRefused", English: "The person using {app} refused this."},
	{Key: "agentClosed", English: "{app} is not open; open it to allow this."},
	{Key: "agentMcp", English: "Serves the commands for AI agents over MCP"},
}

// Agents is what a program serves: its tool list (as clarity-gen writes
// it), the app's title, the name its settings are kept under, its version.
type Agents struct{ Tools, App, Program, Version string }

// AgentCall is a call the library hands over: the command line it becomes,
// and asking the agent a question of a form (conformance/command-line.json);
// false when there is no answer.
type AgentCall interface {
	CommandLine() []string
	Ask(question, form string) (string, bool)
}

// AgentServe serves tools with options (JSON) through the library, running
// each call with run: its answer, and whether it is refused.
type AgentServe func(tools, options string, run func(AgentCall) (string, bool)) error

// AgentForward forwards standard input and output to the running app that
// running.json at file names, until either ends; false when none answers.
type AgentForward func(file string) bool

// cliUnanswered stops a command whose question nobody answered.
type cliUnanswered struct{ text string }

func agentSay(key, app string) string {
	text := cliTranslated(key)
	if text == "" {
		for _, w := range AgentWords {
			if w.Key == key {
				text = w.English
			}
		}
	}
	return strings.ReplaceAll(text, "{app}", app)
}

// AgentOptions is how the program serves while the app is closed, with what
// the person's settings kept allow, as JSON.
func AgentOptions(agents Agents, kept map[string]any) string {
	given := map[string]any{
		"version": agents.Version,
		"open":    false,
		"texts":   map[string]string{"refused": agentSay("agentRefused", agents.App), "closed": agentSay("agentClosed", agents.App)},
	}
	// A switch the person never set is left out: the service uses what the project allows.
	for key, setting := range map[string]string{"closedReads": AgentReadsSetting, "closedChanges": AgentChangesSetting} {
		if on, set := kept[setting].(bool); set {
			given[key] = on
		}
	}
	options, _ := json.Marshal(given)
	return string(options)
}

// AgentForm is a question's form, as conformance/command-line.json writes it.
func AgentForm(asked CliAsked) string {
	form := map[string]any{"kind": asked.Kind}
	switch asked.Kind {
	case "confirm":
		form["yes"], form["no"] = asked.Yes, asked.No
	case "choice":
		form["choices"] = asked.Choices
	default:
		values := asked.Param.Values
		if values == nil {
			values = []string{}
		}
		form["type"], form["values"] = asked.Param.Kind, values
		if asked.Param.Default != nil {
			form["default"] = *asked.Param.Default
		}
	}
	text, _ := json.Marshal(form)
	return string(text)
}

// RunAgentCall runs a call as the command line it becomes: what the command
// printed, and whether the call is refused (a wrong line, a refusal, an exit
// code, a question nobody answered).
func RunAgentCall(program CliProgram, call AgentCall, run func(cli *CliSession, command int, values []*string) (int, error)) (text string, refused bool) {
	parsed := ParseCommandLine(program, call.CommandLine(), CliWordsIn(cliTranslated))
	switch {
	case parsed.Key != "":
		return strings.TrimRight(parsed.Said, "\n"), true
	case !parsed.Run:
		return strings.TrimRight(parsed.Help, "\n"), false
	}
	session := &CliSession{Terminal: NewCliConsole(), agent: call, printed: &strings.Builder{}}
	defer func() {
		if fault := recover(); fault != nil {
			unanswered, ok := fault.(cliUnanswered)
			if !ok {
				panic(fault)
			}
			text, refused = unanswered.text, true
		}
	}()
	code, err := run(session, parsed.Command, parsed.Values)
	printed := strings.TrimRight(session.printed.String(), "\n ")
	switch {
	case err != nil:
		return err.Error(), true
	case code != 0 && printed == "":
		return fmt.Sprintf("exit %d", code), true
	case code != 0:
		return printed, true
	}
	return printed, false
}

// lineCall is a call the running app hands over: its command line, and the agent's asker.
type lineCall struct {
	line []string
	ask  func(question, form string) (string, bool)
}

func (c lineCall) CommandLine() []string                    { return c.line }
func (c lineCall) Ask(question, form string) (string, bool) { return c.ask(question, form) }

// RunAgentLine runs the command line line for an agent in the running app
// (Clarity.UI's `agent_call`), its questions going to ask: what it printed,
// and whether the call is refused.
func RunAgentLine(program CliProgram, line []string, ask func(question, form string) (string, bool), run func(cli *CliSession, command int, values []*string) (int, error)) (string, bool) {
	return RunAgentCall(program, lineCall{line, ask}, run)
}

// RunCliProgramForAgents runs a command-line program whose commands agents
// may call: `mcp` as the only word forwards to the running app when one
// answers (forward), else serves them over standard input and output through
// serve; anything else is the command line as RunCliProgram reads it.
func RunCliProgramForAgents(program CliProgram, agents Agents, serve AgentServe, forward AgentForward, run func(cli *CliSession, command int, values []*string) (int, error)) int {
	if len(os.Args) != 2 || os.Args[1] != "mcp" {
		return RunCliProgram(program, run)
	}
	return ServeAgents(program, agents, serve, forward, run)
}

// ServeAgents is `<program> mcp`, of the command-line program or the desktop
// one: it forwards standard input and output to the running app when one
// answers (forward), else serves the commands as the closed app (serve).
func ServeAgents(program CliProgram, agents Agents, serve AgentServe, forward AgentForward, run func(cli *CliSession, command int, values []*string) (int, error)) int {
	// The running app answers agents itself, granting each change.
	if file, ok := SettingsFile(agents.Program); ok && forward(filepath.Join(filepath.Dir(file), "running.json")) {
		return 0
	}
	err := serve(agents.Tools, AgentOptions(agents, ReadSettings(agents.Program)), func(call AgentCall) (string, bool) {
		return RunAgentCall(program, call, run)
	})
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	return 0
}
