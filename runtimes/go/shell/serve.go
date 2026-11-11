// The web target (DESIGN 7.3, round B of 7.9): the application as a web
// server. It serves the browser files (Clarity.UI's page, its library
// compiled for the browser and the program's files.json and resources) and
// answers the page's wire at /clarity/invoke with the program, which asks
// the same service the desktop does. The database is the server's.

package shell

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"maps"
	"net"
	"net/http"
	"os"
	"reflect"
	"strings"
	"sync/atomic"
	"time"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
)

// InvokePath is where a page the server serves sends its commands.
const InvokePath = "/clarity/invoke"

// largestBody is the largest command the server reads: one with a file in it.
const largestBody = 64 << 20

// Handler serves the program's browser files (its options' Assets), the
// page among them, and answers the page's wire.
func (p *Program) Handler() http.Handler {
	assets := p.options.Options.Assets
	mux := http.NewServeMux()
	mux.HandleFunc(InvokePath, func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "send a command with POST", http.StatusMethodNotAllowed)
			return
		}
		// The page that asks (one id per tab) and its person's settings as it keeps them.
		var sent struct {
			Command  string          `json:"command"`
			Args     json.RawMessage `json:"args"`
			Browser  string          `json:"browser"`
			Settings json.RawMessage `json:"settings"`
		}
		var answer []byte
		body, err := io.ReadAll(io.LimitReader(r.Body, largestBody))
		if err == nil {
			err = json.Unmarshal(body, &sent)
		}
		if err != nil {
			answer = refused("a command is JSON with its command and its args: " + err.Error())
		} else {
			var settings map[string]any
			_ = json.Unmarshal(sent.Settings, &settings)
			// Signing in (step 4): the session the cookie carries, the address it came from.
			session := ""
			if cookie, err := r.Cookie(clarity.SessionCookie); err == nil {
				session = cookie.Value
			}
			address := r.RemoteAddr
			if host, _, err := net.SplitHostPort(address); err == nil {
				address = host
			}
			var setCookie string
			answer, setCookie = p.AnswerVisit(sent.Browser, settings, session, address, sent.Command, sent.Args)
			if setCookie != "" {
				w.Header().Add("Set-Cookie", setCookie)
			}
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write(answer)
	})
	if assets == nil {
		return mux
	}
	files := http.FileServerFS(assets)
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" && r.URL.Path != "/index.html" {
			files.ServeHTTP(w, r)
			return
		}
		page, err := fs.ReadFile(assets, "index.html")
		if err != nil {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		_, _ = w.Write([]byte(Served(string(page))))
	})
	return mux
}

// requests numbers each handler request, as the desktop numbers its threads:
// a watch belongs to the request that names it. Above the threads' numbers.
var requests atomic.Uint64

// Quiet is how long a page may go unasked before the server forgets it: a
// browser tab closes without saying so.
const Quiet = 30 * time.Minute

// servedPage is one browser tab: its person's settings as it last sent
// them, the language its set_language chose, when it last asked.
type servedPage struct {
	Page
	used time.Time
}

// AnswerPage answers command of page browser, as that page: with its
// settings (as the page keeps them; nil keeps those it had) and its
// language, which set_language chooses for it alone. The answer carries the
// page's settings ("settings") when the command changed them, for that page
// alone to keep. What the program does not answer is an error, which the
// library takes as its default.
func (p *Program) AnswerPage(browser string, settings map[string]any, command string, args json.RawMessage) []byte {
	answer, _ := p.AnswerVisit(browser, settings, "", "", command, args)
	return answer
}

// AnswerVisit is AnswerPage for a request that carries the session of its
// clarity_session cookie (session, "" for none) from address: the command
// runs as that session's person, and the answer comes with the value of
// the Set-Cookie header the command made ("" for none) when it signed in
// or out.
func (p *Program) AnswerVisit(browser string, settings map[string]any, session, address, command string, args json.RawMessage) ([]byte, string) {
	now := time.Now()
	p.pagesMu.Lock()
	for id, page := range p.pages {
		if id != browser && now.Sub(page.used) > p.quiet {
			delete(p.pages, id)
		}
	}
	page := p.pages[browser]
	if page == nil {
		page = &servedPage{}
		p.pages[browser] = page
	}
	page.used = now
	if settings != nil {
		page.Settings = settings
	}
	if command == "set_language" {
		var given struct {
			Language *string `json:"language"`
		}
		_ = json.Unmarshal(args, &given)
		if given.Language != nil && *given.Language == "" {
			given.Language = nil
		}
		page.Language = given.Language
		p.pagesMu.Unlock()
		return value(nil), ""
	}
	asking := &Page{Settings: maps.Clone(page.Settings), Language: page.Language, Session: session, Address: address}
	p.pagesMu.Unlock()
	answer := p.answerRequest(WithPage(context.Background(), asking), command, args)
	if answer == nil {
		return refused("there is no command " + command), ""
	}
	var out map[string]json.RawMessage
	changedTo, changed := settingsChanged(args, answer, asking.Settings)
	if !changed || json.Unmarshal(answer, &out) != nil {
		return answer, asking.SetCookie
	}
	p.pagesMu.Lock()
	if page.Settings == nil {
		page.Settings = map[string]any{}
	}
	maps.Copy(page.Settings, changedTo)
	kept, _ := json.Marshal(page.Settings)
	p.pagesMu.Unlock()
	out["settings"] = kept
	if both, err := json.Marshal(out); err == nil {
		return both, asking.SetCookie
	}
	return answer, asking.SetCookie
}

// settingsChanged is the settings record of the window answer gives (its
// id left out) and whether the command changed one of them: against the
// record of the window args sent, or else against before.
func settingsChanged(args, answer []byte, before map[string]any) (map[string]any, bool) {
	var given struct {
		State struct {
			Data struct {
				Settings map[string]any `json:"settings"`
			} `json:"data"`
		} `json:"state"`
	}
	_ = json.Unmarshal(args, &given)
	if given.State.Data.Settings != nil {
		before = given.State.Data.Settings
	}
	var after struct {
		Value struct {
			State struct {
				Data struct {
					Settings map[string]any `json:"settings"`
				} `json:"data"`
			} `json:"state"`
		} `json:"value"`
	}
	if json.Unmarshal(answer, &after) != nil {
		return nil, false
	}
	changedTo := after.Value.State.Data.Settings
	delete(changedTo, "id")
	for name, value := range changedTo {
		if was, had := before[name]; had && !reflect.DeepEqual(was, value) {
			return changedTo, true
		}
	}
	return changedTo, false
}

// answerRequest answers one request of the page. A handler's request that
// names a watch (`"watch": id`) runs under it: how far it is and Stop come
// by other requests meanwhile, which run side by side, and a Stop that came
// first holds.
func (p *Program) answerRequest(ctx context.Context, command string, args json.RawMessage) []byte {
	if command != "dispatch_event" && command != "answer_event" {
		return p.answerIn(ctx, command, args, 0)
	}
	var named struct {
		Watch uint64 `json:"watch"`
	}
	_ = json.Unmarshal(args, &named)
	if named.Watch == 0 {
		return p.answerIn(ctx, command, args, 0)
	}
	request := 1<<63 | requests.Add(1)
	id, _ := json.Marshal(map[string]uint64{"id": named.Watch})
	p.Answer("start_watch", id, request)
	defer p.Answer("end_watch", id, request)
	return p.answerIn(ctx, command, args, request)
}

// Served is the page as the server gives it: it says where its commands go.
func Served(page string) string {
	return strings.Replace(page, "</head>", fmt.Sprintf("<script>window.clarityServer = %q;</script></head>", InvokePath), 1)
}

// Serve runs the program as a web server on address, such as
// "127.0.0.1:8080", until it stops; its options' Assets are the browser
// files it serves.
func Serve(options ProgramOptions, address string) error {
	program := NewProgram(options)
	fmt.Fprintf(os.Stderr, "%s: serving on http://%s\n", options.Options.Name, address)
	return http.ListenAndServe(address, program.Handler())
}

// Address is where the server listens: `--address host:port`, else the
// environment's CLARITY_ADDRESS, else project, where the project file says
// (its port).
func Address(args []string, project string) string {
	for i, arg := range args {
		if arg == "--address" && i+1 < len(args) {
			return args[i+1]
		}
	}
	if address := os.Getenv("CLARITY_ADDRESS"); address != "" {
		return address
	}
	return project
}
