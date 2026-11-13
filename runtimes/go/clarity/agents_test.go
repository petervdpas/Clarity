package clarity

// A command-line program's commands for agents (docs/ledger/app-mcp.md): a
// call runs the command as if typed; what it prints is the answer; its
// questions go to the agent; refusals, exit codes and unanswered questions
// are tool errors. The library itself is run by agents_library_test.go.

import (
	"encoding/json"
	"errors"
	"os"
	"strings"
	"testing"
)

// fakeCall is a call with its command line and the agent's answers, in order.
type fakeCall struct {
	line    []string
	answers []string
	forms   []string
}

func (c *fakeCall) CommandLine() []string { return c.line }

func (c *fakeCall) Ask(question, form string) (string, bool) {
	c.forms = append(c.forms, form)
	if len(c.answers) == 0 {
		return "", false
	}
	answer := c.answers[0]
	c.answers = c.answers[1:]
	return answer, true
}

func agentProgram() CliProgram {
	def := "1000"
	return CliProgram{Name: "shop", Commands: []CliCommand{
		{Line: []string{"customer", "add"}, Arguments: []CliParam{{Name: "name", Kind: "text"}}, Options: []CliParam{{Name: "limit", Kind: "money", Default: &def}}},
		{Line: []string{"customer", "remove"}, Arguments: []CliParam{{Name: "name", Kind: "text"}}},
		{Line: []string{"report", "send"}},
	}}
}

func agentRun(cli *CliSession, command int, values []*string) (int, error) {
	switch command {
	case 0:
		cli.Print(PlainText("Added " + *values[0] + ", limit " + *values[1]))
	case 1:
		copies := cli.Ask(PlainText("How many copies?"), "integer", nil, nil)
		if !cli.Confirm(PlainText("Remove "+*values[0]+"?"), nil, nil) {
			return 1, errors.New("Kept")
		}
		cli.Print(PlainText("Removed " + *values[0] + " after " + *copies + " copies"))
	default:
		return 3, nil
	}
	return 0, nil
}

func TestAnAgentCallRunsTheCommandAsTyped(t *testing.T) {
	text, refused := RunAgentCall(agentProgram(), &fakeCall{line: []string{"customer", "add", "--", "Ada"}}, agentRun)
	if text != "Added Ada, limit 1000" || refused {
		t.Errorf("%q %v", text, refused)
	}
	text, refused = RunAgentCall(agentProgram(), &fakeCall{line: []string{"customer", "add", "--limit=9000x", "--", "Ada"}}, agentRun)
	if !refused || !strings.HasPrefix(text, "--limit cannot be 9000x") {
		t.Errorf("a wrong line: %q %v", text, refused)
	}
}

func TestAnAgentIsAskedAndACommandStopsWithoutAnAnswer(t *testing.T) {
	call := &fakeCall{line: []string{"customer", "remove", "--", "Ada"}, answers: []string{"2", "true"}}
	if text, refused := RunAgentCall(agentProgram(), call, agentRun); text != "Removed Ada after 2 copies" || refused {
		t.Errorf("%q %v", text, refused)
	}
	if call.forms[0] != `{"kind":"value","type":"integer","values":[]}` || call.forms[1] != `{"kind":"confirm","no":"Cancel","yes":"OK"}` {
		t.Errorf("the forms: %v", call.forms)
	}
	if text, refused := RunAgentCall(agentProgram(), &fakeCall{line: []string{"customer", "remove", "--", "Ada"}, answers: []string{"2", "false"}}, agentRun); text != "Kept" || !refused {
		t.Errorf("a refusal: %q %v", text, refused)
	}
	if text, refused := RunAgentCall(agentProgram(), &fakeCall{line: []string{"customer", "remove", "--", "Ada"}}, agentRun); text != "There is nobody to answer: How many copies?" || !refused {
		t.Errorf("unanswered: %q %v", text, refused)
	}
	if text, refused := RunAgentCall(agentProgram(), &fakeCall{line: []string{"report", "send", "--"}}, agentRun); text != "exit 3" || !refused {
		t.Errorf("an exit code: %q %v", text, refused)
	}
}

func TestWhileClosedThePersonsSettingsDecide(t *testing.T) {
	agents := Agents{App: "Shop", Version: "1.0"}
	var options map[string]any
	_ = json.Unmarshal([]byte(AgentOptions(agents, map[string]any{"agentReadsWhileClosed": true})), &options)
	// A switch the person never set is left out: the service uses what the project allows.
	if _, set := options["closedChanges"]; options["open"] != false || options["closedReads"] != true || set || options["version"] != "1.0" {
		t.Errorf("%v", options)
	}
	texts := options["texts"].(map[string]any)
	if texts["closed"] != "Shop is not open; open it to allow this." || texts["refused"] != "The person using Shop refused this." {
		t.Errorf("%v", texts)
	}
}

func TestTheWordsOfRefusalsAreTheSharedOnes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/app-mcp.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Words map[string]string `json:"words"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	if len(file.Words) != len(AgentWords) {
		t.Fatalf("%v", file.Words)
	}
	for _, w := range AgentWords {
		if file.Words[w.Key] != w.English {
			t.Errorf("%s: %q", w.Key, file.Words[w.Key])
		}
	}
}
