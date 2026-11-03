// The program over the wire (program.go, serve.go): what Clarity.UI asks,
// on the desktop and from a browser, answered with the service and the
// program's own long calls.

package shell

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"testing/fstest"
	"time"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
)

func TestALongCallIsWatchedOnTheThreadItsHandlerRunsOn(t *testing.T) {
	c, model := service(t)
	p := NewProgram(ProgramOptions{Options: c.Options(), Watches: true})
	if got := string(p.Answer("watches", nil, 1)); got != `{"value":true}` {
		t.Fatalf("watches: %s", got)
	}
	state := `{"state":{"window":"Card","instance":1,"data":{"name":"Ann"}},"handler":"long"}`
	// The handler runs on thread 9 as watch 4, and says how far it is.
	p.Answer("start_watch", []byte(`{"id":4}`), 9)
	if got := string(p.Answer("watch_progress", []byte(`{"id":4}`), 1)); got != `{"value":null}` {
		t.Fatalf("progress before any: %s", got)
	}
	p.Answer("dispatch_event", []byte(state), 9)
	if got := string(p.Answer("watch_progress", []byte(`{"id":4}`), 1)); got != `{"value":[3,10]}` {
		t.Fatalf("progress: %s", got)
	}
	// Stop from the screen's thread closes the handler's Stop.
	p.Answer("stop_watch", []byte(`{"id":4}`), 1)
	select {
	case <-model.watched[len(model.watched)-1].Stop:
	default:
		t.Fatal("Stop did not reach the handler's watch")
	}
	p.Answer("end_watch", []byte(`{"id":4}`), 9)
	// A handler on another thread, with no watch, runs unwatched.
	p.Answer("dispatch_event", []byte(state), 2)
	if model.watched[len(model.watched)-1].Stop != nil {
		t.Fatal("a handler without a watch was watched")
	}
	// A Stop that comes before its watch starts holds once it starts; one of a watch that ended stops nothing.
	p.Answer("stop_watch", []byte(`{"id":5}`), 1)
	p.Answer("start_watch", []byte(`{"id":5}`), 9)
	p.Answer("dispatch_event", []byte(state), 9)
	select {
	case <-model.watched[len(model.watched)-1].Stop:
	default:
		t.Fatal("an early Stop did not hold")
	}
	p.Answer("end_watch", []byte(`{"id":5}`), 9)
	p.Answer("stop_watch", []byte(`{"id":5}`), 1)
	if len(p.early) != 0 {
		t.Fatalf("a Stop of an ended watch was kept: %v", p.early)
	}
}

// A browser's page asks the same program over HTTP (round B of DESIGN 7.9):
// the server serves the browser files and answers the wire, the library's
// commands with their arguments; what the program does not answer is an
// error, which the library reads as its default.
func TestTheServerServesTheBrowserFilesAndAnswersTheWire(t *testing.T) {
	c, _ := service(t)
	site := fstest.MapFS{
		"index.html":         {Data: []byte("<html><head><title>Shop</title></head><body><canvas id=\"clarity-ui\"></canvas></body></html>")},
		"clarity-ui.wasm":    {Data: []byte("\x00asm")},
		"clarity-ui-page.js": {Data: []byte("// the page")},
		"clarity/files.json": {Data: []byte(`{"title":"Shop"}`)},
	}
	options := c.Options()
	options.Assets = site
	server := httptest.NewServer(NewProgram(ProgramOptions{Options: options, Facts: []byte(`{"fields":{}}`)}).Handler())
	defer server.Close()

	get := func(path string) (int, string, string) {
		reply, err := http.Get(server.URL + path)
		if err != nil {
			t.Fatal(err)
		}
		defer reply.Body.Close()
		body, _ := io.ReadAll(reply.Body)
		return reply.StatusCode, reply.Header.Get("Content-Type"), string(body)
	}
	if status, _, page := get("/"); status != 200 || !strings.Contains(page, `<canvas id="clarity-ui">`) || !strings.Contains(page, `window.clarityServer = "/clarity/invoke"`) {
		t.Fatalf("%d %s", status, page)
	}
	if status, kind, _ := get("/clarity-ui.wasm"); status != 200 || kind != "application/wasm" {
		t.Fatalf("the wasm: %d %s", status, kind)
	}
	if status, kind, _ := get("/clarity-ui-page.js"); status != 200 || !strings.HasPrefix(kind, "text/javascript") {
		t.Fatalf("the page script: %d %s", status, kind)
	}
	if status, _, files := get("/clarity/files.json"); status != 200 || files != `{"title":"Shop"}` {
		t.Fatalf("files.json: %d %s", status, files)
	}
	post := func(body string) string {
		reply, err := http.Post(server.URL+"/clarity/invoke", "application/json", strings.NewReader(body))
		if err != nil {
			t.Fatal(err)
		}
		defer reply.Body.Close()
		answer, _ := io.ReadAll(reply.Body)
		return strings.TrimSpace(string(answer))
	}
	for _, c := range [][2]string{
		{`{"command": "started", "args": {}}`, `{"value":{"main":"Card","unconnected":null}}`},
		{`{"command": "facts", "args": {}}`, `{"value":{"fields":{}}}`},
		{`{"command": "watches", "args": {}}`, `{"value":false}`},
		{`{"command": "engine_open", "args": {"name": "Sky"}}`, `{"error":"there is no command engine_open"}`},
		{`{"command": "nonsense", "args": {}}`, `{"error":"there is no command nonsense"}`},
	} {
		if got := post(c[0]); got != c[1] {
			t.Fatalf("%s: %s, want %s", c[0], got, c[1])
		}
	}
	opened := post(`{"command": "open_window", "args": {"window": "Card", "with": {"name": "Ada"}}}`)
	if !strings.Contains(opened, `"window":"Card"`) || !strings.Contains(opened, `"name":"Ada"`) {
		t.Fatal(opened)
	}
	if got := post(`not json`); !strings.HasPrefix(got, `{"error":`) {
		t.Fatal(got)
	}
	reply, err := http.Get(server.URL + "/clarity/invoke")
	if err != nil {
		t.Fatal(err)
	}
	reply.Body.Close()
	if reply.StatusCode != http.StatusMethodNotAllowed {
		t.Fatalf("a GET of the wire: %d", reply.StatusCode)
	}
}

// A handler's request that names a watch runs under it (round B: the
// browser sends no start_watch); while it is open another request asks how
// far it is and stops it, and a Stop that came first holds.
func TestAWatchBelongsToTheRequestThatNamesIt(t *testing.T) {
	c, model := service(t)
	options := c.Options()
	program := NewProgram(ProgramOptions{Options: options, Watches: true})
	server := httptest.NewServer(program.Handler())
	defer server.Close()
	post := func(body string) string {
		reply, err := http.Post(server.URL+"/clarity/invoke", "application/json", strings.NewReader(body))
		if err != nil {
			t.Error(err)
			return ""
		}
		defer reply.Body.Close()
		answer, _ := io.ReadAll(reply.Body)
		return strings.TrimSpace(string(answer))
	}
	if got := post(`{"command": "watches", "args": {}}`); got != `{"value":true}` {
		t.Fatal(got)
	}
	// plan runs until it is stopped, saying how far it is.
	done := make(chan string)
	go func() {
		done <- post(`{"command": "dispatch_event", "args": {"state": {"window": "Card", "instance": 1, "data": {"name": "Ann"}}, "handler": "plan", "watch": 7}}`)
	}()
	var progress string
	for i := 0; i < 200; i++ {
		if progress = post(`{"command": "watch_progress", "args": {"id": 7}}`); progress != `{"value":null}` {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if !strings.HasPrefix(progress, `{"value":[`) || !strings.HasSuffix(progress, `,365]}`) {
		t.Fatalf("progress while the handler runs: %s", progress)
	}
	post(`{"command": "stop_watch", "args": {"id": 7}}`)
	select {
	case got := <-done:
		if !strings.Contains(got, "it was stopped") {
			t.Fatalf("the stopped handler: %s", got)
		}
	case <-time.After(10 * time.Second):
		t.Fatal("Stop did not reach the handler")
	}
	// A Stop before its handler's request: the handler is stopped at its first look.
	post(`{"command": "stop_watch", "args": {"id": 8}}`)
	got := post(`{"command": "dispatch_event", "args": {"state": {"window": "Card", "instance": 1, "data": {"name": "Ann"}}, "handler": "plan", "watch": 8}}`)
	if !strings.Contains(got, "it was stopped") {
		t.Fatalf("an early Stop: %s", got)
	}
	// Without a watch the handler runs unwatched.
	post(`{"command": "dispatch_event", "args": {"state": {"window": "Card", "instance": 1, "data": {"name": "Ann"}}, "handler": "long"}}`)
	if w := model.watched[len(model.watched)-1]; w.Stop != nil {
		t.Fatal("a handler without a watch was watched")
	}
}

// pagesCards is a card whose settings record starts from the asking page's:
// its open handler writes what it saw, and flip changes a setting and quits.
type pagesCards struct{ *cards }

func (c pagesCards) Open(db *clarity.Db, window string, with map[string]any) (clarity.WindowState, error) {
	return clarity.NewWindowState(window, map[string]any{"settings": map[string]any{"id": 0.0, "showDone": true, "theme": ""}}), nil
}

func (c pagesCards) Handler(window, event string) (string, bool) {
	return "opened", event == "open"
}

func (c pagesCards) Dispatch(db *clarity.Db, state clarity.WindowState, handler string) (clarity.Reply, error) {
	ui := clarity.NewUi(state)
	settings := state.Data["settings"].(map[string]any)
	switch handler {
	case "opened":
		state.Data["saw"] = settings["showDone"]
	case "flip":
		settings["showDone"] = !settings["showDone"].(bool)
		ui.Quit()
	}
	state.Data["said"] = clarity.TextOf("hi")
	return ui.Reply(state.Window, state.Data)
}

// Each browser apart (DESIGN 7.9 B2): a request names its page and brings
// the person's settings as the page keeps them. A handler sees that page's
// settings and language, a page that sends none keeps what it had, settings
// a command changed and quit go back to that page alone, and a page unasked
// for longer than the quiet time is forgotten.
func TestEachBrowserHasItsOwnSettingsAndLanguage(t *testing.T) {
	c, _ := service(t)
	options := c.Options()
	options.Model = []byte(`{"project":{"locale":{"language":"en"},"translations":{"en":{"hi":"Hello"},"nl":{"hi":"Hallo"}}}}`)
	options.Windows = pagesCards{&cards{}}
	defer clarity.SetLanguage(nil)
	program := NewProgram(ProgramOptions{Options: options})
	program.quiet = 300 * time.Millisecond
	server := httptest.NewServer(program.Handler())
	defer server.Close()
	ask := func(body string) map[string]any {
		reply, err := http.Post(server.URL+"/clarity/invoke", "application/json", strings.NewReader(body))
		if err != nil {
			t.Fatal(err)
		}
		defer reply.Body.Close()
		var answer map[string]any
		if err := json.NewDecoder(reply.Body).Decode(&answer); err != nil {
			t.Fatal(err)
		}
		return answer
	}
	data := func(answer map[string]any) map[string]any {
		return answer["value"].(map[string]any)["state"].(map[string]any)["data"].(map[string]any)
	}
	if got := ask(`{"command":"set_language","args":{"language":"nl"},"browser":"a","settings":{"showDone":false}}`); got["value"] != nil || got["error"] != nil {
		t.Fatal(got)
	}
	a := ask(`{"command":"open_window","args":{"window":"Card"},"browser":"a","settings":{"showDone":false}}`)
	if d := data(a); d["saw"] != false || d["said"] != "Hallo" || a["settings"] != nil {
		t.Fatalf("page a: %v", a)
	}
	b := ask(`{"command":"open_window","args":{"window":"Card"},"browser":"b"}`)
	if d := data(b); d["saw"] != true || d["said"] != "Hello" {
		t.Fatalf("page b: %v", b)
	}
	state, _ := json.Marshal(a["value"].(map[string]any)["state"])
	flipped := ask(`{"command":"dispatch_event","args":{"state":` + string(state) + `,"handler":"flip"},"browser":"a"}`)
	if kept, _ := json.Marshal(flipped["settings"]); string(kept) != `{"showDone":true,"theme":""}` || flipped["value"].(map[string]any)["quit"] != true || data(flipped)["said"] != "Hallo" {
		t.Fatalf("a's flip: %v", flipped)
	}
	b = ask(`{"command":"open_window","args":{"window":"Card"},"browser":"b"}`)
	if b["settings"] != nil || b["value"].(map[string]any)["quit"] != nil || data(b)["said"] != "Hello" {
		t.Fatalf("page b after a's flip: %v", b)
	}
	// a sends no settings: it keeps those it had, the changed ones.
	if a = ask(`{"command":"open_window","args":{"window":"Card"},"browser":"a"}`); data(a)["saw"] != true {
		t.Fatalf("a kept: %v", a)
	}
	// Unasked for longer than the quiet time, a page is forgotten: its language too.
	time.Sleep(400 * time.Millisecond)
	ask(`{"command":"open_window","args":{"window":"Card"},"browser":"b"}`)
	if a = ask(`{"command":"open_window","args":{"window":"Card"},"browser":"a"}`); data(a)["said"] != "Hello" {
		t.Fatalf("a forgotten: %v", a)
	}
}
