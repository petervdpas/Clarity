// The service answers the renderer's commands as the Tauri shell
// does; tested without a window, through Invoke.

package shell

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
)

// cards is a project with one window, Card, whose open event and save
// handler a test can see.
type cards struct{ log []string }

func (c *cards) Open(db *clarity.Db, window string, with map[string]any) (clarity.WindowState, error) {
	if window != "Card" {
		return clarity.WindowState{}, &clarity.InvalidError{What: "there is no window " + window}
	}
	data := map[string]any{"name": "new"}
	for k, v := range with {
		data[k] = v
	}
	return clarity.NewWindowState(window, data), nil
}

func (c *cards) Handler(window, event string) (string, bool) {
	if window == "Card" && event == "open" {
		return "opened", true
	}
	return "", false
}

func (c *cards) Dispatch(db *clarity.Db, state clarity.WindowState, handler string) (clarity.Reply, error) {
	c.log = append(c.log, state.Window+":"+handler)
	ui := clarity.NewUi(state)
	switch handler {
	case "opened":
		ui.SetCaption("title", clarity.PlainText("Card of "+state.Data["name"].(string)))
	case "save":
		return clarity.Reply{}, clarity.Refuse(clarity.NewText("{name} is taken").With("name", clarity.SlotText("Ada")))
	case "fail":
		return clarity.Reply{}, &clarity.StorageError{What: "locked"}
	case "remember":
		if err := clarity.Save(db, &memo{Text: "kept"}); err != nil {
			return clarity.Reply{}, err
		}
	case "check":
		// A wizard's on next: step 2 is not done.
		if ui.Step("wizard") == 2 {
			return clarity.Reply{}, clarity.Refuse(clarity.PlainText("step 2 is not done"))
		}
	case "addTask":
		// A task added and chosen: its table shows it at once.
		state.Data["tasks"] = append(state.Data["tasks"].([]any), map[string]any{"title": "Zinc", "hours": 1})
		state.Selected["tasksTable"] = len(state.Data["tasks"].([]any)) - 1
		return clarity.NewUi(state).Reply(state.Window, state.Data)
	case "chooseApples":
		state.Selected["tasksTree"] = 2
		return clarity.NewUi(state).Reply(state.Window, state.Data)
	case "jump":
		ui.SetStep("wizard", 3)
	case "crash":
		var none []int64
		_ = none[3]
	case "plan":
		// As a long engine call does: it says how far it is until it is stopped.
		watch := clarity.CurrentWatch()
		for day := int64(1); ; day++ {
			watch.Progress(day, 365)
			select {
			case <-watch.Stop:
				return clarity.Reply{}, &clarity.StorageError{What: "it was stopped"}
			default:
			}
		}
	}
	return ui.Reply(state.Window, state.Data)
}

// Answer renames the card to the answer, after the values written for it.
func (c *cards) Answer(db *clarity.Db, state clarity.WindowState, handler string, values []json.RawMessage) (clarity.Reply, error) {
	c.log = append(c.log, state.Window+":"+handler+" "+string(values[len(values)-1]))
	name, err := clarity.Answer[string](values[len(values)-1])
	if err != nil {
		return clarity.Reply{}, err
	}
	state.Data["name"] = name
	return clarity.NewUi(state).Reply(state.Window, state.Data)
}

// Redraw draws the card's canvas: a dot per letter of the name.
func (c *cards) Redraw(db *clarity.Db, reply clarity.Reply) (clarity.Reply, error) {
	if reply.State.Window != "Card" {
		return reply, nil
	}
	ui := clarity.NewUi(reply.State)
	ui.BeginDrawing("badge")
	if name, ok := reply.State.Data["name"].(string); ok {
		for i := range name {
			ui.Draw("badge", map[string]any{"op": "circle", "x": float64(i * 10), "y": 5.0, "r": 4.0, "color": "accent"})
		}
	}
	drawn, err := ui.Reply(reply.State.Window, reply.State.Data)
	reply.State.Parts = drawn.State.Parts
	return reply, err
}

func (c *cards) Values(db *clarity.Db, entity, field string) ([]string, error) {
	return []string{"Utrecht", "Gent"}, nil
}

func (c *cards) Options(db *clarity.Db, entity string) ([]clarity.RefOption, error) {
	return []clarity.RefOption{{ID: 1, Text: "Ada"}}, nil
}

// Picture is a PNG for card 1's photo, none for card 2.
func (c *cards) Picture(db *clarity.Db, entity string, id int64, field string) ([]byte, error) {
	if entity != "Card" || field != "photo" {
		return nil, &clarity.InvalidError{What: entity + " has no picture " + field}
	}
	if id == 1 {
		return []byte{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A}, nil
	}
	return nil, nil
}

func service(t *testing.T) (*Service, *cards) {
	db, err := clarity.Memory()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { db.Close() })
	if _, err := db.Migrate([][]string{{"CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"}}); err != nil {
		t.Fatal(err)
	}
	windows := &cards{}
	return New(Options{Name: "shop", Title: "Shop", Main: "Card", Model: []byte(`{"project":{}}`), Db: db, Windows: windows}), windows
}

func call(t *testing.T, c *Service, command string, args string) Answer {
	t.Helper()
	return c.Invoke(context.Background(), command, json.RawMessage(args))
}

func wire(t *testing.T, value any) string {
	t.Helper()
	text, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	return string(text)
}

func TestStartGivesTheModelAndTheMainWindow(t *testing.T) {
	c, _ := service(t)
	got := wire(t, call(t, c, "start", `{}`))
	if got != `{"value":{"main":"Card","model":{"project":{}}}}` {
		t.Fatal(got)
	}
}

func TestTheScreenSetsTheLanguageCodeTextsAreTranslatedTo(t *testing.T) {
	c := New(Options{Model: []byte(`{"project":{"locale":{"language":"en"},"translations":{"en":{"draw":"Draw"},"nl":{"draw":"Gelijkspel"}}}}`)})
	defer clarity.SetLanguage(nil)
	if got := wire(t, call(t, c, "set_language", `{"language":"nl"}`)); got != `{"value":null}` {
		t.Fatal(got)
	}
	if got := clarity.TextOf("draw"); got != "Gelijkspel" {
		t.Fatal(got)
	}
	call(t, c, "set_language", `{"language":null}`)
	if got := clarity.TextOf("draw"); got != "Draw" {
		t.Fatal(got)
	}
}

func TestOpeningAWindowRunsItsOpenHandler(t *testing.T) {
	c, windows := service(t)
	answer := call(t, c, "open_window", `{"window":"Card","with":{"name":"Ada"}}`)
	if answer.Error != nil {
		t.Fatal(answer.Error)
	}
	reply := answer.Value.(clarity.Reply)
	if reply.State.Parts["title"].Caption.Show() != "Card of Ada" || len(windows.log) != 1 {
		t.Fatalf("%#v %v", reply, windows.log)
	}
	// A window without the handler of an event answers with its state as it is.
	back := call(t, c, "return_to", wire(t, map[string]any{"state": reply.State}))
	if back.Error != nil || back.Value.(clarity.Reply).State.Data["name"] != "Ada" {
		t.Fatalf("%#v", back)
	}
	// Its canvases are drawn anew, opened, after a handler, or with none.
	for _, r := range []clarity.Reply{reply, back.Value.(clarity.Reply)} {
		if drawing := r.State.Parts["badge"].Drawing; drawing == nil || len(*drawing) != 3 {
			t.Fatalf("drawing: %#v", r.State.Parts["badge"])
		}
	}
	if got := wire(t, call(t, c, "open_window", `{"window":"Nope","with":{}}`)); got != `{"error":"invalid value: there is no window Nope"}` {
		t.Fatal(got)
	}
}

func TestARefusalKeepsItsSlotsAndOtherErrorsAreText(t *testing.T) {
	c, _ := service(t)
	state := `{"state":{"window":"Card","data":{"name":"Ada"}},"handler":"save"}`
	if got := wire(t, call(t, c, "dispatch_event", state)); got != `{"error":{"text":"{name} is taken","values":{"name":{"kind":"text","value":"Ada"}}}}` {
		t.Fatal(got)
	}
	state = `{"state":{"window":"Card","data":{}},"handler":"fail"}`
	if got := wire(t, call(t, c, "dispatch_event", state)); got != `{"error":"storage: locked"}` {
		t.Fatal(got)
	}
}

// Fail safe: a handler that panics, as on a list index out of range, is an
// error the screen shows, and the next command runs.
func TestAHandlerThatPanicsIsAnErrorAndTheNextCommandRuns(t *testing.T) {
	c, _ := service(t)
	state := `{"state":{"window":"Card","data":{}},"handler":"crash"}`
	if got := wire(t, call(t, c, "dispatch_event", state)); !strings.Contains(got, "index out of range") {
		t.Fatal(got)
	}
	state = `{"state":{"window":"Card","data":{}},"handler":"fail"}`
	if got := wire(t, call(t, c, "dispatch_event", state)); got != `{"error":"storage: locked"}` {
		t.Fatal(got)
	}
}

func TestReferenceOptionsAndCommandsWithoutAWindow(t *testing.T) {
	c, _ := service(t)
	if got := wire(t, call(t, c, "field_values", `{"entity":"Customer","field":"city"}`)); got != `{"value":["Utrecht","Gent"]}` {
		t.Errorf("field_values: %s", got)
	}
	if got := wire(t, call(t, c, "ref_options", `{"entity":"Customer"}`)); got != `{"value":[{"id":1,"text":"Ada"}]}` {
		t.Fatal(got)
	}
	// A window opened in place of its own has no start window.
	if got := wire(t, call(t, c, "apart_start", `{}`)); got != `{"value":null}` {
		t.Fatal(got)
	}
	// The app's own events run in a window of no name, with a number like any.
	started := call(t, c, "start_app", `{}`)
	if reply, ok := started.Value.(clarity.Reply); started.Error != nil || !ok || reply.State.Window != "" || reply.State.Instance == 0 || len(reply.Open) != 0 || reply.Close {
		t.Fatal(wire(t, started))
	}
	if got := wire(t, call(t, c, "nonsense", `{}`)); got != `{"error":"there is no command nonsense"}` {
		t.Fatal(got)
	}
}

func TestAPictureIsFetchedOnItsOwnAsADataURL(t *testing.T) {
	c, _ := service(t)
	if got := wire(t, call(t, c, "picture", `{"entity":"Card","id":1,"field":"photo"}`)); got != `{"value":"data:image/png;base64,iVBORw0KGgo="}` {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "picture", `{"entity":"Card","id":2,"field":"photo"}`)); got != `{"value":null}` {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "picture", `{"entity":"Card","id":1,"field":"logo"}`)); got != `{"error":"invalid value: Card has no picture logo"}` {
		t.Fatal(got)
	}
}

func TestAPictureLinkedFromAFileIsReadFromItsStore(t *testing.T) {
	c, _ := service(t)
	store := t.TempDir()
	if err := os.WriteFile(filepath.Join(store, "cover.png"), []byte{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A}, 0o644); err != nil {
		t.Fatal(err)
	}
	if err := c.options.Db.AddStore("photos", store); err != nil {
		t.Fatal(err)
	}
	if got := wire(t, call(t, c, "store_file", `{"store":"photos","path":"cover.png"}`)); got != `{"value":"data:image/png;base64,iVBORw0KGgo="}` {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "store_file", `{"store":"photos","path":"../x.png"}`)); got != `{"error":"invalid value: '../x.png' is not a picture of its file store"}` {
		t.Fatal(got)
	}
}

// A window that closes is reported once its own `on close` has run, so an
// engine with state per window drops what it had of it.
func TestAClosedWindowIsReported(t *testing.T) {
	c, _ := service(t)
	var closed []uint64
	clarity.OnWindowClosed(func(instance uint64) { closed = append(closed, instance) })
	opened := call(t, c, "open_window", `{"window":"Card","with":{"name":"Ada"}}`).Value.(clarity.Reply)
	if answer := call(t, c, "close_window", wire(t, map[string]any{"state": opened.State})); answer.Error != nil {
		t.Fatal(answer.Error)
	}
	if len(closed) != 1 || closed[0] != opened.State.Instance {
		t.Fatalf("closed %v, window %d", closed, opened.State.Instance)
	}
}

func TestALinkOpensInTheBrowserOnlyForWebAndMailAddresses(t *testing.T) {
	c, _ := service(t)
	links := &linkDesktop{}
	c.SetDesktop(links)
	if got := wire(t, call(t, c, "open_link", `{"address":"https://example.com/rules"}`)); got != `{"value":null}` {
		t.Fatal(got)
	}
	call(t, c, "open_link", `{"address":"mailto:help@example.com"}`)
	if got := wire(t, call(t, c, "open_link", `{"address":"file:///etc/passwd"}`)); got != `{"error":"a link opens only an http, https or mailto address, not file:///etc/passwd"}` {
		t.Fatal(got)
	}
	opened := links.opened
	if strings.Join(opened, " ") != "https://example.com/rules mailto:help@example.com" {
		t.Fatal(opened)
	}
}

// An engine beside the screen logs through the shell (DESIGN 5.12): the
// line goes into the application's log, where the backend's engines write.
func TestAnEngineLineGoesIntoTheApplicationsLog(t *testing.T) {
	c, _ := service(t)
	var lines []string
	c.logLine = func(line string) { lines = append(lines, line) }
	if got := wire(t, call(t, c, "log_line", `{"line":"engine Screen: level 2"}`)); got != `{"value":null}` {
		t.Fatal(got)
	}
	if strings.Join(lines, "|") != "engine Screen: level 2" {
		t.Fatal(lines)
	}
}

func TestAnAnswerRunsTheHandlerItGoesToWithItsValues(t *testing.T) {
	c, windows := service(t)
	got := wire(t, call(t, c, "answer_event", `{"state":{"window":"Card","data":{"name":"Ada"}},"handler":"rename","values":[7,"Grace"]}`))
	if !strings.Contains(got, `"name":"Grace"`) || !strings.Contains(got, `"badge"`) {
		t.Fatal("the answer did not run the handler, or the card was not drawn again: " + got)
	}
	if strings.Join(windows.log, ",") != `Card:rename "Grace"` {
		t.Fatal(windows.log)
	}
}

// The databases did not open (DESIGN 5.27): start says what to show and why,
// and connect opens them once the person has kept their connection.
func TestAnUnconnectedStartSaysWhyAndConnectsLater(t *testing.T) {
	c, _ := service(t)
	tries := 0
	c.options.Unconnected = &Unconnected{Window: "Preferences", Problem: clarity.KeyText("unconnectedNotKept")}
	c.options.Connect = func() (*clarity.Db, error) {
		tries++
		if tries == 1 {
			return nil, &clarity.StorageError{What: "still not kept"}
		}
		return clarity.Memory()
	}
	got := wire(t, call(t, c, "start", `{}`))
	if got != `{"value":{"main":"Card","model":{"project":{}},"unconnected":{"handler":null,"problem":{"key":"unconnectedNotKept"},"window":"Preferences"}}}` {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "connect", `{}`)); got != `{"error":{"key":"unconnectedFailed","values":{"problem":{"kind":"text","value":"still not kept"}}}}` {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "connect", `{}`)); got != `{"value":null}` {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "start", `{}`)); got != `{"value":{"main":"Card","model":{"project":{}}}}` {
		t.Fatal(got)
	}
}

// While the databases are not open, the title bar's close of the main window
// goes to the screen, which connects again (DESIGN 5.27); once they are open,
// or the screen ends the application, it closes as any window does.
func TestTheTitleBarsCloseGoesToTheScreenWhileUnconnected(t *testing.T) {
	c, _ := service(t)
	if c.ClosesToScreen() {
		t.Fatal("a connected application closes as usual")
	}
	c.options.Unconnected = &Unconnected{Window: "Preferences", Problem: clarity.KeyText("unconnectedNotKept")}
	c.options.Connect = clarity.Memory
	call(t, c, "start", `{}`)
	if !c.ClosesToScreen() {
		t.Fatal("unconnected, the screen decides")
	}
	call(t, c, "quit_app", `{}`)
	if c.ClosesToScreen() {
		t.Fatal("ending the application closes it")
	}
	call(t, c, "start", `{}`)
	call(t, c, "connect", `{}`)
	if c.ClosesToScreen() {
		t.Fatal("connected, it closes as usual")
	}
}

// linkDesktop is a desktop that keeps the links it was asked to open.
type linkDesktop struct{ opened []string }

func (d *linkDesktop) Command(_ context.Context, command string, a Args) (Answer, bool) {
	if command != "open_link" {
		return Answer{}, false
	}
	d.opened = append(d.opened, a.Address)
	return Answer{}, true
}

func TestWithoutADesktopItsCommandsDoNothingOrSaySo(t *testing.T) {
	c, _ := service(t)
	for _, command := range []string{"set_title", "quit_app", "apart_start", "close_apart", "close_parts"} {
		if got := wire(t, call(t, c, command, `{}`)); got != `{"value":null}` {
			t.Fatal(command, got)
		}
	}
	if got := wire(t, call(t, c, "open_apart", `{"window":"Card"}`)); got != `{"error":"a window of its own needs the desktop"}` {
		t.Fatal(got)
	}
}

// screen keeps the scripts the shell runs on the screen.
type screen struct {
	mu      sync.Mutex
	scripts []string
}

func (s *screen) Command(context.Context, string, Args) (Answer, bool) { return Answer{}, false }

func (s *screen) Eval(_ context.Context, js string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.scripts = append(s.scripts, js)
}

// A long call on the screen (DESIGN 5.12): the handler's engine calls report
// to the screen, at most ten times a second, until the handler ends; Stop
// comes in while it runs, outside the lock the handler holds.
func TestALongHandlerShowsHowFarItIsAndStops(t *testing.T) {
	c := New(Options{Windows: &cards{}})
	shown := &screen{}
	c.SetDesktop(shown)
	state := clarity.WindowState{Window: "Card", Data: map[string]any{"name": "Ada"}}
	done := make(chan Answer)
	go func() { done <- c.invoke(context.Background(), "dispatch_event", Args{State: state, Handler: "plan"}) }()
	// Stop once the screen has heard of it.
	for {
		shown.mu.Lock()
		heard := len(shown.scripts) > 0
		shown.mu.Unlock()
		if heard {
			break
		}
		time.Sleep(time.Millisecond)
	}
	if a := c.invoke(context.Background(), "stop_call", Args{}); a.Error != nil {
		t.Fatal(a.Error)
	}
	answer := <-done
	if why, _ := answer.Error.(string); !strings.Contains(why, "it was stopped") {
		t.Fatalf("answered %+v", answer)
	}
	shown.mu.Lock()
	defer shown.mu.Unlock()
	first, last := shown.scripts[0], shown.scripts[len(shown.scripts)-1]
	if first != `window.dispatchEvent(new CustomEvent("clarity-progress", {detail: {"done":1,"total":365}}))` {
		t.Errorf("first %s", first)
	}
	if last != `window.dispatchEvent(new CustomEvent("clarity-progress", {detail: null}))` {
		t.Errorf("last %s", last)
	}
	// A handler that reports nothing tells the screen nothing.
	shown.scripts = nil
	shown.mu.Unlock()
	c.invoke(context.Background(), "dispatch_event", Args{State: state, Handler: "opened"})
	shown.mu.Lock()
	if len(shown.scripts) != 0 {
		t.Errorf("told %v", shown.scripts)
	}
}

// memo is a record a handler keeps, for undo.
type memo struct {
	ID   int64
	Text string
}

func (*memo) Table() string     { return "memo" }
func (*memo) Columns() []string { return []string{"text"} }
func (m *memo) GetID() int64    { return m.ID }
func (m *memo) SetID(id int64)  { m.ID = id }
func (m *memo) Values() []any   { return []any{m.Text} }
func (m *memo) Load(id int64, v []any) error {
	r := clarity.NewReader(v)
	*m = memo{id, r.Text(0, "text")}
	return r.Err()
}

// Undo (DESIGN 5.29): the runtime keeps each window's edits. An edit whose
// handler saved is taken back with what it saved and done again; a field
// the person changed goes back to what the window's last reply had.
func TestTheRuntimeKeepsTheEditsAndCtrlZTakesThemBack(t *testing.T) {
	c, _ := service(t)
	db := c.options.Db
	// A reply: the edits start from the window as it is.
	call(t, c, "dispatch_event", `{"state":{"window":"Card","instance":7,"data":{"name":"Ada"}},"handler":"noop"}`)
	// The person typed a name, then pressed a part whose handler saves: two edits of the form.
	call(t, c, "edit_event", `{"state":{"window":"Card","instance":7,"data":{"name":"Ann"}},"part":"nameBox","scope":"form","ends":true}`)
	call(t, c, "edit_event", `{"state":{"window":"Card","instance":7,"data":{"name":"Ann"}},"handler":"remember","part":"saveBox","scope":"form","ends":true}`)
	if memos, _ := clarity.All[memo](db); len(memos) != 1 {
		t.Fatal(memos)
	}
	back := `{"state":{"window":"Card","instance":7,"data":{"name":"Ann"}},"scope":"form","back":true}`
	if got := wire(t, call(t, c, "undo_edit", back)); strings.Contains(got, "error") {
		t.Fatal(got)
	}
	if memos, _ := clarity.All[memo](db); len(memos) != 0 {
		t.Fatal("what the handler saved is taken back", memos)
	}
	if got := wire(t, call(t, c, "undo_edit", back)); !strings.Contains(got, `"name":"Ada"`) {
		t.Fatal("the typed name is taken back:", got)
	}
	again := `{"state":{"window":"Card","instance":7,"data":{"name":"Ada"}},"scope":"form","back":false}`
	if got := wire(t, call(t, c, "undo_edit", again)); !strings.Contains(got, `"name":"Ann"`) {
		t.Fatal("the typed name is done again:", got)
	}
	call(t, c, "undo_edit", again)
	if memos, _ := clarity.All[memo](db); len(memos) != 1 {
		t.Fatal("what the handler saved is done again", memos)
	}
	// A closed window's edits go.
	call(t, c, "close_window", `{"state":{"window":"Card","instance":7,"data":{"name":"Ann"}}}`)
	if got := wire(t, call(t, c, "undo_edit", back)); strings.Contains(got, `"name":"Ada"`) {
		t.Fatal("a closed window kept its edits:", got)
	}
}

// asked is a command's reply as JSON, failing on an error.
func asked(t *testing.T, c *Service, command string, args any) map[string]any {
	t.Helper()
	answer := call(t, c, command, wire(t, args))
	if answer.Error != nil {
		t.Fatalf("%s: %v", command, wire(t, answer))
	}
	var reply map[string]any
	if err := json.Unmarshal([]byte(wire(t, answer.Value)), &reply); err != nil {
		t.Fatal(err)
	}
	return reply
}

func dataOf(reply map[string]any) map[string]any {
	return reply["state"].(map[string]any)["data"].(map[string]any)
}

// Reordering (DESIGN 1.7, 5.30): the screen sends the move, the runtime gives
// the list its order and numbers, chooses the moved row and runs the handler,
// as one edit that Ctrl+Z takes back.
func TestTheRuntimeReordersAMovedRow(t *testing.T) {
	c, windows := service(t)
	db := c.options.Db
	songs := []any{map[string]any{"id": 1, "place": 1}, map[string]any{"id": 2, "place": 2}, map[string]any{"id": 3, "place": 3}}
	state := map[string]any{"window": "Card", "instance": 21, "data": map[string]any{"songs": songs}}
	call(t, c, "dispatch_event", wire(t, map[string]any{"state": state, "handler": "noop"}))
	move := map[string]any{"state": state, "part": "songsTable", "scope": "form", "list": "songs", "by": "place", "parent": nil,
		"move": map[string]any{"row": 2, "where": "before", "target": 0}, "handler": "remember"}
	reply := asked(t, c, "reorder_event", move)
	if got := wire(t, dataOf(reply)["songs"]); got != `[{"id":3,"place":1},{"id":1,"place":2},{"id":2,"place":3}]` {
		t.Fatal("the new order and numbers:", got)
	}
	if got := wire(t, reply["state"].(map[string]any)["selected"]); got != `{"songsTable":0}` {
		t.Fatal("the moved row is chosen:", got)
	}
	if memos, _ := clarity.All[memo](db); len(memos) != 1 || reply["step"] == nil {
		t.Fatal("the handler ran as a step", memos, reply["step"])
	}
	moved := map[string]any{"window": "Card", "instance": 21, "data": dataOf(reply)}
	back := asked(t, c, "undo_edit", map[string]any{"state": moved, "scope": "form", "back": true})
	if got := wire(t, dataOf(back)["songs"]); got != wire(t, songs) {
		t.Fatal("Ctrl+Z takes the move back:", got)
	}
	if memos, _ := clarity.All[memo](db); len(memos) != 0 {
		t.Fatal("and what its handler saved", memos)
	}
	// A refused move answers the window as it is, running nothing.
	ran := len(windows.log)
	move["move"] = map[string]any{"row": 0, "key": "up"}
	refused := asked(t, c, "reorder_event", move)
	if wire(t, dataOf(refused)["songs"]) != wire(t, songs) || len(windows.log) != ran {
		t.Fatal("a refused move changed the list or ran its handler", wire(t, refused), windows.log[ran:])
	}
	// In a tree, a row's parent goes along: right makes it the last under the row before it.
	tree := []any{map[string]any{"id": 1, "up": nil, "place": 1}, map[string]any{"id": 2, "up": nil, "place": 2}}
	branch := asked(t, c, "reorder_event", map[string]any{"state": map[string]any{"window": "Card", "data": map[string]any{"tree": tree}},
		"part": "treeView", "list": "tree", "by": "place", "parent": "up", "move": map[string]any{"row": 1, "key": "right"}})
	if got := wire(t, dataOf(branch)["tree"]); got != `[{"id":1,"place":1,"up":null},{"id":2,"place":1,"up":1}]` {
		t.Fatal("the tree:", got)
	}
}

// Canvas modes (DESIGN 5.31): the screen sends where the person dropped an
// object, the runtime snaps it, chooses it and runs on change, as one edit
// Ctrl+Z takes back; a vector canvas keeps its drawing as SVG.
func TestTheRuntimeMovesAnObjectAndEditsADrawing(t *testing.T) {
	c, windows := service(t)
	db := c.options.Db
	types := []any{map[string]any{"id": 1, "x": 0, "y": 0}, map[string]any{"id": 2, "x": 40, "y": 40}}
	state := map[string]any{"window": "Card", "instance": 31, "data": map[string]any{"types": types, "note": map[string]any{"drawing": ""}}}
	call(t, c, "dispatch_event", wire(t, map[string]any{"state": state, "handler": "noop"}))
	drop := map[string]any{"state": state, "part": "diagram", "scope": "diagram", "mode": "objects", "list": "types", "at": []any{"x", "y"},
		"grid": 20, "snap": true, "action": map[string]any{"move": map[string]any{"id": 2, "x": 67, "y": 31}}, "handler": "remember"}
	reply := asked(t, c, "canvas_event", drop)
	if got := wire(t, dataOf(reply)["types"]); got != `[{"id":1,"x":0,"y":0},{"id":2,"x":60,"y":40}]` {
		t.Fatal("the object, snapped:", got)
	}
	if got := wire(t, reply["state"].(map[string]any)["selected"]); got != `{"diagram":1}` {
		t.Fatal("the moved object is chosen:", got)
	}
	if memos, _ := clarity.All[memo](db); len(memos) != 1 || reply["step"] == nil {
		t.Fatal("on change ran as a step", memos, reply["step"])
	}
	moved := map[string]any{"window": "Card", "instance": 31, "data": dataOf(reply)}
	back := asked(t, c, "undo_edit", map[string]any{"state": moved, "scope": "diagram", "back": true})
	if got := wire(t, dataOf(back)["types"]); got != wire(t, types) {
		t.Fatal("Ctrl+Z takes the move back:", got)
	}
	// Dropped where it is: refused, running nothing.
	ran := len(windows.log)
	drop["action"] = map[string]any{"move": map[string]any{"id": 1, "x": 5, "y": -4}}
	refused := asked(t, c, "canvas_event", drop)
	if refused["refused"] != true || len(windows.log) != ran {
		t.Fatal("a drop where it is ran its handler", wire(t, refused))
	}
	drawn := asked(t, c, "canvas_event", map[string]any{"state": state, "part": "drawing", "scope": "drawing", "mode": "vector", "path": "note.drawing",
		"units": []any{400, 300}, "action": map[string]any{"add": map[string]any{"op": "rect", "x": 1, "y": 2, "width": 3, "height": 4, "color": "#ff0000"}}})
	want := "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 300\">\n<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#ff0000\"/>\n</svg>\n"
	if got := dataOf(drawn)["note"].(map[string]any)["drawing"]; got != want {
		t.Fatal("the drawing:", got)
	}
}

// A typed item for a list field (U1c): the runtime adds it, or refuses it
// and the list stays.
func TestTheRuntimeAddsATypedItemOrRefusesIt(t *testing.T) {
	c, windows := service(t)
	state := map[string]any{"window": "Card", "instance": 22, "data": map[string]any{"album": map[string]any{"tags": []any{"jazz"}, "years": []any{1999}}}}
	call(t, c, "dispatch_event", wire(t, map[string]any{"state": state, "handler": "noop"}))
	added := asked(t, c, "item_event", map[string]any{"state": state, "part": "tagsBox", "scope": "form", "path": "album.tags", "item": "text", "typed": "  live "})
	if got := wire(t, dataOf(added)["album"]); got != `{"tags":["jazz","live"],"years":[1999]}` || added["refused"] != nil {
		t.Fatal(got, added["refused"])
	}
	back := asked(t, c, "undo_edit", map[string]any{"state": map[string]any{"window": "Card", "instance": 22, "data": dataOf(added)}, "scope": "form", "back": true})
	if got := wire(t, dataOf(back)["album"]); got != `{"tags":["jazz"],"years":[1999]}` {
		t.Fatal("Ctrl+Z takes the item back:", got)
	}
	ran := len(windows.log)
	refused := asked(t, c, "item_event", map[string]any{"state": state, "part": "yearsBox", "path": "album.years", "item": "integer", "typed": "twelve", "handler": "remember"})
	if refused["refused"] != true || wire(t, dataOf(refused)["album"]) != `{"tags":["jazz"],"years":[1999]}` || len(windows.log) != ran {
		t.Fatal("twelve is no integer", wire(t, refused))
	}
	// An enum takes only its values.
	enum := asked(t, c, "item_event", map[string]any{"state": state, "part": "tagsBox", "path": "album.tags", "item": "enum", "values": []string{"Jazz"}, "typed": "Polka"})
	if enum["refused"] != true {
		t.Fatal("Polka is no value of the enum", wire(t, enum))
	}
}

// A slider's or spin box's value after a key, a drag or typed text: within
// its range, in its steps, as the runtime decides.
func TestTheRuntimeMovesARangedValue(t *testing.T) {
	c, _ := service(t)
	state := map[string]any{"window": "Card", "data": map[string]any{"volume": 5, "price": "0.2"}}
	volume := map[string]any{"state": state, "part": "volumeSlider", "path": "volume", "min": "0", "max": "10", "step": "1", "whole": true}
	for _, at := range []struct {
		name, value string
		want        string
	}{{"key", "up", "6"}, {"at", "7.6", "8"}, {"typed", "120", "10"}, {"key", "home", "0"}} {
		args := map[string]any{}
		for k, v := range volume {
			args[k] = v
		}
		args[at.name] = at.value
		if got := wire(t, dataOf(asked(t, c, "range_event", args))["volume"]); got != at.want {
			t.Errorf("%s %s: %s, want %s", at.name, at.value, got, at.want)
		}
	}
	// A decimal stays exact, as text.
	price := asked(t, c, "range_event", map[string]any{"state": state, "part": "priceBox", "path": "price", "min": nil, "max": "1", "step": "0.1", "whole": false, "key": "up"})
	if got := wire(t, dataOf(price)["price"]); got != `"0.3"` {
		t.Fatal(got)
	}
	// Typed text that is no number is refused, the value as it was.
	bad := asked(t, c, "range_event", map[string]any{"state": state, "part": "priceBox", "path": "price", "step": "0.1", "typed": "lots"})
	if bad["refused"] != true || dataOf(bad)["price"] != "0.2" {
		t.Fatal(wire(t, bad))
	}
}

// A wizard (DESIGN 5.15): Next runs on next on the step shown and moves on,
// unless it refused or set the step; Back runs nothing; Finish runs on finish.
func TestTheRuntimeWalksAWizard(t *testing.T) {
	c, windows := service(t)
	at := func(step any) map[string]any {
		parts := map[string]any{}
		if step != nil {
			parts["wizard"] = map[string]any{"step": step}
		}
		return map[string]any{"window": "Card", "data": map[string]any{"name": "Ada"}, "parts": parts}
	}
	stepOf := func(reply map[string]any) any {
		return reply["state"].(map[string]any)["parts"].(map[string]any)["wizard"].(map[string]any)["step"]
	}
	wizard := func(state map[string]any, button, next string) map[string]any {
		return map[string]any{"state": state, "part": "wizard", "button": button, "steps": 3, "next": next, "finish": "remember"}
	}
	if got := stepOf(asked(t, c, "step_event", wizard(at(nil), "next", "check"))); got != 2.0 {
		t.Fatal("next moves on:", got)
	}
	if windows.log[len(windows.log)-1] != "Card:check" {
		t.Fatal(windows.log)
	}
	if got := wire(t, call(t, c, "step_event", wire(t, wizard(at(2), "next", "check")))); !strings.Contains(got, "step 2 is not done") {
		t.Fatal("a refusal keeps the step:", got)
	}
	if got := stepOf(asked(t, c, "step_event", wizard(at(1), "next", "jump"))); got != 3.0 {
		t.Fatal("the step the handler set wins:", got)
	}
	ran := len(windows.log)
	if got := stepOf(asked(t, c, "step_event", wizard(at(2), "back", "check"))); got != 1.0 || len(windows.log) != ran {
		t.Fatal("back goes back and runs nothing:", got)
	}
	if got := stepOf(asked(t, c, "step_event", wizard(at(nil), "next", ""))); got != 2.0 {
		t.Fatal("without on next, next just moves on:", got)
	}
	finished := asked(t, c, "step_event", wizard(at(3), "finish", "check"))
	if stepOf(finished) != 3.0 || windows.log[len(windows.log)-1] != "Card:remember" || finished["step"] == nil {
		t.Fatal("finish runs on finish:", wire(t, finished))
	}
}

// What a dropdown offers for typed text (U1c), as the runtime finds it.
func TestTheRuntimeFindsADropdownsOptions(t *testing.T) {
	c, _ := service(t)
	if got := wire(t, call(t, c, "find_options", `{"kind":"typed","entity":"Customer","field":"city","typed":" ut"}`)); got != `{"value":["Utrecht"]}` {
		t.Error(got)
	}
	if got := wire(t, call(t, c, "find_options", `{"kind":"search","entity":"Customer","show":["name"],"typed":"AD"}`)); got != `{"value":[{"id":1,"text":"Ada"}]}` {
		t.Error(got)
	}
	if got := wire(t, call(t, c, "find_options", `{"kind":"search","entity":"Customer","typed":"rome"}`)); got != `{"value":[]}` {
		t.Error(got)
	}
}

// tasksWindow is a window with a table of tasks shown two to a page, a tree
// of them and a chart of their hours.
func tasksWindow() map[string]any {
	tasks := []any{
		map[string]any{"id": 1, "title": "Milk", "hours": 9, "up": nil},
		map[string]any{"id": 2, "title": "bread", "hours": 10, "up": 1},
		map[string]any{"id": 3, "title": "Apples", "hours": 9, "up": 2},
	}
	views := map[string]any{
		"tasksTable": map[string]any{"kind": "table", "sortable": true, "list": "tasks", "columns": []any{map[string]any{"field": "title", "kind": "text"}, map[string]any{"field": "hours", "kind": "number"}}, "order": nil, "page": 2, "at": 1},
		"tasksTree":  map[string]any{"kind": "tree", "list": "tasks", "parent": "up", "key": "id", "open": []any{}},
		"hoursChart": map[string]any{"kind": "chart", "list": "tasks", "by": "hours", "byKind": "number", "measure": "count", "of": "", "values": []any{}},
	}
	return map[string]any{"window": "Card", "data": map[string]any{"tasks": tasks}, "views": views, "selected": map[string]any{}}
}

func viewOf(reply map[string]any, part string) string {
	views, _ := reply["state"].(map[string]any)["views"].(map[string]any)
	view, _ := views[part].(map[string]any)
	shown := map[string]any{}
	for _, name := range []string{"shown", "pages", "at", "order", "open", "groups"} {
		if v, ok := view[name]; ok {
			shown[name] = v
		}
	}
	text, _ := json.Marshal(shown)
	return string(text)
}

// The shown rows (DESIGN 1.7): after every reply the runtime fills in what
// each table, tree and chart shows, so a handler's new row shows at once.
func TestEveryReplyFillsTheShownRows(t *testing.T) {
	c, _ := service(t)
	reply := asked(t, c, "dispatch_event", map[string]any{"state": tasksWindow(), "handler": "noop"})
	if got := viewOf(reply, "tasksTable"); got != `{"at":1,"order":null,"pages":{"count":3,"from":1,"page":1,"pages":2,"to":2},"shown":[0,1]}` {
		t.Error("table:", got)
	}
	if got := viewOf(reply, "tasksTree"); got != `{"open":[],"shown":[[0,0,true]]}` {
		t.Error("tree:", got)
	}
	if got := viewOf(reply, "hoursChart"); got != `{"groups":[{"key":9,"value":2},{"key":10,"value":1}]}` {
		t.Error("chart:", got)
	}
	// A handler adds a task and chooses it: the table turns to its page, the tree opens above it.
	state := tasksWindow()
	state["data"].(map[string]any)["tasks"] = append(state["data"].(map[string]any)["tasks"].([]any), map[string]any{"id": 4, "title": "Tea", "hours": 2, "up": 3})
	added := asked(t, c, "dispatch_event", map[string]any{"state": state, "handler": "addTask"})
	if got := viewOf(added, "tasksTable"); got != `{"at":3,"order":null,"pages":{"count":5,"from":5,"page":3,"pages":3,"to":5},"shown":[4]}` {
		t.Error("table after the handler:", got)
	}
	picked := asked(t, c, "dispatch_event", map[string]any{"state": tasksWindow(), "handler": "chooseApples"})
	if got := viewOf(picked, "tasksTree"); got != `{"open":[1,2],"shown":[[0,0,true],[1,1,true],[2,2,false]]}` {
		t.Error("the tree opens above its chosen row:", got)
	}
}

// view_event: the person sorted, turned a page, or opened or closed a tree
// row; the runtime changes that view, runs nothing and makes no edit.
func TestAViewEventChangesHowAPartShowsItsRows(t *testing.T) {
	c, windows := service(t)
	state := tasksWindow()
	state["instance"] = 31
	event := func(part string, action map[string]any) map[string]any {
		reply := asked(t, c, "view_event", map[string]any{"state": state, "part": part, "action": action})
		state = reply["state"].(map[string]any)
		return reply
	}
	ran := len(windows.log)
	if got := viewOf(event("tasksTable", map[string]any{"page": 2}), "tasksTable"); got != `{"at":2,"order":null,"pages":{"count":3,"from":3,"page":2,"pages":2,"to":3},"shown":[2]}` {
		t.Error("page 2:", got)
	}
	// Sorting goes back to the first page; the same header again sorts the other way.
	if got := viewOf(event("tasksTable", map[string]any{"sort": "title"}), "tasksTable"); got != `{"at":1,"order":{"desc":false,"field":"title"},"pages":{"count":3,"from":1,"page":1,"pages":2,"to":2},"shown":[2,1]}` {
		t.Error("sorted:", got)
	}
	if got := viewOf(event("tasksTable", map[string]any{"sort": "title"}), "tasksTable"); got != `{"at":1,"order":{"desc":true,"field":"title"},"pages":{"count":3,"from":1,"page":1,"pages":2,"to":2},"shown":[0,1]}` {
		t.Error("the other way:", got)
	}
	if got := viewOf(event("tasksTree", map[string]any{"open": 1}), "tasksTree"); got != `{"open":[1],"shown":[[0,0,true],[1,1,true]]}` {
		t.Error("opened:", got)
	}
	if got := viewOf(event("tasksTree", map[string]any{"close": 1}), "tasksTree"); got != `{"open":[],"shown":[[0,0,true]]}` {
		t.Error("closed:", got)
	}
	if len(windows.log) != ran {
		t.Error("a view event ran a handler", windows.log[ran:])
	}
	// Never an edit: Ctrl+Z has nothing to take back.
	if got := wire(t, call(t, c, "undo_edit", wire(t, map[string]any{"state": state, "scope": "tasksTable", "back": true}))); strings.Contains(got, `"desc":false`) {
		t.Error("a view event was an edit:", got)
	}
	if got := wire(t, call(t, c, "view_event", wire(t, map[string]any{"state": state, "part": "nope", "action": map[string]any{"page": 1}}))); !strings.Contains(got, "error") {
		t.Error("a part without a view:", got)
	}
}

// readonly (DESIGN 1.7): the runtime takes back what the screen changed, as
// the window's last reply had it, runs no handler, records no edit, and
// answers refused.
func TestAReadonlyPartsChangeIsTakenBack(t *testing.T) {
	c, windows := service(t)
	db := c.options.Db
	start := map[string]any{"name": "Ada", "volume": 5, "tags": []any{"jazz"}, "songs": []any{map[string]any{"id": 1, "place": 1}, map[string]any{"id": 2, "place": 2}}}
	call(t, c, "dispatch_event", wire(t, map[string]any{"state": map[string]any{"window": "Card", "instance": 41, "data": start}, "handler": "noop"}))
	changed := map[string]any{"window": "Card", "instance": 41, "data": map[string]any{"name": "Ann", "volume": 5, "tags": []any{"jazz"}, "songs": start["songs"]}}
	ran := len(windows.log)
	for _, ask := range []struct {
		command string
		args    map[string]any
	}{
		{"edit_event", map[string]any{"ends": true}},
		{"reorder_event", map[string]any{"list": "songs", "by": "place", "move": map[string]any{"row": 1, "key": "up"}}},
		{"item_event", map[string]any{"path": "tags", "item": "text", "typed": "live"}},
		{"range_event", map[string]any{"path": "volume", "min": "0", "max": "10", "step": "1", "whole": true, "key": "up"}},
	} {
		args := map[string]any{"state": changed, "part": "box", "scope": "form", "handler": "remember", "readonly": true}
		for k, v := range ask.args {
			args[k] = v
		}
		reply := asked(t, c, ask.command, args)
		if reply["refused"] != true || wire(t, dataOf(reply)) != wire(t, start) {
			t.Errorf("%s: %s", ask.command, wire(t, reply))
		}
	}
	if memos, _ := clarity.All[memo](db); len(memos) != 0 || len(windows.log) != ran {
		t.Fatal("a readonly part ran its handler", memos, windows.log[ran:])
	}
	if got := wire(t, call(t, c, "undo_edit", wire(t, map[string]any{"state": changed, "scope": "form", "back": true}))); !strings.Contains(got, `"name":"Ann"`) {
		t.Fatal("a readonly change was recorded as an edit:", got)
	}
}

// editable (DESIGN 1.7): a table's edit keeps to the columns its cells edit;
// a change to any other field of its rows is taken back.
func TestATablesEditKeepsToItsEditableColumns(t *testing.T) {
	c, _ := service(t)
	start := map[string]any{"tasks": []any{map[string]any{"id": 1, "title": "Milk", "hours": 9}}}
	call(t, c, "dispatch_event", wire(t, map[string]any{"state": map[string]any{"window": "Card", "instance": 42, "data": start}, "handler": "noop"}))
	edit := func(row map[string]any) map[string]any {
		state := map[string]any{"window": "Card", "instance": 42, "data": map[string]any{"tasks": []any{row}}}
		return asked(t, c, "edit_event", map[string]any{"state": state, "part": "tasksTable", "scope": "tasksTable", "ends": true, "path": "tasks", "editable": []string{"hours"}})
	}
	if reply := edit(map[string]any{"id": 1, "title": "Tea", "hours": 9}); reply["refused"] != true || wire(t, dataOf(reply)) != wire(t, start) {
		t.Error("a title that is not editable:", wire(t, reply))
	}
	if reply := edit(map[string]any{"id": 1, "title": "Milk", "hours": 12}); reply["refused"] != nil || wire(t, dataOf(reply)["tasks"]) != `[{"hours":12,"id":1,"title":"Milk"}]` {
		t.Error("hours are editable:", wire(t, reply))
	}
}

// sortable (DESIGN 1.7): a header click on a table that is not sortable changes nothing.
func TestATableThatIsNotSortableKeepsItsOrder(t *testing.T) {
	c, _ := service(t)
	state := tasksWindow()
	state["views"].(map[string]any)["tasksTable"].(map[string]any)["sortable"] = false
	reply := asked(t, c, "view_event", map[string]any{"state": state, "part": "tasksTable", "action": map[string]any{"sort": "title"}})
	if got := viewOf(reply, "tasksTable"); !strings.Contains(got, `"order":null`) || !strings.Contains(got, `"shown":[0,1]`) {
		t.Error(got)
	}
}

// A chart of a list of numbers (DESIGN 1.7, points): its last points
// numbers, decimals as their value; what is no number is left out.
func TestANumbersViewShowsTheLastPoints(t *testing.T) {
	c, _ := service(t)
	views := map[string]any{
		"last": map[string]any{"kind": "numbers", "list": "readings", "points": 2},
		"all":  map[string]any{"kind": "numbers", "list": "readings", "points": nil},
	}
	state := map[string]any{"window": "Card", "data": map[string]any{"readings": []any{1, "2.5", "", nil, "x", 4}}, "views": views}
	reply := asked(t, c, "dispatch_event", map[string]any{"state": state, "handler": "noop"})
	shown := reply["state"].(map[string]any)["views"].(map[string]any)
	if got := wire(t, shown["last"].(map[string]any)["numbers"]); got != `[2.5,4]` {
		t.Error("last:", got)
	}
	if got := wire(t, shown["all"].(map[string]any)["numbers"]); got != `[1,2.5,4]` {
		t.Error("all:", got)
	}
}
