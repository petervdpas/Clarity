// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4). The
// generated program names its entity of people and its strategies once,
// after its databases open (Db.AddSignin), with the function that reaches
// Clarity.Auth (an AuthCall, its cgo binding in the program). Db.Signin
// then finds the person by what Clarity.Auth says, checks them and makes
// them the current person. On the desktop that is one person for the
// program; served, each call is the person of its session cookie
// (Db.Served), so Me is read per call. Refusals are Clarity's own words, by
// key (conformance/signin.json).
//
// The first account: when the table of people has no rows and the strategy
// can make one (its catalog entry says firstAccount), the record is
// inserted with only the fields the strategy names (the match field, the
// secret as its hash); every other column takes its default from the
// table, so a required field without a default makes the insert fail.

package clarity

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

// SessionCookie is the name of the session cookie a served program sets.
const SessionCookie = "clarity_session"

// Remembered is the name of the remembered desktop sign-in among the program's secrets.
const Remembered = "clarityRemembered"

// AuthCall is one operation of Clarity.Auth by name, its arguments and its
// result as JSON (clarity_auth.h): the program's cgo binding of
// clarity_auth_api()->call.
type AuthCall func(op, args string) (string, error)

// SigninStrategy is one strategy the project uses: its name in Clarity.Auth's
// catalog and the columns of the table of people it names, in the catalog's
// order of roles (`password email passwordHash` is ["email", "password_hash"]).
type SigninStrategy struct {
	Name    string
	Columns []string
}

// SigninSource is the project's signin block as the program uses it: where
// its people are kept, how they sign in, how long a sign-in lasts and how
// many failed tries are allowed. A zero Key is "id", zero SessionSeconds 8
// hours, zero Tries and TriesSeconds 5 per 15 minutes, no Targets the web.
type SigninSource struct {
	// Source is the data source of the entity of people; "" for the first.
	Source string
	Table  string
	Key    string
	// Page is the window of the kind signin.
	Page string
	// Targets are where people sign in: desktop, web, cli.
	Targets        []string
	Strategies     []SigninStrategy
	SessionSeconds int64
	Tries          int
	TriesSeconds   int64
	// Program is the program's name, under which its secrets are kept (the remembered sign-in).
	Program string
	// RememberSeconds is how long the desktop program keeps a sign-in
	// between runs (`remember N days`) in the system's protected store
	// (Secrets); 0 without the line.
	RememberSeconds int64
	// RememberAsks is whether the sign-in part asks whether to remember
	// (`remember N days when asked`): then only a sign-in whose input
	// remember is "true" is remembered; else every one is.
	RememberAsks bool
}

func (s SigninSource) on(target string) bool {
	for _, t := range s.Targets {
		if t == target {
			return true
		}
	}
	return false
}

// authStrategy is a strategy of Clarity.Auth's catalog.
type authStrategy struct {
	Name   string `json:"name"`
	Fields []struct {
		Role string `json:"role"`
		Hash string `json:"hash"`
	} `json:"fields"`
	Inputs       []string `json:"inputs"`
	Targets      []string `json:"targets"`
	FirstAccount bool     `json:"firstAccount"`
}

// people is what the database handle knows of who is using the program.
type people struct {
	source  SigninSource
	call    AuthCall
	catalog []authStrategy
	// me is the person signed in on the desktop or the command line; 0 for nobody.
	me int64
	// cli is set once a command signed in (SigninCli): it signs in as cli, not desktop.
	cli bool
	// visit is the served call answered now, nil off the web.
	visit *Visit
}

// Visit is one served call: its person, the session and the address it came
// with, and what signing in or out changed of its session.
type Visit struct {
	me      int64
	token   string
	address string
	opened  string
	seconds int64
	closed  bool
}

// SigninRefused is Clarity's refusal when signing in did not work.
func SigninRefused() error {
	return Refuse(KeyedText("signinRefused", "Signing in did not work. Check what you typed and try again."))
}

// SigninTooManyTries is Clarity's refusal when too many tries failed.
func SigninTooManyTries() error {
	return Refuse(KeyedText("signinTooManyTries", "Too many tries. Wait a few minutes and try again."))
}

// NotAllowed is Clarity's refusal when `allow when` does not hold, or nobody is signed in.
func NotAllowed() error { return Refuse(KeyedText("notAllowed", "You are not allowed to do this.")) }

// NotYours is Clarity's refusal when `readable when` or `writable when` does not hold.
func NotYours() error { return Refuse(KeyedText("notYours", "This record is not yours.")) }

// Allow is `allow when <condition>` on an action, command or window:
// refused with notAllowed when it does not hold.
func Allow(holds bool) error {
	if holds {
		return nil
	}
	return NotAllowed()
}

// Yours is `readable when` or `writable when` of an entity, checked on a
// find or a save: refused with notYours when it does not hold.
func Yours(holds bool) error {
	if holds {
		return nil
	}
	return NotYours()
}

// authCall calls op of Clarity.Auth and reads its result into into.
func authCall(call AuthCall, op string, args any, into any) error {
	given, err := json.Marshal(args)
	if err != nil {
		return err
	}
	made, err := call(op, string(given))
	if err != nil {
		return err
	}
	if into == nil {
		return nil
	}
	return json.Unmarshal([]byte(made), into)
}

// SecretTooShort is Clarity's refusal of a secret shorter than its field's minimum.
func SecretTooShort(minimum int) error {
	return Refuse(KeyedText("secretTooShort", "This needs at least {minimum} characters.").With("minimum", SlotInteger(int64(minimum), "")))
}

// Hashed is text assigned to a hashed field: what is stored, by the field's
// method (argon2id; sha256 for a token field, sha256-lines for a recovery
// field). With the field's minimum, a shorter text is refused with
// secretTooShort before anything is hashed.
func Hashed(call AuthCall, method, text string, minimum ...int) (string, error) {
	for _, least := range minimum {
		var fits bool
		if err := authCall(call, "secretFits", map[string]any{"text": text, "minimum": least}, &fits); err != nil {
			return "", &ValueError{What: err.Error()}
		}
		if !fits {
			return "", SecretTooShort(least)
		}
	}
	var kept string
	if err := authCall(call, "hash", map[string]string{"method": method, "text": text}, &kept); err != nil {
		return "", &ValueError{What: err.Error()}
	}
	return kept, nil
}

// NewRecoveryCodes is newRecoveryCodes(count) in code: one-time codes, one
// per line, to show the person once and assign to a recovery field.
func NewRecoveryCodes(call AuthCall, count int64) (string, error) {
	var codes string
	if err := authCall(call, "newRecoveryCodes", map[string]any{"count": max(count, 0)}, &codes); err != nil {
		return "", &ValueError{What: err.Error()}
	}
	return codes, nil
}

// NewToken is newToken() in code.
func NewToken(call AuthCall) (string, error) {
	var token string
	if err := authCall(call, "newToken", map[string]any{}, &token); err != nil {
		return "", &ValueError{What: err.Error()}
	}
	return token, nil
}

// AddSignin names the people of the program and how they sign in, after
// the databases opened. Fails when Clarity.Auth cannot be reached or does
// not know a strategy, or a strategy names more columns than it has roles.
func (db *Db) AddSignin(source SigninSource, call AuthCall) error {
	if source.Key == "" {
		source.Key = "id"
	}
	if source.SessionSeconds <= 0 {
		source.SessionSeconds = 8 * 3600
	}
	if source.Tries <= 0 {
		source.Tries = 5
	}
	if source.TriesSeconds <= 0 {
		source.TriesSeconds = 15 * 60
	}
	if len(source.Targets) == 0 {
		source.Targets = []string{"web"}
	}
	var catalog []authStrategy
	if err := authCall(call, "catalog", map[string]any{}, &catalog); err != nil {
		return &StorageError{What: "Clarity.Auth: " + err.Error()}
	}
	for _, used := range source.Strategies {
		info := strategyOf(catalog, used.Name)
		if info == nil {
			return &StorageError{What: fmt.Sprintf("Clarity.Auth has no sign-in strategy '%s'", used.Name)}
		}
		if len(used.Columns) != len(info.Fields) {
			return &StorageError{What: fmt.Sprintf("the sign-in strategy %s names %d fields, not %d", used.Name, len(info.Fields), len(used.Columns))}
		}
	}
	db.people = &people{source: source, call: call, catalog: catalog}
	return nil
}

func strategyOf(catalog []authStrategy, name string) *authStrategy {
	for i := range catalog {
		if catalog[i].Name == name {
			return &catalog[i]
		}
	}
	return nil
}

// SigninPage is the sign-in window of a program whose people sign in on
// the web: a served call without a session reaches only it. "" for none.
func (db *Db) SigninPage() string {
	if db == nil || db.people == nil || !db.people.source.on("web") {
		return ""
	}
	return db.people.source.Page
}

// target is where the program answers now: web while serving a call, else
// the command line once a command signed in, else the desktop.
func (db *Db) target() string {
	switch {
	case db.people.visit != nil:
		return "web"
	case db.people.cli:
		return "cli"
	default:
		return "desktop"
	}
}

// MeID is the record signed in for this call: served, the one of the
// call's session; else the one of the program. 0 for nobody.
func (db *Db) MeID() int64 {
	if db == nil || db.people == nil {
		return 0
	}
	if v := db.people.visit; v != nil {
		return v.me
	}
	return db.people.me
}

// Me is `me` in code: the record signed in now; refused with notAllowed
// when nobody is.
func Me[T any, P interface {
	*T
	Record
}](db *Db) (T, error) {
	id := db.MeID()
	if id == 0 {
		var none T
		return none, NotAllowed()
	}
	return Get[T, P](db, id)
}

// SigninFirst is the sign-in window the program starts with, as the
// start's signin says it: "" when it names no people, when its people do
// not sign in where it runs (web served, else desktop), or when someone is
// signed in. On the desktop a sign-in remembered (`remember N days`) that
// still holds signs its person in, else the account strategy signs the
// computer's user in by itself when they have a record (no try counted).
func (db *Db) SigninFirst() string {
	if db == nil || db.people == nil {
		return ""
	}
	p := db.people
	target := db.target()
	if !p.source.on(target) || db.MeID() != 0 {
		return ""
	}
	if target == "desktop" {
		if id := db.remembered(); id > 0 {
			p.me = id
			return ""
		}
	}
	if target == "desktop" && usesStrategy(p.source, "account") {
		if _, err := db.attempt("account", map[string]string{}, false); err == nil {
			return ""
		}
	}
	return p.source.Page
}

func usesStrategy(source SigninSource, name string) bool {
	for _, s := range source.Strategies {
		if s.Name == name {
			return true
		}
	}
	return false
}

// Signin signs a person in with strategy and its inputs (name, password,
// token, as the catalog names them): the record signed in, or Clarity's
// refusal (signinRefused, signinTooManyTries). Failed tries count per
// account and, served, per address.
func (db *Db) Signin(strategy string, inputs map[string]string) (int64, error) {
	if db == nil || db.people == nil {
		return 0, &StorageError{What: "the program names no people who sign in"}
	}
	return db.attempt(strategy, inputs, true)
}

func (db *Db) attempt(strategy string, inputs map[string]string, counted bool) (int64, error) {
	p := db.people
	target := db.target()
	// Only a strategy the project names, on a target the catalog lets it work on.
	var used *SigninStrategy
	for i := range p.source.Strategies {
		if p.source.Strategies[i].Name == strategy {
			used = &p.source.Strategies[i]
		}
	}
	info := strategyOf(p.catalog, strategy)
	if used == nil || info == nil || !contains(info.Targets, target) {
		return 0, SigninRefused()
	}
	if inputs == nil {
		inputs = map[string]string{}
	}
	var found struct {
		Role   string `json:"role"`
		Equals string `json:"equals"`
	}
	if authCall(p.call, "find", map[string]any{"strategy": strategy, "inputs": inputs}, &found) != nil {
		return 0, SigninRefused()
	}
	keys := []string{"account:" + found.Equals}
	if v := p.visit; v != nil && v.address != "" {
		keys = append(keys, "address:"+v.address)
	}
	if counted {
		for _, key := range keys {
			var allowed bool
			if err := authCall(p.call, "triesAllowed", map[string]any{"key": key, "tries": p.source.Tries, "window": p.source.TriesSeconds}, &allowed); err != nil || !allowed {
				return 0, SigninTooManyTries()
			}
		}
	}
	failed := func() error {
		if counted {
			for _, key := range keys {
				_ = authCall(p.call, "triesFailed", map[string]string{"key": key}, nil)
			}
		}
		return SigninRefused()
	}
	column := func(role string) string {
		for at, f := range info.Fields {
			if f.Role == role && at < len(used.Columns) {
				return used.Columns[at]
			}
		}
		return ""
	}
	by := column(found.Role)
	if by == "" {
		return 0, failed()
	}
	id, stored, has, err := db.personBy(by, column("secret"), found.Equals)
	if err != nil {
		return 0, err
	}
	switch {
	case has:
		kept := map[string]string{"secret": stored}
		var ok bool
		if authCall(p.call, "check", map[string]any{"strategy": strategy, "inputs": inputs, "stored": kept}, &ok) != nil || !ok {
			return 0, failed()
		}
		// What signing in changes of what is stored: a recovery code is used up.
		var changed map[string]string
		if authCall(p.call, "afterSignin", map[string]any{"strategy": strategy, "inputs": inputs, "stored": kept}, &changed) != nil {
			return 0, failed()
		}
		if err := db.storeAfter(info, used, id, changed); err != nil {
			return 0, err
		}
	case info.FirstAccount:
		none, err := db.noPeople()
		if err != nil {
			return 0, err
		}
		if !none {
			return 0, failed()
		}
		if id, err = db.firstAccount(info, used, strategy, inputs); err != nil {
			return 0, failed()
		}
	default:
		return 0, failed()
	}
	if counted {
		for _, key := range keys {
			_ = authCall(p.call, "triesCleared", map[string]string{"key": key}, nil)
		}
	}
	db.signedIn(id)
	if target == "desktop" && p.source.remembers(inputs) {
		db.remember(column("secret"), id)
	}
	return id, nil
}

// remembered is the person of the remembered sign-in when it still holds:
// the entry names a record whose stored secret of some strategy it was
// signed with, and it has not expired. 0 for anything else (no entry, no
// store, the record gone, the secret changed).
func (db *Db) remembered() int64 {
	p := db.people
	if p.source.RememberSeconds <= 0 || Secrets == nil {
		return 0
	}
	entry := Secrets.Get(p.source.Program, Remembered)
	if entry == "" {
		return 0
	}
	var record int64
	if authCall(p.call, "rememberRecord", map[string]string{"entry": entry}, &record) != nil || record <= 0 {
		return 0
	}
	for _, column := range p.secretColumns() {
		stored := db.storedSecret(column, record)
		if stored == "" {
			continue
		}
		var holds int64
		if authCall(p.call, "rememberCheck", map[string]string{"entry": entry, "stored": stored}, &holds) == nil && holds == record {
			return record
		}
	}
	return 0
}

// remembers is whether a sign-in with inputs is remembered: always, unless
// the sign-in part asks (when asked) and its box's answer is not "true".
func (s SigninSource) remembers(inputs map[string]string) bool {
	return !s.RememberAsks || inputs["remember"] == "true"
}

// remember keeps, after a sign-in on the desktop with `remember N days`, an
// entry signed with the person's stored secret in column (the strategy
// used) in the system's protected store. A strategy without a secret
// (account) remembers nothing; a store that cannot keep it leaves the
// sign-in as it is.
func (db *Db) remember(column string, id int64) {
	p := db.people
	if p.source.RememberSeconds <= 0 || column == "" || Secrets == nil {
		return
	}
	// Read after signing in: what signing in changed (a recovery code used up) is what holds.
	stored := db.storedSecret(column, id)
	if stored == "" {
		return
	}
	var entry string
	if authCall(p.call, "rememberMake", map[string]any{"record": id, "seconds": p.source.RememberSeconds, "stored": stored}, &entry) == nil {
		_ = Secrets.Set(p.source.Program, Remembered, entry)
	}
}

// storedSecret is the stored secret in column of the people's record id;
// "" when the record is gone, the column empty or the database cannot be read.
func (db *Db) storedSecret(column string, id int64) string {
	p := db.people
	s, err := db.source(p.source.Source)
	if err != nil {
		return ""
	}
	d := s.driver.dialect()
	rows, err := s.driver.rows("SELECT "+d.Quote(column)+" FROM "+d.Quote(p.source.Table)+" WHERE "+d.Quote(p.source.Key)+" = "+d.Mark(1), []any{id})
	if err != nil || len(rows) == 0 || len(rows[0]) == 0 {
		return ""
	}
	switch v := rows[0][0].(type) {
	case string:
		return v
	case []byte:
		return string(v)
	}
	return ""
}

// personBy is the key and the stored secret ("" without one) of the record
// whose column equals equals; has is false when there is none.
func (db *Db) personBy(column, secret, equals string) (int64, string, bool, error) {
	p := db.people
	s, err := db.source(p.source.Source)
	if err != nil {
		return 0, "", false, err
	}
	d := s.driver.dialect()
	wanted := d.Quote(p.source.Key)
	if secret != "" {
		wanted += ", " + d.Quote(secret)
	}
	rows, err := s.driver.rows("SELECT "+wanted+" FROM "+d.Quote(p.source.Table)+" WHERE "+d.Quote(column)+" = "+d.Mark(1), []any{equals})
	if err != nil || len(rows) == 0 {
		return 0, "", false, err
	}
	id, err := wholeOf(rows[0][0])
	if err != nil {
		return 0, "", false, err
	}
	stored := ""
	if len(rows[0]) > 1 {
		switch v := rows[0][1].(type) {
		case string:
			stored = v
		case []byte:
			stored = string(v)
		}
	}
	return id, stored, true, nil
}

// storeAfter writes what signing in changed (by role) to the person's row.
func (db *Db) storeAfter(info *authStrategy, used *SigninStrategy, id int64, changed map[string]string) error {
	p := db.people
	s, err := db.source(p.source.Source)
	if err != nil {
		return err
	}
	d := s.driver.dialect()
	var sets []string
	var values []any
	for at, f := range info.Fields {
		if value, ok := changed[f.Role]; ok && at < len(used.Columns) {
			values = append(values, value)
			sets = append(sets, d.Quote(used.Columns[at])+" = "+d.Mark(len(values)))
		}
	}
	if len(sets) == 0 {
		return nil
	}
	values = append(values, id)
	_, err = s.driver.execute("UPDATE "+d.Quote(p.source.Table)+" SET "+strings.Join(sets, ", ")+" WHERE "+d.Quote(p.source.Key)+" = "+d.Mark(len(values)), values)
	return err
}

// secretColumns are the columns of the people that hold a secret, by the catalog.
func (p *people) secretColumns() []string {
	var columns []string
	for _, used := range p.source.Strategies {
		info := strategyOf(p.catalog, used.Name)
		if info == nil {
			continue
		}
		for at, f := range info.Fields {
			if f.Role == "secret" && at < len(used.Columns) && !contains(columns, used.Columns[at]) {
				columns = append(columns, used.Columns[at])
			}
		}
	}
	return columns
}

// secretsOf are the secret columns of the people's record id when table is
// the people's (before a save, to see whether it changes them); nil else.
func (db *Db) secretsOf(table string, id int64) []any {
	if db == nil || db.people == nil || db.people.source.Table != table || id == 0 {
		return nil
	}
	p := db.people
	columns := p.secretColumns()
	if len(columns) == 0 {
		return nil
	}
	s, err := db.source(p.source.Source)
	if err != nil {
		return nil
	}
	d := s.driver.dialect()
	wanted := make([]string, len(columns))
	for i, c := range columns {
		wanted[i] = d.Quote(c)
	}
	rows, err := s.driver.rows("SELECT "+strings.Join(wanted, ", ")+" FROM "+d.Quote(p.source.Table)+" WHERE "+d.Quote(p.source.Key)+" = "+d.Mark(1), []any{id})
	if err != nil || len(rows) == 0 {
		return nil
	}
	return rows[0]
}

// secretsSaved: a save of a person changed their secret (before, as
// secretsOf read it). Served, the person's other sessions close when they
// changed their own, every one of theirs when someone else did. On the
// desktop and the command line nothing: one person is signed in for the run.
func (db *Db) secretsSaved(table string, id int64, before []any) {
	if before == nil || db.people.visit == nil {
		return
	}
	if fmt.Sprint(db.secretsOf(table, id)) == fmt.Sprint(before) {
		return
	}
	p := db.people
	if v := p.visit; v.me == id && v.token != "" {
		_ = authCall(p.call, "sessionCloseOthers", map[string]any{"record": id, "token": v.token}, nil)
		return
	}
	_ = authCall(p.call, "sessionCloseAll", map[string]any{"record": id}, nil)
}

// wholeOf is a key or a count as a database gives it.
func wholeOf(value any) (int64, error) {
	switch v := value.(type) {
	case int64:
		return v, nil
	case int32:
		return int64(v), nil
	case int:
		return int64(v), nil
	case float64:
		return int64(v), nil
	case []byte:
		return strconv.ParseInt(string(v), 10, 64)
	case string:
		return strconv.ParseInt(v, 10, 64)
	}
	return 0, &StorageError{What: fmt.Sprintf("a key of the people is %v", value)}
}

func (db *Db) noPeople() (bool, error) {
	p := db.people
	s, err := db.source(p.source.Source)
	if err != nil {
		return false, err
	}
	rows, err := s.driver.rows("SELECT COUNT(*) FROM "+s.driver.dialect().Quote(p.source.Table), nil)
	if err != nil || len(rows) == 0 || len(rows[0]) == 0 {
		return false, err
	}
	n, err := wholeOf(rows[0][0])
	return n == 0, err
}

// firstAccount inserts the first person, with only the fields the strategy names.
func (db *Db) firstAccount(info *authStrategy, used *SigninStrategy, strategy string, inputs map[string]string) (int64, error) {
	p := db.people
	var made map[string]string
	if err := authCall(p.call, "firstAccount", map[string]any{"strategy": strategy, "inputs": inputs}, &made); err != nil {
		return 0, err
	}
	var columns []string
	var values []any
	for at, f := range info.Fields {
		if value, ok := made[f.Role]; ok && at < len(used.Columns) {
			columns = append(columns, used.Columns[at])
			values = append(values, value)
		}
	}
	s, err := db.source(p.source.Source)
	if err != nil {
		return 0, err
	}
	return s.driver.insert(s.driver.dialect().insert(p.source.Table, p.source.Key, columns), values)
}

func (db *Db) signedIn(id int64) {
	p := db.people
	v := p.visit
	if v == nil {
		p.me = id
		return
	}
	// A new session for the new sign-in; the one it came with is closed.
	if v.token != "" {
		_ = authCall(p.call, "sessionClose", map[string]string{"token": v.token}, nil)
	}
	var token string
	if authCall(p.call, "sessionOpen", map[string]any{"record": id, "seconds": p.source.SessionSeconds}, &token) == nil {
		v.opened, v.seconds, v.token = token, p.source.SessionSeconds, token
	}
	v.me = id
}

// Signout signs the person out: served, the call's session is closed and
// its cookie cleared; else nobody is signed in any more.
func (db *Db) Signout() {
	if db == nil || db.people == nil {
		return
	}
	p := db.people
	v := p.visit
	if v == nil {
		p.me = 0
		if p.source.RememberSeconds > 0 && Secrets != nil {
			_ = Secrets.Set(p.source.Program, Remembered, "")
		}
		return
	}
	if v.token != "" {
		_ = authCall(p.call, "sessionClose", map[string]string{"token": v.token}, nil)
	}
	v.token, v.opened, v.me, v.closed = "", "", 0, true
}

// SessionOf is the session token a request's Cookie header carries; "" for none.
func SessionOf(cookieHeader string) string {
	for _, pair := range strings.Split(cookieHeader, ";") {
		name, value, found := strings.Cut(pair, "=")
		if found && strings.TrimSpace(name) == SessionCookie {
			return strings.TrimSpace(value)
		}
	}
	return ""
}

// Served runs one served call (run) as the person of its session: token is
// the session the request's cookie carries, address where it came from.
// run gets the person (0 for nobody) and says whether the call went well.
// Gives the Set-Cookie header's value when the call signed in (a new
// session) or out (the cookie cleared), else "". The caller keeps calls
// one at a time, as the shell's commands take turns. A program without
// people just runs.
func (db *Db) Served(token, address string, run func(me int64) bool) string {
	if db == nil || db.people == nil {
		run(0)
		return ""
	}
	p := db.people
	v := &Visit{address: address}
	if token != "" {
		var me int64
		if authCall(p.call, "sessionFind", map[string]string{"token": token}, &me) == nil && me > 0 {
			v.me, v.token = me, token
		}
	}
	// The call's visit ends with it, even when run panics.
	was := p.visit
	p.visit = v
	defer func() { p.visit = was }()
	ok := run(v.me)
	switch {
	case v.opened != "" && ok:
		return fmt.Sprintf("%s=%s; HttpOnly; SameSite=Strict; Path=/; Max-Age=%d", SessionCookie, v.opened, v.seconds)
	case v.closed:
		return SessionCookie + "=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
	}
	return ""
}
