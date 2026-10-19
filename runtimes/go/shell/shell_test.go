// The service answers the renderer's commands as the Tauri shell
// does; tested without a window, through Invoke.

package shell

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
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
	if got := wire(t, call(t, c, "start_app", `{}`)); got != `{"value":{"state":{"window":"","data":{},"parts":{},"selected":{}},"messages":[],"open":[],"close":false}}` {
		t.Fatal(got)
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
