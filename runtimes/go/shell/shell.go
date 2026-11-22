// Package shell answers the commands of a generated Go application's
// screen: the web wire (manual/project.run-build.md), the same on every stack. Clarity.UI's
// desktop asks them through package ui, and in a browser the web target's
// page asks them over HTTP (serve.go); Program (program.go) answers both.
package shell

import (
	"context"
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
)

// Windows is what the generated program knows of its windows and handlers.
type Windows interface {
	// Open is the state of window when it opens: its data loaded, with
	// with filled in by name.
	Open(db *clarity.Db, window string, with map[string]any) (clarity.WindowState, error)
	// Handler is the handler of a window's own event (open, close, return,
	// start); the app's start has no window.
	Handler(window, event string) (string, bool)
	// Dispatch runs a handler on a window's state.
	Dispatch(db *clarity.Db, state clarity.WindowState, handler string) (clarity.Reply, error)
	// Answer runs the handler the answer to a question goes to (DESIGN
	// 5.14), with the values written for it and the answer last.
	Answer(db *clarity.Db, state clarity.WindowState, handler string, values []json.RawMessage) (clarity.Reply, error)
	// Redraw draws the canvases of the window a reply shows anew, each by
	// its draw handler (conformance/drawing.json). The shell redraws after
	// a window opens and after every handler.
	Redraw(db *clarity.Db, reply clarity.Reply) (clarity.Reply, error)
	// Options are the records a reference field of entity can point at.
	Options(db *clarity.Db, entity string) ([]clarity.RefOption, error)
	// Values are the values a text field already has, which a typed dropdown offers.
	Values(db *clarity.Db, entity, field string) ([]string, error)
	// Picture is the picture of image field field of record id of entity;
	// nil when it has none.
	Picture(db *clarity.Db, entity string, id int64, field string) ([]byte, error)
}

type Options struct {
	// Name is the program's name; Title the title of its window until a
	// Clarity window gives its own.
	Name, Title string
	// Main is the window the application opens with; Width and Height the
	// size its desktop window opens at (zero for the standard size).
	Main          string
	Width, Height int
	// Model is the windows, frame and data dictionary the renderer draws from.
	Model []byte
	// Assets are the browser files a web server serves (browser/ of the
	// program): the page, Clarity.UI's browser files, files.json and the resources.
	Assets  fs.FS
	Db      *clarity.Db
	Windows Windows
	// Unconnected is what the screen shows when the databases did not open
	// (manual/data.settings.md), with Db an empty one in memory until Connect opens them;
	// nil when they opened.
	Unconnected *Unconnected
	Connect     func() (*clarity.Db, error)
	// Secrets are the names of the secret settings (manual/data.settings.md), kept in
	// the system's protected store, never in the settings file.
	Secrets []string
}

// Unconnected is why the databases did not open, and what the screen shows
// first: the app's `on unconnected` handler, or else the window that keeps
// a secret connection; "" for none.
type Unconnected struct {
	Handler, Window string
	// Problem is why, in Clarity's own words the project translates.
	Problem clarity.Text
}

// orNull is text as the screen takes it: null when there is none.
func orNull(text string) any {
	if text == "" {
		return nil
	}
	return text
}

// Answer is what a command gives: its value, or what went wrong as the
// screen shows it. A refusal keeps its slots, so the screen fills them in
// the format of the person using the application.
type Answer struct {
	Value any
	Error any
}

// MarshalJSON writes {"error": ...} for a failed command, {"value": ...} otherwise.
func (a Answer) MarshalJSON() ([]byte, error) {
	if a.Error != nil {
		return json.Marshal(map[string]any{"error": a.Error})
	}
	return json.Marshal(map[string]any{"value": a.Value})
}

// Desktop answers the commands that belong to a desktop window: its title,
// windows of their own, custom parts, links opened in the person's browser
// and quitting, for a screen that asks them as commands; a web server has none.
type Desktop interface {
	// Command answers command; false when it is not the desktop's.
	Command(ctx context.Context, command string, a Args) (Answer, bool)
}

// Screen is a desktop that runs a script on the screen of the window a
// command came from: how a long call says how far it is (manual/code.engines.md).
type Screen interface {
	Eval(ctx context.Context, js string)
}

// Service answers the renderer's commands.
type Service struct {
	// stop is closed by stop_call to stop the handler that runs; each
	// watched handler gets a new one. Not behind mu, which the handler holds.
	stopMu  sync.Mutex
	stop    chan struct{}
	options Options
	desktop Desktop
	// mu keeps one command at a time on the databases, as the Tauri shell does.
	mu sync.Mutex
	// logLine writes a line of an engine beside the screen into the
	// application's log: the standard error, where the backend's engines write.
	logLine func(line string)
	// toScreen is whether the title bar's close of the main window goes to
	// the screen: while the databases are not open, closing it connects again
	// (manual/data.settings.md). Not behind mu: a command that holds mu waits on the
	// desktop's thread, where the close arrives.
	toScreen atomic.Bool
}

// ClosesToScreen is whether the title bar's close of the main window goes to the screen.
func (c *Service) ClosesToScreen() bool { return c.toScreen.Load() }

// Options are the service's options, for the desktop.
func (c *Service) Options() Options { return c.options }

// SetDesktop gives the service the desktop window's side of the commands.
func (c *Service) SetDesktop(desktop Desktop) { c.desktop = desktop }

// Lock and Unlock keep the databases for the desktop while it changes what
// the service's commands read, as one command does.
func (c *Service) Lock()   { c.mu.Lock() }
func (c *Service) Unlock() { c.mu.Unlock() }

func New(options Options) *Service {
	// Text code makes is translated from the model's translations.
	clarity.LoadTranslations(options.Model)
	return &Service{options: options, logLine: func(line string) { fmt.Fprintln(os.Stderr, line) }}
}

// opensLink says whether a link of a markdown part may open: a web or a
// mail address, as every stack allows.
func opensLink(address string) bool {
	for _, scheme := range []string{"http://", "https://", "mailto:"} {
		if strings.HasPrefix(address, scheme) {
			return true
		}
	}
	return false
}

// failed is the answer of a command that failed: a refusal, and a value or a
// record that is not there or still used, in their words (clarity.Said);
// anything else by its message.
func failed(err error) Answer {
	if said, ok := clarity.Said(err); ok {
		return Answer{Error: said}
	}
	return Answer{Error: err.Error()}
}

func answer(value any, err error) Answer {
	if err != nil {
		return failed(err)
	}
	return Answer{Value: value}
}

// onEvent runs the handler of a window's own event, or gives the state as
// it is when the window has none.
func (c *Service) onEvent(state clarity.WindowState, event string) (clarity.Reply, error) {
	if handler, ok := c.options.Windows.Handler(state.Window, event); ok {
		return c.dispatch(state, handler)
	}
	reply, err := clarity.NewUi(state).Reply(state.Window, state.Data)
	if err != nil {
		return reply, err
	}
	return c.options.Windows.Redraw(c.options.Db, reply)
}

// onEventless draws the window as its data is, running no handler.
func (c *Service) onEventless(state clarity.WindowState) (clarity.Reply, error) {
	reply, err := clarity.NewUi(state).Reply(state.Window, state.Data)
	if err != nil {
		return reply, err
	}
	return c.options.Windows.Redraw(c.options.Db, reply)
}

// stepped runs a handler, if any, as one database step: the reply, and the
// step it made (none when it changed nothing). No handler: the window as it is.
func (c *Service) stepped(state clarity.WindowState, handler string) (clarity.Reply, []int64, error) {
	if handler == "" {
		reply, err := c.onEventless(state)
		return reply, nil, err
	}
	db := c.options.Db
	before := db.LastStep()
	var reply clarity.Reply
	err := db.Step(func() error {
		var err error
		reply, err = c.dispatch(state, handler)
		return err
	})
	if err != nil || db.LastStep() == before {
		return reply, nil, err
	}
	reply.Step = db.LastStep()
	return reply, []int64{reply.Step}, nil
}

// dispatch runs a handler, then draws the window's canvases anew.
func (c *Service) dispatch(state clarity.WindowState, handler string) (clarity.Reply, error) {
	reply, err := c.options.Windows.Dispatch(c.options.Db, state, handler)
	if err != nil {
		return reply, err
	}
	return c.options.Windows.Redraw(c.options.Db, reply)
}

// Page is one browser tab of a served program (manual/start.how-it-works.md B2): its
// person's settings as the page keeps them and the language its
// set_language chose. A command asked with one in its context runs in that
// language, and a window it opens starts from those settings.
//
// Signing in (docs/ledger/signin.md): Session is the token
// of the request's clarity_session cookie and Address where it came from;
// SetCookie is what the command sets of that cookie, for the server to send.
type Page struct {
	Settings  map[string]any
	Language  *string
	Session   string
	Address   string
	SetCookie string
}

type pageKey struct{}

// WithPage is ctx asking for page.
func WithPage(ctx context.Context, page *Page) context.Context {
	return context.WithValue(ctx, pageKey{}, page)
}

func pageOf(ctx context.Context) *Page {
	page, _ := ctx.Value(pageKey{}).(*Page)
	return page
}

type desktopKey struct{}

// OnTheDesktop is ctx asking for the desktop: a window it opens starts
// from the settings the person kept, as a page's start from the page's.
func OnTheDesktop(ctx context.Context) context.Context {
	return context.WithValue(ctx, desktopKey{}, true)
}

func onTheDesktop(ctx context.Context) bool {
	desktop, _ := ctx.Value(desktopKey{}).(bool)
	return desktop
}

// withPageSettings has the window's settings record start from the page's:
// each setting the record has, of the same kind.
func withPageSettings(data map[string]any, page *Page) {
	if page != nil {
		withSettings(data, page.Settings)
	}
}

// withSettings has the window's settings record start from settings: each
// setting the record has, of the same kind.
func withSettings(data map[string]any, settings map[string]any) {
	if len(settings) == 0 || data["settings"] == nil {
		return
	}
	text, err := json.Marshal(data["settings"])
	if err != nil {
		return
	}
	var record map[string]any
	if json.Unmarshal(text, &record) != nil || record == nil {
		return
	}
	kind := func(v any) string { return fmt.Sprintf("%T", v) }
	for name, value := range settings {
		if was, has := record[name]; has && name != "id" && kind(was) == kind(value) {
			record[name] = value
		}
	}
	data["settings"] = record
}

// secrets are the names of the secret settings: those the command names,
// or else the program's own.
func (c *Service) secrets(a Args) []string {
	if a.Secrets != nil {
		return a.Secrets
	}
	return c.options.Secrets
}

// Invoke runs a command of the renderer: the same commands, with the same
// arguments, as the Tauri shell has.
func (c *Service) Invoke(ctx context.Context, command string, args json.RawMessage) Answer {
	var a Args
	if len(args) > 0 {
		if err := json.Unmarshal(args, &a); err != nil {
			return Answer{Error: fmt.Sprintf("the arguments of %s do not fit it: %v", command, err)}
		}
	}
	return c.invoke(ctx, command, a)
}

// InvokeWatched runs a command as Invoke does, with watch as the watch of
// every engine call its handler makes: how a desktop on Clarity.UI shows how
// far a long call is and stops it (manual/code.engines.md).
func (c *Service) InvokeWatched(ctx context.Context, command string, args json.RawMessage, watch clarity.Watch) Answer {
	var out Answer
	clarity.Watching(watch, func() { out = c.Invoke(ctx, command, args) })
	return out
}

// Args are the arguments of every command, by name.
type Args struct {
	Window   string              `json:"window"`
	With     map[string]any      `json:"with"`
	Language *string             `json:"language"`
	Width    int                 `json:"width"`
	Height   int                 `json:"height"`
	State    clarity.WindowState `json:"state"`
	Handler  string              `json:"handler"`
	Values   []json.RawMessage   `json:"values"`
	Entity   string              `json:"entity"`
	Title    string              `json:"title"`
	ID       int64               `json:"id"`
	Field    string              `json:"field"`
	Store    string              `json:"store"`
	Path     string              `json:"path"`
	// Scope is where an edit is taken back (manual/code.handlers.md): its form or panel,
	// or the part itself; Ends says whether the edit ended (a drag goes on
	// to its drop); Back whether Ctrl+Z takes it back or Ctrl+Y does it again.
	Scope    string         `json:"scope"`
	Ends     bool           `json:"ends"`
	Back     bool           `json:"back"`
	Address  string         `json:"address"`
	Line     string         `json:"line"`
	Settings map[string]any `json:"settings"`
	// The secret settings by name (manual/data.settings.md): kept in the system's protected store, never in the file.
	Secrets  []string        `json:"secrets"`
	Name     string          `json:"name"`
	Data     string          `json:"data"`
	Instance uint64          `json:"instance"`
	Part     string          `json:"part"`
	Hear     string          `json:"hear"`
	X        int             `json:"x"`
	Y        int             `json:"y"`
	Shown    bool            `json:"shown"`
	Value    json.RawMessage `json:"value"`
	// A canvas's list (canvas_event, manual/parts.graphics.md).
	List string `json:"list"`
	// The x and y fields of a canvas of objects (canvas_event).
	At json.RawMessage `json:"at"`
	// A wizard's button (step_event), its count of steps and its on next and
	// on finish handlers.
	Button string `json:"button"`
	Steps  int64  `json:"steps"`
	Next   string `json:"next"`
	Finish string `json:"finish"`
	// Action is what the person did on a canvas (canvas_event).
	Action map[string]json.RawMessage `json:"action"`
	// Readonly is the part's readonly as the screen has it (manual/start.how-it-works.md): the
	// runtime takes back what the person changed. Editable are the fields a
	// table's cells edit (edit_event, its list at Path); nil when not given.
	Readonly bool     `json:"readonly"`
	Editable []string `json:"editable"`
	// A canvas's mode and its parameters (canvas_event, manual/parts.graphics.md), with
	// List, Path, At (an object's x and y fields) and Action.
	Mode  string  `json:"mode"`
	Units *[2]int `json:"units"`
	Grid  int     `json:"grid"`
	Snap  bool    `json:"snap"`
	// Signing in (signin): the strategy and its inputs by name (name, password, token).
	Strategy string            `json:"strategy"`
	Inputs   map[string]string `json:"inputs"`
}

// watched are the commands that run a handler, which may make a long engine call.
var watched = map[string]bool{"start_app": true, "open_window": true, "close_window": true, "return_to": true, "dispatch_event": true, "answer_event": true, "edit_event": true, "step_event": true, "canvas_event": true}

// progressScript tells the screen how far a long call is; none once it ended.
func progressScript(done, total int64, ended bool) string {
	if ended {
		return `window.dispatchEvent(new CustomEvent("clarity-progress", {detail: null}))`
	}
	return fmt.Sprintf(`window.dispatchEvent(new CustomEvent("clarity-progress", {detail: {"done":%d,"total":%d}}))`, done, total)
}

func (c *Service) invoke(ctx context.Context, command string, a Args) (answer Answer) {
	// Fail safe: a handler that panics, as on a list index out of range, is
	// an error the screen shows, and the application keeps working.
	defer func() {
		if r := recover(); r != nil {
			fmt.Fprintf(os.Stderr, "%s failed: %v\n", command, r)
			answer = Answer{Error: fmt.Sprint(r)}
		}
	}()
	if a.State.Data == nil {
		a.State.Data = map[string]any{}
	}
	// The person stops the handler that runs, which holds mu.
	if command == "stop_call" {
		c.stopMu.Lock()
		defer c.stopMu.Unlock()
		if c.stop != nil {
			close(c.stop)
			c.stop = nil
		}
		return Answer{}
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	// Served, the command speaks its page's language: set under mu, as commands take turns.
	if page := pageOf(ctx); page != nil {
		clarity.SetLanguage(page.Language)
	}
	// Served, the command runs as the person of its session (Db.Served).
	if page := pageOf(ctx); page != nil {
		page.SetCookie = c.options.Db.Served(page.Session, page.Address, func(me int64) bool {
			if signin := c.options.Db.SigninPage(); signin != "" && me == 0 && !openWithoutSession(command, a, signin) {
				answer = failed(clarity.NotAllowed())
			} else {
				answer = c.watchedAnswer(ctx, command, a)
			}
			return answer.Error == nil
		})
		return answer
	}
	return c.watchedAnswer(ctx, command, a)
}

// signedInOrOpen is whether a call answered beside the service may run:
// off the web always; served, when people do not sign in on the web or the
// call's session names someone.
func (c *Service) signedInOrOpen(ctx context.Context) bool {
	page := pageOf(ctx)
	if page == nil {
		return true
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.options.Db.SigninPage() == "" {
		return true
	}
	signedIn := false
	c.options.Db.Served(page.Session, page.Address, func(me int64) bool { signedIn = me != 0; return true })
	return signedIn
}

// openWithoutSession is whether a served call without a session may run
// while people sign in on the web: the program's start, its settings,
// signing in and out, and the sign-in window itself (fail safe: the rest is
// refused with notAllowed, whatever a browser sends).
func openWithoutSession(command string, a Args, page string) bool {
	switch command {
	case "start", "read_settings", "set_language", "signin", "signout":
		return true
	case "open_window":
		return a.Window == page
	}
	return a.State.Window == page
}

// watchedAnswer answers command, holding mu; a handler's engine calls say
// how far they are on a desktop that shows it.
func (c *Service) watchedAnswer(ctx context.Context, command string, a Args) (answer Answer) {
	screen, shows := c.desktop.(Screen)
	if !watched[command] || !shows {
		return c.answer(ctx, command, a)
	}
	// The handler's engine calls say how far they are: to the screen at
	// most ten times a second, and that it ended once it did.
	stop := make(chan struct{})
	c.stopMu.Lock()
	c.stop = stop
	c.stopMu.Unlock()
	var last time.Time
	told := false
	watch := clarity.Watch{Stop: stop, Progress: func(done, total int64) {
		if now := time.Now(); !told || now.Sub(last) >= 100*time.Millisecond || done >= total {
			last, told = now, true
			screen.Eval(ctx, progressScript(done, total, false))
		}
	}}
	clarity.Watching(watch, func() { answer = c.answer(ctx, command, a) })
	c.stopMu.Lock()
	if c.stop == stop {
		c.stop = nil
	}
	c.stopMu.Unlock()
	if told {
		screen.Eval(ctx, progressScript(0, 0, true))
	}
	return answer
}

// answer runs a command, holding mu. A window's reply is where its edits
// start (manual/code.handlers.md): what a handler changed is never the person's edit.
func (c *Service) answer(ctx context.Context, command string, a Args) Answer {
	out := c.command(ctx, command, a)
	reply, ok := out.Value.(clarity.Reply)
	if !ok {
		return out
	}
	out.Value = reply
	// A closed window's record is gone; its last reply starts none.
	if command != "close_window" {
		clarity.EditsOf(reply.State.Instance).Reset(clarity.EditData(reply.State))
	}
	return out
}

// command runs a command, holding mu.
func (c *Service) command(ctx context.Context, command string, a Args) Answer {
	db := c.options.Db
	switch command {
	case "start":
		start := map[string]any{"model": json.RawMessage(c.options.Model), "main": c.options.Main}
		if u := c.options.Unconnected; u != nil {
			c.toScreen.Store(true)
			start["unconnected"] = map[string]any{"handler": orNull(u.Handler), "window": orNull(u.Window), "problem": u.Problem}
		} else if page := db.SigninFirst(); page != "" {
			// Sign-in comes first (step 4): the window of the kind signin.
			start["signin"] = page
		}
		return Answer{Value: start}
	case "signin":
		// The person signs in with a strategy and its inputs: the record signed in, or Clarity's refusal.
		return answer(db.Signin(a.Strategy, a.Inputs))
	case "signout":
		db.Signout()
		return Answer{}
	case "connect":
		// The databases again, as when the person has kept their connection.
		if c.options.Connect == nil {
			return Answer{}
		}
		opened, err := c.options.Connect()
		if err != nil {
			return Answer{Error: clarity.UnconnectedReason(err)}
		}
		if c.options.Db != nil {
			c.options.Db.Close()
		}
		c.options.Db = opened
		c.options.Unconnected = nil
		c.toScreen.Store(false)
		return Answer{}
	case "read_settings":
		// The settings the person kept (manual/data.settings.md); the screen takes back those that still fit.
		return Answer{Value: clarity.ReadSettingsWith(c.options.Name, c.secrets(a))}
	case "keep_settings":
		// The person's settings, as the screen has them now; its secrets go to the protected store.
		if err := clarity.KeepSettingsWith(c.options.Name, a.Settings, c.secrets(a)); err != nil {
			return failed(err)
		}
		return Answer{}
	case "set_language":
		// The language the screen picked, which text code makes is translated to.
		clarity.SetLanguage(a.Language)
		return Answer{}
	case "open_window":
		state, err := c.options.Windows.Open(db, a.Window, a.With)
		if err != nil {
			return failed(err)
		}
		if onTheDesktop(ctx) {
			withSettings(state.Data, clarity.ReadSettingsWith(c.options.Name, c.options.Secrets))
		}
		withPageSettings(state.Data, pageOf(ctx))
		return answer(c.onEvent(state, "open"))
	case "close_window":
		reply, err := c.onEvent(a.State, "close")
		if err == nil {
			// The window has closed: what it had of an engine goes, and its edits.
			clarity.WindowClosed(a.State.Instance)
			clarity.EditsClosed(a.State.Instance)
		}
		return answer(reply, err)
	case "return_to":
		return answer(c.onEvent(a.State, "return"))
	case "start_app":
		return answer(c.onEvent(clarity.NewWindowState("", nil), "start"))
	case "dispatch_event":
		// One undo step (manual/code.handlers.md): the reply carries its id when the handler changed the database.
		before := db.LastStep()
		var reply clarity.Reply
		err := db.Step(func() error {
			var err error
			reply, err = c.dispatch(a.State, a.Handler)
			return err
		})
		if err == nil && db.LastStep() != before {
			reply.Step = db.LastStep()
		}
		return answer(reply, err)
	case "edit_event":
		// The person ended an edit of a part (manual/code.handlers.md): its handler runs as
		// one database step, and the edit goes on the window's record with
		// what it changed. A drag goes on until its drop.
		if a.Readonly || !c.keptToEditable(a) {
			return c.takenBack(a.State)
		}
		reply, steps, err := c.stepped(a.State, a.Handler)
		if err != nil {
			return failed(err)
		}
		bus := clarity.EditsOf(a.State.Instance)
		if a.Ends {
			bus.Edit(a.Part, a.Scope, clarity.EditData(reply.State), steps)
		} else {
			bus.Drag(a.Part, a.Scope, clarity.EditData(reply.State), steps)
		}
		return answer(reply, nil)
	case "canvas_event":
		// The person's action on a canvas, which the runtime decides (manual/start.how-it-works.md), then
		// ends as an edit does.
		if a.Readonly {
			return c.takenBack(a.State)
		}
		if a.State.Selected == nil {
			a.State.Selected = map[string]int{}
		}
		changed, refused, err := canvasChanged(a)
		if err != nil {
			return failed(err)
		}
		if !changed {
			// Refused or the same: the window as it is, running nothing.
			reply, err := c.onEventless(a.State)
			reply.Refused = refused
			return answer(reply, err)
		}
		return c.edited(a.State, a.Part, a.Scope, a.Handler)
	case "step_event":
		return answer(c.stepEvent(a))
	case "find_options":
		return c.findOptions(a)
	case "undo_edit":
		// Ctrl+Z (Back) or Ctrl+Y on a scope: its last edit taken back or done
		// again, with its database steps, and the window as it was.
		// All of the edit's steps, or none of them.
		data, found, err := clarity.EditsOf(a.State.Instance).Undo(a.Scope, clarity.EditData(a.State), a.Back, func(steps []int64, back bool) error {
			_, err := db.ReplaySteps(steps, back)
			return err
		})
		if err != nil {
			return failed(err)
		}
		if !found {
			return answer(c.onEventless(a.State))
		}
		return answer(c.onEventless(clarity.WithEditData(a.State, data)))
	case "answer_event":
		// The answer to a question a handler asked runs the handler it goes to.
		reply, err := c.options.Windows.Answer(db, a.State, a.Handler, a.Values)
		if err != nil {
			return failed(err)
		}
		return answer(c.options.Windows.Redraw(db, reply))
	case "ref_options":
		return answer(c.options.Windows.Options(db, a.Entity))
	case "field_values":
		return answer(c.options.Windows.Values(db, a.Entity, a.Field))
	case "picture":
		// A picture of a record, fetched on its own, as the page shows it.
		b, err := c.options.Windows.Picture(db, a.Entity, a.ID, a.Field)
		if err != nil || b == nil {
			return answer(nil, err)
		}
		return answer(clarity.DataURL(b))
	case "file_data":
		// The file of a file field of a record, fetched on its own, as base64.
		b, err := c.options.Windows.Picture(db, a.Entity, a.ID, a.Field)
		if err != nil || b == nil {
			return answer(nil, err)
		}
		return answer(clarity.FileData(b), nil)
	case "open_file":
		// Opens a file in the program the system uses for its kind.
		b, err := clarity.FromFileData(a.Data)
		if err == nil {
			err = clarity.OpenFile(a.Name, b)
		}
		return answer(nil, err)
	case "save_file":
		// Saves a copy of a file in the person's Downloads folder; where it went.
		b, err := clarity.FromFileData(a.Data)
		if err != nil {
			return failed(err)
		}
		return answer(clarity.SaveCopy(a.Name, b))
	case "store_file":
		// A picture linked from a file of a store, before its record is saved.
		b, err := db.StoreFile(a.Store, a.Path)
		if err != nil {
			return answer(nil, err)
		}
		return answer(clarity.DataURL(b))
	case "store_upload":
		// A file a person picked in a web app, kept in a file store: its path on the server.
		b, err := clarity.FromFileData(a.Data)
		if err != nil {
			return failed(err)
		}
		return answer(db.StoreUpload(a.Store, a.Name, b))
	case "stores":
		// The file stores and their folders, which a folder part picks from in a web app.
		return answer(db.Stores(), nil)
	case "open_link":
		if !opensLink(a.Address) {
			return Answer{Error: "a link opens only an http, https or mailto address, not " + a.Address}
		}
	case "log_line":
		c.logLine(a.Line)
		return Answer{}
	case "quit_app":
		c.toScreen.Store(false)
	}
	if c.desktop != nil {
		if answer, ok := c.desktop.Command(ctx, command, a); ok {
			return answer
		}
	}
	// Without a desktop (a web server), what belongs to one has nothing to do.
	switch command {
	case "set_title", "quit_app", "stop_call":
		return Answer{}
	case "open_link":
		return Answer{Error: "a link opens in the browser"}
	}
	return Answer{Error: "there is no command " + command}
}

// edited ends a person's action as edit_event does (manual/code.handlers.md): with a
// scope, the handler runs as one database step and the whole goes on the
// window's edit record; without one, the handler runs as dispatch_event
// does; without a handler, the window is drawn as it is.
func (c *Service) edited(state clarity.WindowState, part, scope, handler string) Answer {
	reply, steps, err := c.stepped(state, handler)
	if err != nil {
		return failed(err)
	}
	if scope != "" {
		clarity.EditsOf(state.Instance).Edit(part, scope, clarity.EditData(reply.State), steps)
	}
	return answer(reply, nil)
}

// canvasChanged applies the person's action on a canvas (manual/parts.graphics.md).
func canvasChanged(a Args) (bool, bool, error) {
	canvas := clarity.CanvasArgs{Part: a.Part, Mode: a.Mode, List: a.List, Path: a.Path, Units: a.Units, Grid: a.Grid, Snap: a.Snap}
	if a.Mode == "objects" {
		if err := json.Unmarshal(a.At, &canvas.At); err != nil {
			return false, false, &clarity.InvalidError{What: "a canvas of objects names their x and y fields, as at: [\"x\", \"y\"]"}
		}
	}
	changed, err := clarity.CanvasEvent(a.State, canvas, a.Action)
	return changed, !changed, err
}

// stepEvent is a wizard's button (conformance/steps.json): Next runs on next
// on the step shown, stays when it refused or failed, keeps the step its
// handler set, else moves on; Back goes back and runs nothing; Finish runs
// on finish. Handlers run as dispatch_event does, as no edit.
func (c *Service) stepEvent(a Args) (clarity.Reply, error) {
	parts := make(map[string]clarity.PartState, len(a.State.Parts))
	for name, part := range a.State.Parts {
		parts[name] = part
	}
	a.State.Parts = parts
	shown := clarity.ShownStep(parts[a.Part].Step, a.Steps)
	at := func(state clarity.WindowState, step int64) {
		part := state.Parts[a.Part]
		part.Step = &step
		state.Parts[a.Part] = part
	}
	at(a.State, shown)
	switch a.Button {
	case "back":
		at(a.State, clarity.BackStep(shown))
		return c.onEventless(a.State)
	case "finish":
		reply, _, err := c.stepped(a.State, a.Finish)
		return reply, err
	case "next":
		reply, _, err := c.stepped(a.State, a.Next)
		if err != nil {
			return reply, err
		}
		if reply.State.Parts == nil {
			reply.State.Parts = map[string]clarity.PartState{}
		}
		at(reply.State, clarity.NextStep(shown, reply.State.Parts[a.Part].Step, a.Steps))
		return reply, nil
	}
	return clarity.Reply{}, &clarity.InvalidError{What: fmt.Sprintf("a wizard has no button %s", a.Button)}
}

// findOptions is what a search dropdown offers: the records of the entity,
// which the screen searches as the person types (conformance/dropdown-options.json).
// A list, not a window.
func (c *Service) findOptions(a Args) Answer {
	options, err := c.options.Windows.Options(c.options.Db, a.Entity)
	if err != nil {
		return failed(err)
	}
	return Answer{Value: options}
}

// takenBack refuses what the person changed (manual/start.how-it-works.md): the window's data
// as its edit baseline had it, no handler run, no edit recorded, and the
// reply refused.
func (c *Service) takenBack(state clarity.WindowState) Answer {
	if baseline := clarity.EditsOf(state.Instance).Baseline(); baseline != nil {
		state = clarity.WithEditData(state, baseline)
	}
	reply, err := c.onEventless(state)
	reply.Refused = true
	return answer(reply, err)
}

// keptToEditable says whether a table's edit kept to its editable fields,
// against the list at a.Path as the edit baseline had it.
func (c *Service) keptToEditable(a Args) bool {
	if a.Editable == nil {
		return true
	}
	baseline := clarity.EditsOf(a.State.Instance).Baseline()
	before, had := clarity.ValueAt(baseline, a.Path)
	if baseline == nil || !had {
		return true
	}
	after, _ := clarity.ValueAt(a.State.Data, a.Path)
	return clarity.TableEditFits(before, after, a.Editable)
}
