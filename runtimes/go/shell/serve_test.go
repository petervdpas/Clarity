// The web target (DESIGN 7.3): the shell serves the renderer to a browser
// and answers its commands over HTTP, with the same Invoke the desktop
// shell binds into its window.

package shell

import (
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"testing/fstest"
)

func TestTheServerServesThePageAndAnswersItsCommands(t *testing.T) {
	c, _ := service(t)
	assets := fstest.MapFS{
		"index.html": {Data: []byte("<html><head><title>Shop</title></head><body></body></html>")},
		"runtime.js": {Data: []byte("// the renderer")},
	}
	server := httptest.NewServer(c.Handler(assets))
	defer server.Close()

	get := func(path string) (int, string) {
		reply, err := http.Get(server.URL + path)
		if err != nil {
			t.Fatal(err)
		}
		defer reply.Body.Close()
		body, _ := io.ReadAll(reply.Body)
		return reply.StatusCode, string(body)
	}
	// The page says where it sends its commands.
	if status, page := get("/"); status != 200 || !strings.Contains(page, `<script>window.clarityServer = "/clarity/invoke";</script></head>`) {
		t.Fatalf("%d %s", status, page)
	}
	if status, script := get("/runtime.js"); status != 200 || script != "// the renderer" {
		t.Fatalf("%d %s", status, script)
	}
	// A command and its arguments, answered as the desktop shell answers them.
	post := func(body string) string {
		reply, err := http.Post(server.URL+"/clarity/invoke", "application/json", strings.NewReader(body))
		if err != nil {
			t.Fatal(err)
		}
		defer reply.Body.Close()
		answer, _ := io.ReadAll(reply.Body)
		return strings.TrimSpace(string(answer))
	}
	var lines []string
	c.logLine = func(line string) { lines = append(lines, line) }
	if got := post(`{"command": "log_line", "args": {"line": "engine Sky: hello"}}`); got != `{"value":null}` || strings.Join(lines, "") != "engine Sky: hello" {
		t.Fatal(got, lines)
	}
	if got := post(`{"command": "nonsense", "args": {}}`); got != `{"error":"there is no command nonsense"}` {
		t.Fatal(got)
	}
	if got := post(`not json`); !strings.HasPrefix(got, `{"error":`) {
		t.Fatal(got)
	}
}

func TestTheServerListensWhereItIsTold(t *testing.T) {
	t.Setenv("CLARITY_ADDRESS", "")
	if got := Address([]string{"--serve"}); got != DefaultAddress {
		t.Fatal(got)
	}
	if got := Address([]string{"--serve", "--address", "0.0.0.0:9000"}); got != "0.0.0.0:9000" {
		t.Fatal(got)
	}
	t.Setenv("CLARITY_ADDRESS", ":7000")
	if got := Address(nil); got != ":7000" {
		t.Fatal(got)
	}
}
