// Signing in through the wire (docs/APP-CAPABILITIES-PLAN.md, step 4): the
// start says when sign-in comes first, signin and signout answer, and a
// served call runs as the person of its clarity_session cookie, a call
// without one reaching only the sign-in window.

package shell

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
	"github.com/petervdpas/Clarity/runtimes/go/internal/authfake"
)

// signinService is the cards service with people who sign in on targets.
func signinService(t *testing.T, targets ...string) *Service {
	c, _ := service(t)
	db := c.Options().Db
	if err := db.Batch("", "CREATE TABLE members (id INTEGER PRIMARY KEY AUTOINCREMENT, email TEXT, password_hash TEXT, login TEXT); INSERT INTO members (email, password_hash) VALUES ('ann@shell.test', '"+authfake.Hash("pw")+"')"); err != nil {
		t.Fatal(err)
	}
	source := clarity.SigninSource{Table: "members", Page: "SignIn", Targets: targets, Strategies: []clarity.SigninStrategy{{Name: "password", Columns: []string{"email", "password_hash"}}}}
	if err := db.AddSignin(source, authfake.New().Call); err != nil {
		t.Fatal(err)
	}
	return c
}

func TestSigninComesFirstOnTheDesktopUntilSomeoneSignedIn(t *testing.T) {
	c := signinService(t, "desktop")
	if got := wire(t, call(t, c, "start", `{}`)); !strings.Contains(got, `"signin":"SignIn"`) {
		t.Fatalf("start: %s", got)
	}
	if got := wire(t, call(t, c, "signin", `{"strategy":"password","inputs":{"name":"ann@shell.test","password":"no"}}`)); got != `{"error":{"key":"signinRefused","text":"Signing in did not work. Check what you typed and try again."}}` {
		t.Fatalf("a wrong password: %s", got)
	}
	if got := wire(t, call(t, c, "signin", `{"strategy":"password","inputs":{"name":"ann@shell.test","password":"pw"}}`)); got != `{"value":1}` {
		t.Fatalf("signin: %s", got)
	}
	if got := wire(t, call(t, c, "start", `{}`)); strings.Contains(got, "signin") {
		t.Fatalf("start after signing in: %s", got)
	}
	call(t, c, "signout", `{}`)
	if got := wire(t, call(t, c, "start", `{}`)); !strings.Contains(got, `"signin":"SignIn"`) {
		t.Fatalf("start after signing out: %s", got)
	}
	// The program's start on the wire says it too.
	p := NewProgram(ProgramOptions{Options: c.Options()})
	if got := string(p.Answer("started", nil, 0)); !strings.Contains(got, `"signin":"SignIn"`) {
		t.Fatalf("started: %s", got)
	}
}

func TestSigninServedCallsRunAsThePersonOfTheirSessionCookie(t *testing.T) {
	c := signinService(t, "web")
	server := httptest.NewServer(NewProgram(ProgramOptions{Options: c.Options()}).Handler())
	defer server.Close()
	ask := func(cookie, body string) (map[string]any, *http.Response) {
		t.Helper()
		request, _ := http.NewRequest(http.MethodPost, server.URL+InvokePath, strings.NewReader(body))
		if cookie != "" {
			request.Header.Set("Cookie", "theme=dark; "+clarity.SessionCookie+"="+cookie)
		}
		reply, err := http.DefaultClient.Do(request)
		if err != nil {
			t.Fatal(err)
		}
		defer reply.Body.Close()
		var answer map[string]any
		if err := json.NewDecoder(reply.Body).Decode(&answer); err != nil {
			t.Fatal(err)
		}
		return answer, reply
	}
	refusedKey := func(answer map[string]any) any {
		e, _ := answer["error"].(map[string]any)
		return e["key"]
	}
	started, _ := ask("", `{"command":"started","browser":"a"}`)
	if started["value"].(map[string]any)["signin"] != "SignIn" {
		t.Fatalf("started: %v", started)
	}
	// Without a session only the sign-in window and its commands are answered.
	for _, body := range []string{`{"command":"open_window","args":{"window":"Card"},"browser":"a"}`, `{"command":"start_app","browser":"a"}`, `{"command":"dispatch_event","args":{"state":{"window":"Card","data":{}},"handler":"remember"},"browser":"a"}`, `{"command":"engine_open","args":{"name":"x"},"browser":"a"}`} {
		if got, _ := ask("", body); refusedKey(got) != "notAllowed" {
			t.Fatalf("%s: %v", body, got)
		}
		if got, _ := ask("made-up", body); refusedKey(got) != "notAllowed" {
			t.Fatalf("a made-up session, %s: %v", body, got)
		}
	}
	if got, _ := ask("", `{"command":"open_window","args":{"window":"SignIn"},"browser":"a"}`); refusedKey(got) == "notAllowed" {
		t.Fatalf("the sign-in window: %v", got)
	}
	got, reply := ask("", `{"command":"signin","args":{"strategy":"password","inputs":{"name":"ann@shell.test","password":"pw"}},"browser":"a"}`)
	set := reply.Header.Get("Set-Cookie")
	if got["value"] != float64(1) || !strings.HasPrefix(set, clarity.SessionCookie+"=") || !strings.HasSuffix(set, "; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800") {
		t.Fatalf("signin: %v, cookie %q", got, set)
	}
	session := clarity.SessionOf(strings.Split(set, ";")[0])
	if got, _ := ask(session, `{"command":"open_window","args":{"window":"Card"},"browser":"a"}`); got["error"] != nil {
		t.Fatalf("with the session: %v", got)
	}
	if got, _ := ask(session, `{"command":"started","browser":"a"}`); got["value"].(map[string]any)["signin"] != nil {
		t.Fatalf("started with the session: %v", got)
	}
	_, reply = ask(session, `{"command":"signout","browser":"a"}`)
	if cleared := reply.Header.Get("Set-Cookie"); cleared != clarity.SessionCookie+"=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0" {
		t.Fatalf("signout cookie %q", cleared)
	}
	if got, _ := ask(session, `{"command":"open_window","args":{"window":"Card"},"browser":"a"}`); refusedKey(got) != "notAllowed" {
		t.Fatalf("after signing out: %v", got)
	}
	// The desktop is not the web: its calls are not gated.
	if got := c.Invoke(OnTheDesktop(context.Background()), "open_window", json.RawMessage(`{"window":"Card"}`)); got.Error != nil {
		t.Fatalf("desktop: %v", got.Error)
	}
}
