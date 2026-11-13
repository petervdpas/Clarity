package shell

// A desktop program whose commands agents may call (docs/ledger/app-mcp.md)
// answers the library what it offers, and runs a command line for an agent,
// its questions going to the agent's asker.

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestAProgramAnswersWhatItOffersAgentsAndRunsTheirCalls(t *testing.T) {
	asked := ""
	agents := &Agents{Tools: `{"tools":[]}`, App: "Shop", Program: "shop", Version: "1.0", Call: func(line []string, ask func(question, form string) (string, bool)) (string, bool) {
		answer, _ := ask("Sure?", `{"kind":"confirm","yes":"Yes","no":"No"}`)
		asked = answer
		return "ran " + strings.Join(line, " "), false
	}}
	before := AgentAsk
	AgentAsk = func(question, form string) (string, bool) { return "true", true }
	defer func() { AgentAsk = before }()
	p := NewProgram(ProgramOptions{Agents: agents})
	var offered struct {
		Value map[string]string `json:"value"`
	}
	_ = json.Unmarshal(p.Answer("agents", nil, 0), &offered)
	if offered.Value["app"] != "Shop" || offered.Value["program"] != "shop" {
		t.Errorf("offered %v", offered.Value)
	}
	var ran struct {
		Value struct {
			Text    string `json:"text"`
			Refused bool   `json:"refused"`
		} `json:"value"`
	}
	_ = json.Unmarshal(p.Answer("agent_call", []byte(`{"line":["customer","add","--","Ada"]}`), 0), &ran)
	if ran.Value.Text != "ran customer add -- Ada" || ran.Value.Refused || asked != "true" {
		t.Errorf("%+v, asked %q", ran.Value, asked)
	}
	if got := NewProgram(ProgramOptions{}).Answer("agents", nil, 0); got != nil {
		t.Errorf("an app without commands for agents answers nothing: %s", got)
	}
}
