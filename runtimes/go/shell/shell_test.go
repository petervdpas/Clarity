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

// Undo (DESIGN 5.29): a handler that changed the database answers with its
// step, which undo_step takes back and does again, drawing the window anew.
func TestAHandlerAnswersWithItsStepAndUndoStepTakesItBack(t *testing.T) {
	c, _ := service(t)
	db := c.options.Db
	state := `{"state":{"window":"Card","data":{}},"handler":"remember"}`
	got := wire(t, call(t, c, "dispatch_event", state))
	if !strings.Contains(got, `"step":1`) {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "dispatch_event", `{"state":{"window":"Card","data":{}},"handler":"fail"}`)); strings.Contains(got, `"step"`) {
		t.Fatal(got)
	}
	if got := wire(t, call(t, c, "undo_step", `{"state":{"window":"Card","data":{}},"step":1,"back":true}`)); strings.Contains(got, "error") {
		t.Fatal(got)
	}
	if memos, _ := clarity.All[memo](db); len(memos) != 0 {
		t.Fatal(memos)
	}
	call(t, c, "undo_step", `{"state":{"window":"Card","data":{}},"step":1,"back":false}`)
	if memos, _ := clarity.All[memo](db); len(memos) != 1 {
		t.Fatal(memos)
	}
}
