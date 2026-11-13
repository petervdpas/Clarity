//go:build clarityagents

package clarity

// Clarity.AppMcp through cgo, as a program links it: the test runs itself
// as the program (`mcp`), served over standard input and output.

import (
	"encoding/json"
	"os"
	"os/exec"
	"strings"
	"testing"
)

const agentTools = `{"name":"shop","instructions":"","tools":[
{"name":"customer-add","description":"Adds","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"limit":{"type":"number"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","add"],"arguments":["name"]},
{"name":"customer-remove","description":"Removes","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","remove"],"arguments":["name"]}]}`

func TestAgentsServedByTheProgram(t *testing.T) {
	if os.Getenv("CLARITY_AGENTS_PROGRAM") == "1" {
		serve := func(tools, options string, run func(AgentCall) (string, bool)) error {
			return agentsServe(tools, options, func(c agentCall) (string, bool) { return run(c) })
		}
		os.Args = []string{"shop", "mcp"}
		program := os.Getenv("CLARITY_AGENTS_SETTINGS")
		os.Exit(RunCliProgramForAgents(agentProgram(), Agents{Tools: agentTools, App: "Shop", Program: program}, serve, agentsForward, agentRun))
	}
	dir := t.TempDir()
	settings := dir + "/shop/settings.json"
	if err := os.MkdirAll(dir+"/shop", 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(settings, []byte(`{"agentChangesWhileClosed": true}`), 0o644); err != nil {
		t.Fatal(err)
	}
	sends := []string{
		`{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{"elicitation":{}}}}`,
		`{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-add","arguments":{"name":"Ada","limit":250}}}`,
		`{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"customer-remove","arguments":{"name":"Ada"}}}`,
		`{"jsonrpc":"2.0","id":"ask-1","result":{"action":"accept","content":{"answer":2}}}`,
		`{"jsonrpc":"2.0","id":"ask-2","result":{"action":"accept","content":{"answer":true}}}`,
	}
	child := exec.Command(os.Args[0], "-test.run=^TestAgentsServedByTheProgram$")
	child.Env = append(os.Environ(), "CLARITY_AGENTS_PROGRAM=1", "CLARITY_AGENTS_SETTINGS=shop", "XDG_CONFIG_HOME="+dir)
	child.Stdin = strings.NewReader(strings.Join(sends, "\n") + "\n")
	out, err := child.Output()
	if err != nil {
		t.Fatalf("%v: %s", err, out)
	}
	var replies []map[string]any
	for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
		var reply map[string]any
		if err := json.Unmarshal([]byte(line), &reply); err != nil {
			t.Fatalf("%v: %s", err, line)
		}
		replies = append(replies, reply)
	}
	said := func(reply map[string]any) (string, bool) {
		result := reply["result"].(map[string]any)
		return result["content"].([]any)[0].(map[string]any)["text"].(string), result["isError"].(bool)
	}
	if len(replies) != 5 {
		t.Fatalf("%s", out)
	}
	if text, refused := said(replies[1]); text != "Added Ada, limit 250" || refused {
		t.Errorf("%q %v", text, refused)
	}
	if replies[2]["method"] != "elicitation/create" || replies[3]["method"] != "elicitation/create" {
		t.Errorf("the questions: %v %v", replies[2], replies[3])
	}
	if text, refused := said(replies[4]); text != "Removed Ada after 2 copies" || refused {
		t.Errorf("%q %v", text, refused)
	}
}
