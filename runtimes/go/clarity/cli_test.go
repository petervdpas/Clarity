// Command-line programs against Clarity's shared cases
// (conformance/command-line.json): reading a command line, the help, a
// printed table and asking on a terminal.

package clarity

import (
	"encoding/json"
	"os"
	"reflect"
	"strings"
	"testing"
)

type cliCases struct {
	Words   map[string]string `json:"words"`
	Program CliProgram        `json:"program"`
	Parse   []struct {
		What string   `json:"what"`
		Args []string `json:"args"`
		Run  *struct {
			Command string             `json:"command"`
			Values  map[string]*string `json:"values"`
		} `json:"run"`
		Help  *string `json:"help"`
		Wrong *struct {
			Key   string            `json:"key"`
			Slots map[string]string `json:"slots"`
			Said  string            `json:"said"`
		} `json:"wrong"`
	} `json:"parse"`
	Table []struct {
		What     string     `json:"what"`
		Headings []string   `json:"headings"`
		Right    []bool     `json:"right"`
		Rows     [][]string `json:"rows"`
		Said     string     `json:"said"`
	} `json:"table"`
	Ask []struct {
		What     string `json:"what"`
		Question string `json:"question"`
		Form     struct {
			Kind    string   `json:"kind"`
			Yes     string   `json:"yes"`
			No      string   `json:"no"`
			Type    string   `json:"type"`
			Values  []string `json:"values"`
			Default *string  `json:"default"`
			Choices []string `json:"choices"`
		} `json:"form"`
		Terminal bool      `json:"terminal"`
		Typed    []*string `json:"typed"`
		Said     string    `json:"said"`
		Answer   *string   `json:"answer"`
		Exit     *int      `json:"exit"`
	} `json:"ask"`
}

func readCliCases(t *testing.T) cliCases {
	text, err := os.ReadFile("../../../conformance/command-line.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases cliCases
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	return cases
}

var english = CliWordsIn(func(string) string { return "" })

func TestTheCliWordsAreTheSharedOnes(t *testing.T) {
	cases := readCliCases(t)
	if len(cases.Words) != len(CliWords) {
		t.Fatalf("%d words, the cases have %d", len(CliWords), len(cases.Words))
	}
	for _, w := range CliWords {
		if cases.Words[w.Key] != w.English {
			t.Errorf("%s: %q, the cases say %q", w.Key, w.English, cases.Words[w.Key])
		}
	}
}

func TestCommandLinesAreRead(t *testing.T) {
	cases := readCliCases(t)
	for _, c := range cases.Parse {
		got := ParseCommandLine(cases.Program, c.Args, english)
		switch {
		case c.Run != nil:
			if !got.Run {
				t.Errorf("%s: %+v", c.What, got)
				continue
			}
			command := cases.Program.Commands[got.Command]
			if strings.Join(command.Line, " ") != c.Run.Command {
				t.Errorf("%s: ran %v", c.What, command.Line)
			}
			values := map[string]*string{}
			for i, p := range append(append([]CliParam{}, command.Arguments...), command.Options...) {
				values[p.Name] = got.Values[i]
			}
			if !reflect.DeepEqual(values, c.Run.Values) {
				t.Errorf("%s: values %v, wanted %v", c.What, show(values), show(c.Run.Values))
			}
		case c.Help != nil:
			if got.Run || got.Help != *c.Help {
				t.Errorf("%s: help %q, wanted %q", c.What, got.Help, *c.Help)
			}
		default:
			slots := map[string]string{}
			for _, s := range got.Slots {
				slots[s[0]] = s[1]
			}
			if got.Key != c.Wrong.Key || !reflect.DeepEqual(slots, c.Wrong.Slots) || got.Said != c.Wrong.Said {
				t.Errorf("%s: %s %v %q, wanted %s %v %q", c.What, got.Key, slots, got.Said, c.Wrong.Key, c.Wrong.Slots, c.Wrong.Said)
			}
		}
	}
}

func show(values map[string]*string) map[string]string {
	out := map[string]string{}
	for k, v := range values {
		if v == nil {
			out[k] = "<none>"
		} else {
			out[k] = *v
		}
	}
	return out
}

func TestTablesArePrinted(t *testing.T) {
	for _, c := range readCliCases(t).Table {
		if got := CliTable(c.Headings, c.Right, c.Rows); got != c.Said {
			t.Errorf("%s: %q, wanted %q", c.What, got, c.Said)
		}
	}
}

type typedTerminal struct {
	terminal bool
	lines    []*string
	said     strings.Builder
}

func (t *typedTerminal) IsTerminal() bool { return t.terminal }

func (t *typedTerminal) ReadLine() (string, bool) {
	if len(t.lines) == 0 || t.lines[0] == nil {
		return "", false
	}
	line := *t.lines[0]
	t.lines = t.lines[1:]
	return line, true
}

func (t *typedTerminal) Write(text string) { t.said.WriteString(text) }

func TestCommandsAskOnATerminal(t *testing.T) {
	for _, c := range readCliCases(t).Ask {
		asked := CliAsked{Kind: c.Form.Kind, Yes: c.Form.Yes, No: c.Form.No, Choices: c.Form.Choices,
			Param: CliParam{Kind: c.Form.Type, Values: c.Form.Values, Default: c.Form.Default}}
		terminal := &typedTerminal{terminal: c.Terminal, lines: c.Typed}
		answer, exit := CliAsk(c.Question, asked, terminal, english)
		if c.Exit != nil {
			if exit != *c.Exit {
				t.Errorf("%s: exit %d, wanted %d", c.What, exit, *c.Exit)
			}
		} else if exit != 0 || answer != *c.Answer {
			t.Errorf("%s: %q (exit %d), wanted %q", c.What, answer, exit, *c.Answer)
		}
		if terminal.said.String() != c.Said {
			t.Errorf("%s: said %q, wanted %q", c.What, terminal.said.String(), c.Said)
		}
	}
}

func TestCommandLineValuesBecomeTyped(t *testing.T) {
	text := func(s string) *string { return &s }
	if CliInteger(text("42")) != 42 || CliInteger(nil) != 0 {
		t.Error("integer")
	}
	if CliText(text("Ada")) != "Ada" || CliText(nil) != "" {
		t.Error("text")
	}
	if !CliBool(text("true")) || CliBool(nil) {
		t.Error("bool")
	}
	if CliDecimal(text("12.5")).String() != "12.5" || !CliDecimal(nil).IsZero() {
		t.Error("decimal")
	}
	if d := CliDate(text("2026-10-23")); d == nil || d.String() != "2026-10-23" {
		t.Errorf("date %v", d)
	}
	if CliDate(nil) != nil {
		t.Error("no date")
	}
	if m := CliDateTime(text("2026-10-23T14:30:00")); m == nil || m.Hour != 14 {
		t.Errorf("moment %v", m)
	}
	if CliTextOf(int64(42)) != "42" || CliTextOf("Ada") != "Ada" || CliTextOf((*Date)(nil)) != "" {
		t.Error("text of")
	}
}

func TestTheLanguageIsTheSystemsWhenTheProjectHasIt(t *testing.T) {
	available := []string{"en", "nl"}
	if l := CliPickLanguage("nl_NL.UTF-8", available, "en"); l == nil || *l != "nl" {
		t.Errorf("nl: %v", l)
	}
	for _, locale := range []string{"en_US.UTF-8", "de_DE.UTF-8", "C", ""} {
		if l := CliPickLanguage(locale, available, "en"); l != nil {
			t.Errorf("%s: %v", locale, *l)
		}
	}
}

func TestKeptSettingsThatFitAreTakenBack(t *testing.T) {
	type settings struct {
		ID       int64  `json:"id"`
		PageSize int64  `json:"pageSize"`
		Name     string `json:"name"`
	}
	kept := map[string]any{"pageSize": 50, "name": 7, "gone": true}
	got := CliKeptSettings(kept, settings{PageSize: 20, Name: "x"})
	if got.PageSize != 50 || got.Name != "x" {
		t.Errorf("%+v", got)
	}
}
