// The Go layer over Clarity.UI, driven through the library as a person
// would (clarity_driven_*): a window model in Go behind the library's
// commands, the desktop's own commands, long calls by thread, engines
// beside the screen and custom parts written in Go.

package ui

import (
	"encoding/json"
	"fmt"
	"strings"
	"sync"
	"testing"
	"testing/fstest"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
	"github.com/petervdpas/Clarity/runtimes/go/shell"
)

const mainWindow = `<Window name="Main"><Textbox name="nameBox" at="1,1" span="4" bind="name" undoScope="nameBox" /><Button name="save" at="1,2" span="3" caption="Save" onclick="save" /><Button name="quit" at="4,2" span="3" caption="Quit" onclick="quitIt" /><Button name="other" at="7,2" span="3" caption="Other" onclick="openOther" /><Custom name="gauge" at="1,3" span="4" rows="3" part="GoGauge" bind="speed" tells='[{"tell":"reached","handler":"gaugeReached"}]' /></Window>`

const otherWindow = `<Window name="Other"><Label name="hello" at="1,1" span="4" caption="Other" /></Window>`

// windows is a window model as a generated program answers it: Main with a
// name and a speed, its handlers save, quitIt, openOther and gaugeReached.
type windows struct {
	mu      sync.Mutex
	ran     []string
	watched []clarity.Watch
}

func (w *windows) Open(db *clarity.Db, window string, with map[string]any) (clarity.WindowState, error) {
	if window == "Other" {
		return clarity.NewWindowState("Other", map[string]any{}), nil
	}
	return clarity.NewWindowState(window, map[string]any{"name": "Ann", "speed": 3}), nil
}

func (w *windows) Handler(window, event string) (string, bool) { return "", false }

func (w *windows) Dispatch(db *clarity.Db, state clarity.WindowState, handler string) (clarity.Reply, error) {
	w.mu.Lock()
	w.ran = append(w.ran, handler)
	w.watched = append(w.watched, clarity.CurrentWatch())
	w.mu.Unlock()
	ui := clarity.NewUi(state)
	switch handler {
	case "save":
		state.Data["name"] = "Saved"
		ui.Message(clarity.Info, clarity.PlainText("Saved"))
	case "quitIt":
		ui.Quit()
	case "openOther":
		ui.Open("Other", nil)
	case "long":
		if watch := clarity.CurrentWatch(); watch.Progress != nil {
			watch.Progress(3, 10)
		}
	}
	return ui.Reply(state.Window, state.Data)
}

// Answer runs a question's handler, or the one a custom part's tell is wired to, with its values.
func (w *windows) Answer(db *clarity.Db, state clarity.WindowState, handler string, values []json.RawMessage) (clarity.Reply, error) {
	w.mu.Lock()
	w.ran = append(w.ran, fmt.Sprintf("%s %s", handler, values))
	w.mu.Unlock()
	if handler == "gaugeReached" {
		state.Data["name"] = "Reached"
	}
	return clarity.NewUi(state).Reply(state.Window, state.Data)
}

func (w *windows) Redraw(db *clarity.Db, reply clarity.Reply) (clarity.Reply, error) {
	return reply, nil
}

func (w *windows) Options(db *clarity.Db, entity string) ([]clarity.RefOption, error) {
	return []clarity.RefOption{{ID: 1, Text: "One"}}, nil
}

func (w *windows) Values(db *clarity.Db, entity, field string) ([]string, error) {
	return []string{"a"}, nil
}

func (w *windows) Picture(db *clarity.Db, entity string, id int64, field string) ([]byte, error) {
	return []byte{1, 2, 3}, nil
}

func desktop(t *testing.T, model *windows) Desktop {
	t.Helper()
	db, err := clarity.Memory()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { db.Close() })
	return Desktop{
		Options: shell.Options{Name: "GoLayerTest", Title: "Test", Main: "Main", Db: db, Windows: model},
		Files: fstest.MapFS{
			"windows/Main.cxaml":  {Data: []byte(mainWindow)},
			"windows/Other.cxaml": {Data: []byte(otherWindow)},
		},
	}
}

// driving starts the desktop's program on a screen without a window.
func driving(t *testing.T, d Desktop) (*Driven, *Program) {
	t.Helper()
	files, err := FilesOf(d.Files, d.Options.Model, d.Options.Title, 0, 0)
	if err != nil {
		t.Fatal(err)
	}
	program := NewProgram(d)
	driven, err := StartDriven(program, files)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(driven.Close)
	return driven, program
}

func step(t *testing.T, d *Driven, step string) map[string]any {
	t.Helper()
	said, err := d.Step(step)
	if err != nil {
		t.Fatalf("%s: %v (%s)", step, err, said)
	}
	var out map[string]any
	if err := json.Unmarshal([]byte(said), &out); err != nil {
		t.Fatalf("%s: %v", said, err)
	}
	return out
}

func TestTheHeaderAndTheLibraryBelongTogether(t *testing.T) {
	library, header := Interface()
	if library != header {
		t.Fatalf("the library has interface %d, the header %d", library, header)
	}
	if Version() == "" {
		t.Fatal("the library names no version")
	}
}

func TestADrivenProgramRunsItsHandlerAndShowsItsReply(t *testing.T) {
	model := &windows{}
	driven, _ := driving(t, desktop(t, model))
	read := step(t, driven, `{"do":"read"}`)
	if data := read["data"].(map[string]any); data["name"] != "Ann" {
		t.Fatalf("the window opened with %v", read)
	}
	step(t, driven, `{"do":"click","part":"save"}`)
	read = step(t, driven, `{"do":"read"}`)
	if data := read["data"].(map[string]any); data["name"] != "Saved" {
		t.Fatalf("after save: %v", read)
	}
	if model.ran[len(model.ran)-1] != "save" {
		t.Fatalf("ran %v", model.ran)
	}
	// A step that cannot be done fails, and still says why.
	said, err := driven.Step(`{"do":"click","part":"nothing"}`)
	if err == nil || !strings.Contains(said, "nothing") {
		t.Fatalf("a click on no part: %v %s", err, said)
	}
}

func TestTypingInABoundBoxIsAnEditOfTheWindowModel(t *testing.T) {
	driven, _ := driving(t, desktop(t, &windows{}))
	step(t, driven, `{"do":"focus","part":"nameBox"}`)
	step(t, driven, `{"do":"key","key":"End"}`)
	step(t, driven, `{"do":"type","text":" B"}`)
	step(t, driven, `{"do":"key","key":"Tab"}`)
	read := step(t, driven, `{"do":"read"}`)
	if data := read["data"].(map[string]any); data["name"] != "Ann B" {
		t.Fatalf("after typing: %v", read["data"])
	}
}

func TestAHandlerOpensAWindowAndQuits(t *testing.T) {
	driven, _ := driving(t, desktop(t, &windows{}))
	step(t, driven, `{"do":"click","part":"quit"}`)
	read := step(t, driven, `{"do":"read"}`)
	if read["quit"] != true {
		t.Fatalf("code quit, and the run says %v", read)
	}
	other, _ := driving(t, desktop(t, &windows{}))
	step(t, other, `{"do":"click","part":"other"}`)
	read = step(t, other, `{"do":"read"}`)
	if fmt.Sprint(read["windows"]) != "[Main Other]" {
		t.Fatalf("windows %v", read["windows"])
	}
}

func TestStartedSaysTheMainWindowAndWhyTheDatabasesDidNotOpen(t *testing.T) {
	d := desktop(t, &windows{})
	answer := string(NewProgram(d).Answer("started", nil, 1))
	if answer != `{"value":{"main":"Main","unconnected":null}}` {
		t.Fatalf("started: %s", answer)
	}
	d.Options.Unconnected = &shell.Unconnected{Window: "Connection", Problem: clarity.PlainText("no server")}
	answer = string(NewProgram(d).Answer("started", nil, 1))
	if !strings.Contains(answer, `"main":"Main"`) || !strings.Contains(answer, `"window":"Connection"`) || !strings.Contains(answer, `"handler":null`) {
		t.Fatalf("started unconnected: %s", answer)
	}
}

func TestTheDesktopsOwnCommandsAndThoseOfTheWire(t *testing.T) {
	d := desktop(t, &windows{})
	p := NewProgram(d)
	cases := map[string][2]string{
		"field_bytes":  {`{"entity":"Album","id":1,"field":"cover"}`, `{"value":"AQID"}`},
		"ref_options":  {`{"entity":"Album"}`, `{"value":[{"id":1,"text":"One"}]}`},
		"field_values": {`{"entity":"Album","field":"title"}`, `{"value":["a"]}`},
		"watches":      {`{}`, `{"value":false}`},
		"set_language": {`{"language":"nl"}`, `{"value":null}`},
	}
	for command, c := range cases {
		if got := string(p.Answer(command, []byte(c[0]), 1)); got != c[1] {
			t.Fatalf("%s: %s, want %s", command, got, c[1])
		}
	}
	// What the program does not answer takes the library's default.
	for _, command := range []string{"facts", "engine_open", "no_such_command", "set_title"} {
		if got := p.Answer(command, []byte(`{"name":"Sky"}`), 1); got != nil {
			t.Fatalf("%s answered %s", command, got)
		}
	}
	d.Facts = []byte(`{"fields":{}}`)
	if got := string(NewProgram(d).Answer("facts", nil, 1)); got != `{"value":{"fields":{}}}` {
		t.Fatalf("facts: %s", got)
	}
}

// engines is a program's engines beside the screen: Sky alone.
type engines struct{ closed []uint64 }

func (e *engines) EngineOpen(name string) (uint64, bool, error) {
	switch name {
	case "Sky":
		return 7, true, nil
	case "Broken":
		return 0, true, fmt.Errorf("no such file")
	}
	return 0, false, nil
}

func (e *engines) EngineHears(id uint64) (bool, bool) { return true, false }

func (e *engines) EngineFrame(id uint64, ms float64, input json.RawMessage) (json.RawMessage, error) {
	return json.RawMessage(fmt.Sprintf(`{"shapes":[],"tells":[],"ms":%g,"input":%s}`, ms, input)), nil
}

func (e *engines) EngineClose(id uint64) { e.closed = append(e.closed, id) }

func TestEnginesBesideTheScreenGoByNumber(t *testing.T) {
	d := desktop(t, &windows{})
	own := &engines{}
	d.Engines = own
	p := NewProgram(d)
	cases := [][3]string{
		{"engine_open", `{"name":"Sky"}`, `{"value":7}`},
		{"engine_open", `{"name":"Moon"}`, `{"value":null}`},
		{"engine_open", `{"name":"Broken"}`, `{"error":"no such file"}`},
		{"engine_hears", `{"id":7}`, `{"value":{"keys":false,"pointer":true}}`},
		{"engine_frame", `{"id":7,"ms":16,"input":[{"key":{"kind":"down","name":"a"}}]}`, `{"value":{"shapes":[],"tells":[],"ms":16,"input":[{"key":{"kind":"down","name":"a"}}]}}`},
		{"engine_close", `{"id":7}`, `{"value":null}`},
	}
	for _, c := range cases {
		if got := string(p.Answer(c[0], []byte(c[1]), 1)); got != c[2] {
			t.Fatalf("%s %s: %s, want %s", c[0], c[1], got, c[2])
		}
	}
	if fmt.Sprint(own.closed) != "[7]" {
		t.Fatalf("closed %v", own.closed)
	}
}

// gauge is a custom part written in Go: it shows its speed, and a click
// changes it and tells reached.
type gauge struct {
	speed   int64
	started bool
	drawn   int
}

func (g *gauge) Set(name string, value json.RawMessage, cx *Cx) {
	if name == "value" {
		_ = json.Unmarshal(value, &g.speed)
	}
}

func (g *gauge) Hear(hear string, values []json.RawMessage, cx *Cx) {}

func (g *gauge) Draw(canvas *Canvas, x, y, width, height float64) {
	g.drawn++
	canvas.Fill(x, y, width, height, canvas.LookColor("accent"), 4)
	canvas.Text(fmt.Sprint(g.speed), x, y, width, height, 0xFF000000, 0, true, 1)
	pixels := make([]byte, 4*2*2)
	canvas.Pixels(pixels, 2, 2, 8, x, y, 4, 4)
}

func (g *gauge) Start(cx *Cx) { g.started = true }

func (g *gauge) Click(cx *Cx) {
	g.speed++
	cx.Change(g.speed)
	cx.Tell("reached", g.speed)
}

var made []*gauge

func TestACustomPartWrittenInGoDrawsHearsAndTells(t *testing.T) {
	if err := Register("GoGauge", func() Part {
		g := &gauge{}
		made = append(made, g)
		return g
	}); err != nil {
		t.Fatal(err)
	}
	model := &windows{}
	driven, _ := driving(t, desktop(t, model))
	step(t, driven, `{"do":"read"}`)
	g := made[len(made)-1]
	if g.speed != 3 || !g.started || g.drawn == 0 {
		t.Fatalf("the gauge was made with speed %d, started %v, drawn %d", g.speed, g.started, g.drawn)
	}
	step(t, driven, `{"do":"click","part":"gauge"}`)
	read := step(t, driven, `{"do":"read"}`)
	data := read["data"].(map[string]any)
	ranOf := model.ran
	// Its change reached what it binds, and its tell ran the handler wired to it.
	if data["speed"] != float64(4) || data["name"] != "Reached" || fmt.Sprint(ranOf) != "[gaugeReached [4]]" {
		t.Fatalf("after a click on the gauge: %v, ran %v", data, ranOf)
	}
}

// settingsWindows is a window model whose Main has the person's settings
// record; its handler flip says what it saw and turns showDone over.
type settingsWindows struct {
	windows
	saw, tokens []any
}

func (w *settingsWindows) Open(db *clarity.Db, window string, with map[string]any) (clarity.WindowState, error) {
	return clarity.NewWindowState(window, map[string]any{"settings": map[string]any{"id": 0.0, "showDone": true, "theme": "", "apiToken": ""}}), nil
}

func (w *settingsWindows) Dispatch(db *clarity.Db, state clarity.WindowState, handler string) (clarity.Reply, error) {
	settings := state.Data["settings"].(map[string]any)
	w.mu.Lock()
	w.saw = append(w.saw, settings["showDone"])
	w.tokens = append(w.tokens, settings["apiToken"])
	w.mu.Unlock()
	settings["showDone"] = !settings["showDone"].(bool)
	return clarity.NewUi(state).Reply(state.Window, state.Data)
}

// A window starts from the settings the person kept, and settings a handler
// changes are kept: the next run's handler sees them.
func TestAWindowStartsFromTheKeptSettingsAndAHandlersChangeIsKept(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())
	run := func() []any {
		model := &settingsWindows{}
		d := desktop(t, &model.windows)
		d.Options.Windows = model
		d.Files = fstest.MapFS{"windows/Main.cxaml": {Data: []byte(`<Window name="Main"><Button name="flip" at="1,1" span="3" caption="Flip" onclick="flip" /></Window>`)}}
		driven, _ := driving(t, d)
		step(t, driven, `{"do":"click","part":"flip"}`)
		driven.Close()
		return model.saw
	}
	if saw := run(); fmt.Sprint(saw) != "[true]" {
		t.Fatalf("the first run saw %v", saw)
	}
	if kept := clarity.ReadSettings("GoLayerTest"); kept["showDone"] != false {
		t.Fatalf("kept %v", kept)
	}
	if saw := run(); fmt.Sprint(saw) != "[false]" {
		t.Fatalf("the second run saw %v, not the kept setting", saw)
	}
}

// storeOf is a protected store in memory.
type storeOf map[string]string

func (s storeOf) Get(program, name string) string { return s[program+"/"+name] }
func (s storeOf) Set(program, name, value string) error {
	s[program+"/"+name] = value
	return nil
}

// A secret setting a handler reads has its value from the protected store,
// never from the settings file.
func TestASecretSettingAHandlerReadsComesFromTheStore(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())
	clarity.Secrets = storeOf{"GoLayerTest/apiToken": "s3cret"}
	t.Cleanup(func() { clarity.Secrets = nil })
	model := &settingsWindows{}
	d := desktop(t, &model.windows)
	d.Options.Windows = model
	d.Options.Secrets = []string{"apiToken"}
	d.Files = fstest.MapFS{"windows/Main.cxaml": {Data: []byte(`<Window name="Main"><Button name="flip" at="1,1" span="3" caption="Flip" onclick="flip" /></Window>`)}}
	driven, _ := driving(t, d)
	step(t, driven, `{"do":"click","part":"flip"}`)
	if fmt.Sprint(model.tokens) != "[s3cret]" {
		t.Fatalf("the handler read %v (%v)", model.tokens, step(t, driven, `{"do":"read"}`))
	}
	if kept := clarity.ReadSettings("GoLayerTest"); kept["apiToken"] != nil {
		t.Fatalf("the secret went to the file: %v", kept)
	}
}
