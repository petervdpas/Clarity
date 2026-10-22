// The service answers the renderer's commands as the Tauri shell
// does; tested without a window, through Invoke.

package shell

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

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

func TestReferenceOptionsAndCommandsWithoutAWindow(t *testing.T) {
	c, _ := service(t)
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

func TestADesktopWindowOpensAtItsWindowsSizeOrTheStandardOne(t *testing.T) {
	if w, h := size(320, 460); w != 320 || h != 460 {
		t.Fatalf("got %dx%d", w, h)
	}
	if w, h := size(0, 0); w != 1100 || h != 760 {
		t.Fatalf("got %dx%d for no size", w, h)
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
	var opened []string
	c.openURL = func(address string) error {
		opened = append(opened, address)
		return nil
	}
	if got := wire(t, call(t, c, "open_link", `{"address":"https://example.com/rules"}`)); got != `{"value":null}` {
		t.Fatal(got)
	}
	call(t, c, "open_link", `{"address":"mailto:help@example.com"}`)
	if got := wire(t, call(t, c, "open_link", `{"address":"file:///etc/passwd"}`)); got != `{"error":"a link opens only an http, https or mailto address, not file:///etc/passwd"}` {
		t.Fatal(got)
	}
	if strings.Join(opened, " ") != "https://example.com/rules mailto:help@example.com" {
		t.Fatal(opened)
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
