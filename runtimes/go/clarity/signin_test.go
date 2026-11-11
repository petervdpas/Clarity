// Who is using the program on the Go runtime (conformance/signin.json):
// signing in with each strategy, refusals in Clarity's own words, the first
// account, failed tries, signing out, served sessions by their cookie, and
// the command line. With the tag clarityauth against Clarity.Auth itself,
// else against its stand-in (internal/authfake).

package clarity

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"
	"testing"
)

type signinFile struct {
	Refusals map[string]string `json:"refusals"`
	People   struct {
		Table  string `json:"table"`
		Key    string `json:"key"`
		Create string `json:"create"`
	} `json:"people"`
	Signin struct {
		Page       string           `json:"page"`
		Targets    []string         `json:"targets"`
		Strategies []SigninStrategy `json:"strategies"`
		Session    int64            `json:"sessionMinutes"`
		Tries      int              `json:"tries"`
		TriesMin   int64            `json:"triesMinutes"`
		Program    string           `json:"program"`
	} `json:"signin"`
	Cases []struct {
		Name     string                       `json:"name"`
		Remember int64                        `json:"rememberDays"`
		Asks     bool                         `json:"rememberAsks"`
		Served   bool                         `json:"served"`
		Address  string                       `json:"address"`
		Rows     []map[string]json.RawMessage `json:"rows"`
		Steps    []map[string]json.RawMessage `json:"steps"`
	} `json:"cases"`
}

func readSigninFile(t *testing.T) signinFile {
	text, err := os.ReadFile("../../../conformance/signin.json")
	must(t, err)
	var file signinFile
	must(t, json.Unmarshal(text, &file))
	return file
}

func (f signinFile) source() SigninSource {
	return SigninSource{Table: f.People.Table, Key: f.People.Key, Page: f.Signin.Page, Targets: f.Signin.Targets, Strategies: f.Signin.Strategies, SessionSeconds: f.Signin.Session * 60, Tries: f.Signin.Tries, TriesSeconds: f.Signin.TriesMin * 60, Program: f.Signin.Program}
}

// literal is a row's value as SQL: text quoted, a {hash, text} hashed as Clarity.Auth does.
func literal(t *testing.T, raw json.RawMessage) string {
	var hashed struct{ Hash, Text string }
	if json.Unmarshal(raw, &hashed) == nil && hashed.Hash != "" {
		kept, err := Hashed(testAuth(), hashed.Hash, hashed.Text)
		must(t, err)
		return "'" + strings.ReplaceAll(kept, "'", "''") + "'"
	}
	var text string
	if json.Unmarshal(raw, &text) == nil {
		return "'" + strings.ReplaceAll(text, "'", "''") + "'"
	}
	return string(raw)
}

func peopleDb(t *testing.T, f signinFile, rows []map[string]json.RawMessage) *Db {
	db, err := Memory()
	must(t, err)
	t.Cleanup(func() { db.Close() })
	must(t, db.Batch("", f.People.Create))
	for _, row := range rows {
		var columns, values []string
		for column, value := range row {
			columns = append(columns, column)
			values = append(values, literal(t, value))
		}
		must(t, db.Batch("", fmt.Sprintf("INSERT INTO %s (%s) VALUES (%s)", f.People.Table, strings.Join(columns, ", "), strings.Join(values, ", "))))
	}
	must(t, db.AddSignin(f.source(), testAuth()))
	return db
}

func countPeople(t *testing.T, db *Db, table string) int64 {
	s, err := db.source("")
	must(t, err)
	rows, err := s.driver.rows("SELECT COUNT(*) FROM "+table, nil)
	must(t, err)
	n, err := wholeOf(rows[0][0])
	must(t, err)
	return n
}

// refusedKey is the key of a refusal, "" for another error or none.
func refusedKey(err error) string {
	var refused *RefusedError
	if errors.As(err, &refused) {
		return refused.Text.Key
	}
	return ""
}

// member is a person as a generated program reads one: every column of
// the conformance file's table, the key apart.
type member struct {
	id     int64
	values []any
}

var memberColumns = []string{"name", "email", "login", "password_hash", "api_token", "recovery_codes"}

func (*member) Table() string     { return "members" }
func (*member) Columns() []string { return memberColumns }
func (m *member) GetID() int64    { return m.id }
func (m *member) SetID(id int64)  { m.id = id }
func (m *member) Values() []any   { return m.values }
func (m *member) Load(id int64, values []any) error {
	m.id, m.values = id, append([]any(nil), values...)
	return nil
}

// refusedAs checks that err is the refusal key, with the file's English.
func refusedAs(t *testing.T, f signinFile, what string, err error, key string) {
	t.Helper()
	var refused *RefusedError
	if !errors.As(err, &refused) || refused.Text.Key != key {
		t.Errorf("%s: refused %v, want %q", what, err, key)
		return
	}
	if refused.Text.Template != f.Refusals[key] {
		t.Errorf("%s: English %q, want %q", what, refused.Text.Template, f.Refusals[key])
	}
}

func TestSigninPeopleSignInAndOutAsEveryRuntimeDoes(t *testing.T) {
	f := readSigninFile(t)
	defer func() { Secrets = nil }()
	for _, c := range f.Cases {
		db := peopleDb(t, f, c.Rows)
		// remember N days keeps its entry in the secret store, here in memory, empty per case.
		source := f.source()
		source.RememberSeconds = c.Remember * 24 * 3600
		// remember N days when asked keeps it only when the input remember is "true".
		source.RememberAsks = c.Asks
		store := &countingStore{kept: map[string]string{}}
		Secrets = store
		must(t, db.AddSignin(source, testAuth()))
		// The session cookie each browser carries, as the program last set it.
		cookies := map[string]string{}
		for at, step := range c.Steps {
			what := fmt.Sprintf("%s, step %d", c.Name, at+1)
			browser := "a"
			_ = json.Unmarshal(step["browser"], &browser)
			call := func(run func() bool) {
				if !c.Served {
					run()
					return
				}
				set := db.Served(cookies[browser], c.Address, func(int64) bool { return run() })
				if set != "" {
					cookies[browser] = SessionOf(strings.Split(set, ";")[0])
				}
			}
			var strategy string
			var count *int64
			var key string
			_ = json.Unmarshal(step["refused"], &key)
			switch {
			case json.Unmarshal(step["signin"], &strategy) == nil && strategy != "":
				var inputs map[string]string
				_ = json.Unmarshal(step["inputs"], &inputs)
				var id int64
				var err error
				call(func() bool { id, err = db.Signin(strategy, inputs); return err == nil })
				if key != "" {
					refusedAs(t, f, what, err, key)
					continue
				}
				var want int64
				must(t, json.Unmarshal(step["me"], &want))
				if err != nil || id != want {
					t.Errorf("%s: signed in %d (%v), want %d", what, id, err, want)
				}
			case step["hashed"] != nil:
				var given struct {
					Method, Text string
					Minimum      *int
				}
				must(t, json.Unmarshal(step["hashed"], &given))
				var minimum []int
				if given.Minimum != nil {
					minimum = append(minimum, *given.Minimum)
				}
				kept, err := Hashed(testAuth(), given.Method, given.Text, minimum...)
				if key != "" {
					refusedAs(t, f, what, err, key)
					var slot int64
					_ = json.Unmarshal(step["minimum"], &slot)
					var refused *RefusedError
					if errors.As(err, &refused) && refused.Text.Values["minimum"].Value != slot {
						t.Errorf("%s: minimum %v, want %d", what, refused.Text.Values["minimum"].Value, slot)
					}
					continue
				}
				if err != nil || kept == "" || kept == given.Text {
					t.Errorf("%s: kept %q (%v)", what, kept, err)
				}
			case step["save"] != nil:
				var changes map[string]json.RawMessage
				must(t, json.Unmarshal(step["save"], &changes))
				var id int64
				must(t, json.Unmarshal(changes["id"], &id))
				call(func() bool {
					person, err := Get[member](db, id)
					must(t, err)
					for at, column := range memberColumns {
						if raw, ok := changes[column]; ok {
							value := literal(t, raw)
							person.values[at] = strings.ReplaceAll(strings.TrimSuffix(strings.TrimPrefix(value, "'"), "'"), "''", "'")
						}
					}
					must(t, Save(db, &person))
					return true
				})
			case step["signout"] != nil:
				call(func() bool { db.Signout(); return true })
			case step["me"] != nil:
				var want *int64
				must(t, json.Unmarshal(step["me"], &want))
				var me int64
				call(func() bool { me = db.MeID(); return true })
				if (want == nil && me != 0) || (want != nil && me != *want) {
					t.Errorf("%s: me %d, want %s", what, me, step["me"])
				}
			case step["restart"] != nil:
				var page *string
				must(t, json.Unmarshal(step["page"], &page))
				must(t, db.AddSignin(source, testAuth()))
				got := db.SigninFirst()
				if (page == nil && got != "") || (page != nil && got != *page) {
					t.Errorf("%s: starts with %q, want %s", what, got, step["page"])
				}
			case step["remembered"] != nil:
				var want bool
				must(t, json.Unmarshal(step["remembered"], &want))
				if got := store.Get(source.Program, Remembered) != ""; got != want {
					t.Errorf("%s: remembered %v, want %v", what, got, want)
				}
			case json.Unmarshal(step["count"], &count) == nil && count != nil:
				if got := countPeople(t, db, f.People.Table); got != *count {
					t.Errorf("%s: %d people, want %d", what, got, *count)
				}
			default:
				t.Fatalf("%s: no step %v", what, step)
			}
		}
	}
}

func TestSigninAllowAndYoursRefuseInClaritysOwnWords(t *testing.T) {
	f := readSigninFile(t)
	if Allow(true) != nil || Yours(true) != nil {
		t.Fatal("a condition that holds refused")
	}
	for key, err := range map[string]error{"notAllowed": Allow(false), "notYours": Yours(false)} {
		refusedAs(t, f, key, err, key)
	}
}

func TestSigninHashedKeepsTheHashOfWhatCodeAssigns(t *testing.T) {
	kept, err := Hashed(testAuth(), "argon2id", "typed")
	if err != nil || kept == "" || kept == "typed" {
		t.Fatalf("kept %q (%v)", kept, err)
	}
	if token, err := NewToken(testAuth()); err != nil || token == "" {
		t.Fatalf("token %q (%v)", token, err)
	}
}

func TestSigninAServedCallWithoutASessionSignsNobodyIn(t *testing.T) {
	f := readSigninFile(t)
	db := peopleDb(t, f, nil)
	if db.SigninPage() != f.Signin.Page {
		t.Fatalf("page %q", db.SigninPage())
	}
	var seen int64 = -1
	if set := db.Served("made-up", "192.0.2.50", func(me int64) bool { seen = me; return true }); set != "" || seen != 0 {
		t.Fatalf("a made-up session: me %d, cookie %q", seen, set)
	}
	if SessionOf("theme=dark; clarity_session=abc ; x=y") != "abc" || SessionOf("theme=dark") != "" {
		t.Fatal("the session of a cookie header")
	}
}

func TestSigninTheProgramStartsWithTheSigninWindowUntilSomeoneSignedIn(t *testing.T) {
	f := readSigninFile(t)
	db := peopleDb(t, f, []map[string]json.RawMessage{{"name": json.RawMessage(`"Gus"`), "email": json.RawMessage(`"gus@go.signin.test"`), "password_hash": json.RawMessage(`{"hash":"argon2id","text":"pw"}`)}})
	// The desktop: the computer's user has no record, so the window shows.
	if db.SigninFirst() != f.Signin.Page {
		t.Fatalf("first %q", db.SigninFirst())
	}
	if _, err := db.Signin("password", map[string]string{"name": "gus@go.signin.test", "password": "pw"}); err != nil {
		t.Fatal(err)
	}
	if db.SigninFirst() != "" || db.MeID() != 1 {
		t.Fatalf("after signing in: first %q, me %d", db.SigninFirst(), db.MeID())
	}
	// Served: per call, by its session.
	var first string
	db.Served("", "", func(int64) bool { first = db.SigninFirst(); return true })
	if first != f.Signin.Page {
		t.Fatalf("served first %q", first)
	}
	// A program without people starts as it did.
	plain, err := Memory()
	must(t, err)
	defer plain.Close()
	if plain.SigninFirst() != "" || plain.SigninPage() != "" || plain.MeID() != 0 {
		t.Fatal("a program without people asks for a sign-in")
	}
}

func TestSigninTheDesktopAccountSignsInByItself(t *testing.T) {
	f := readSigninFile(t)
	db := peopleDb(t, f, nil)
	// No people yet: the account strategy makes the computer's user the first.
	if db.SigninFirst() != "" || db.MeID() != 1 {
		t.Fatalf("first %q, me %d", db.SigninFirst(), db.MeID())
	}
	db.Signout()
	if db.SigninFirst() != "" || db.MeID() != 1 {
		t.Fatalf("again: first %q, me %d", db.SigninFirst(), db.MeID())
	}
}

func TestSigninAServedSigninSetsTheCookieAndSignoutClearsIt(t *testing.T) {
	f := readSigninFile(t)
	db := peopleDb(t, f, []map[string]json.RawMessage{{"name": json.RawMessage(`"Hal"`), "email": json.RawMessage(`"hal@go.signin.test"`), "password_hash": json.RawMessage(`{"hash":"argon2id","text":"pw"}`)}})
	set := db.Served("", "192.0.2.60", func(int64) bool {
		_, err := db.Signin("password", map[string]string{"name": "hal@go.signin.test", "password": "pw"})
		return err == nil
	})
	want := fmt.Sprintf("; HttpOnly; SameSite=Strict; Path=/; Max-Age=%d", f.Signin.Session*60)
	if !strings.HasPrefix(set, SessionCookie+"=") || !strings.HasSuffix(set, want) {
		t.Fatalf("cookie %q", set)
	}
	token := SessionOf(strings.Split(set, ";")[0])
	var me int64
	db.Served(token, "192.0.2.60", func(m int64) bool { me = m; return true })
	if me != 1 || db.MeID() != 0 {
		t.Fatalf("the session names %d, the desktop %d", me, db.MeID())
	}
	cleared := db.Served(token, "192.0.2.60", func(int64) bool { db.Signout(); return true })
	if cleared != SessionCookie+"=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0" {
		t.Fatalf("cleared %q", cleared)
	}
	db.Served(token, "192.0.2.60", func(m int64) bool { me = m; return true })
	if me != 0 {
		t.Fatal("a closed session still names its person")
	}
}

func TestSigninAddSigninRefusesWhatClarityAuthDoesNotKnow(t *testing.T) {
	db, err := Memory()
	must(t, err)
	defer db.Close()
	if err := db.AddSignin(SigninSource{Table: "p", Strategies: []SigninStrategy{{Name: "pigeon", Columns: []string{"a"}}}}, testAuth()); err == nil {
		t.Fatal("an unknown strategy was taken")
	}
	if err := db.AddSignin(SigninSource{Table: "p", Strategies: []SigninStrategy{{Name: "password", Columns: []string{"a"}}}}, testAuth()); err == nil {
		t.Fatal("a strategy with too few fields was taken")
	}
	if err := db.AddSignin(SigninSource{Table: "p"}, func(string, string) (string, error) { return "", errors.New("no library") }); err == nil {
		t.Fatal("no library was taken")
	}
}

func TestSigninTheCommandLineSignsInBeforeACommandRuns(t *testing.T) {
	f := readSigninFile(t)
	source := f.source()
	source.Targets = []string{"cli"}
	db, err := Memory()
	must(t, err)
	defer db.Close()
	must(t, db.Batch("", f.People.Create))
	robot, err := Hashed(testAuth(), "sha256", "go-cli-robot")
	must(t, err)
	ivy, err := Hashed(testAuth(), "argon2id", "ivy-pw")
	must(t, err)
	must(t, db.Batch("", fmt.Sprintf("INSERT INTO members (name, api_token) VALUES ('Robot', '%s'); INSERT INTO members (name, email, password_hash) VALUES ('Ivy', 'ivy@go.signin.test', '%s')", robot, ivy)))
	must(t, db.AddSignin(source, testAuth()))
	none := func(string) string { return "" }
	typed := func(text string) func() (string, error) { return func() (string, error) { return text, nil } }

	flags, rest := CliSigninFlags([]string{"requests", "--user", "ivy@go.signin.test", "overdue", "--token=x"})
	if flags.User != "ivy@go.signin.test" || flags.Token != "x" || strings.Join(rest, " ") != "requests overdue" {
		t.Fatalf("flags %+v, rest %v", flags, rest)
	}
	if id, err := db.SigninCli(CliSignin{Token: "go-cli-robot"}, none, nil); err != nil || id != 1 {
		t.Fatalf("token: %d (%v)", id, err)
	}
	env := func(name string) string { return map[string]string{"CLARITY_TOKEN": "go-cli-robot"}[name] }
	if id, err := db.SigninCli(CliSignin{}, env, nil); err != nil || id != 1 {
		t.Fatalf("CLARITY_TOKEN: %d (%v)", id, err)
	}
	if id, err := db.SigninCli(CliSignin{User: "ivy@go.signin.test"}, none, typed("ivy-pw")); err != nil || id != 2 || db.MeID() != 2 {
		t.Fatalf("password: %d (%v)", id, err)
	}
	if _, err := db.SigninCli(CliSignin{User: "ivy@go.signin.test"}, none, typed("wrong")); refusedKey(err) != "signinRefused" {
		t.Fatalf("a wrong password: %v", err)
	}
	// The computer's user has no record and there are people: refused.
	if _, err := db.SigninCli(CliSignin{}, none, nil); refusedKey(err) != "signinRefused" {
		t.Fatalf("account: %v", err)
	}
	// Not on the command line: nothing to sign in.
	desk := f.source()
	desk.Targets = []string{"desktop"}
	must(t, db.AddSignin(desk, testAuth()))
	if id, err := db.SigninCli(CliSignin{}, none, nil); id != 0 || err != nil {
		t.Fatalf("off the command line: %d (%v)", id, err)
	}
}
