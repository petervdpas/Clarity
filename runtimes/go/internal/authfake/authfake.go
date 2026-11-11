// Package authfake is a stand-in for Clarity.Auth in the runtime's tests
// that run without cgo: the same operations with the same JSON (the
// catalog of clarity_auth.h), readable hashes ("fake$" in place of
// Argon2id and SHA-256, "lines$" for one-time codes) and the computer's
// account always "tester". The real library runs in the tests built with
// the tag clarityauth.
package authfake

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"sync"
	"unicode/utf8"
)

// Account is the computer's account the fake says the program runs as.
const Account = "tester"

const catalog = `[
 {"name":"password","fields":[{"role":"match","kinds":["text","email"]},{"role":"secret","kinds":["text"],"hash":"argon2id"}],"inputs":["name","password"],"targets":["desktop","web","cli"],"firstAccount":true},
 {"name":"account","fields":[{"role":"match","kinds":["text"]}],"inputs":[],"targets":["desktop","cli"],"firstAccount":true},
 {"name":"token","fields":[{"role":"secret","kinds":["text"],"hash":"sha256"}],"inputs":["token"],"targets":["desktop","web","cli"],"firstAccount":false},
 {"name":"recovery","fields":[{"role":"match","kinds":["text","email"]},{"role":"secret","kinds":["text"],"hash":"sha256-lines"}],"inputs":["name","code"],"targets":["desktop","web","cli"],"firstAccount":false}
]`

// Hash is what the fake keeps of a secret.
func Hash(text string) string { return "fake$" + text }

// Lines is what the fake keeps of one-time codes, one per line.
func Lines(text string) string {
	var codes []string
	for _, line := range strings.Split(text, "\n") {
		if code := strings.TrimSpace(line); code != "" {
			codes = append(codes, code)
		}
	}
	return "lines$" + strings.Join(codes, ",")
}

// withoutCode is stored without code, when it holds it.
func withoutCode(code, stored string) (string, bool) {
	codes, ok := strings.CutPrefix(stored, "lines$")
	if !ok {
		return "", false
	}
	var left []string
	found := false
	for _, c := range strings.Split(codes, ",") {
		if c == code && !found {
			found = true
			continue
		}
		if c != "" {
			left = append(left, c)
		}
	}
	return "lines$" + strings.Join(left, ","), found
}

// Auth is one fake library: its failed tries and sessions.
type Auth struct {
	mu       sync.Mutex
	tries    map[string]int
	sessions map[string]int64
	made     int
}

func New() *Auth { return &Auth{tries: map[string]int{}, sessions: map[string]int64{}} }

type args struct {
	Strategy string            `json:"strategy"`
	Inputs   map[string]string `json:"inputs"`
	Stored   map[string]string `json:"stored"`
	Method   string            `json:"method"`
	Text     string            `json:"text"`
	Key      string            `json:"key"`
	Tries    int               `json:"tries"`
	Record   int64             `json:"record"`
	Token    string            `json:"token"`
	Count    int               `json:"count"`
	Minimum  int               `json:"minimum"`
}

// Call is the fake's AuthCall.
func (a *Auth) Call(op, given string) (string, error) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if strings.HasPrefix(op, "remember") {
		return remember(op, given)
	}
	var in args
	if err := json.Unmarshal([]byte(given), &in); err != nil {
		return "", err
	}
	input := func(name string) (string, error) {
		v, ok := in.Inputs[name]
		if !ok {
			return "", fmt.Errorf("the %s strategy needs the input '%s'", in.Strategy, name)
		}
		return v, nil
	}
	out := func(v any) (string, error) {
		made, err := json.Marshal(v)
		return string(made), err
	}
	switch op {
	case "catalog":
		return catalog, nil
	case "hash":
		if in.Method == "sha256-lines" {
			return out(Lines(in.Text))
		}
		return out(Hash(in.Text))
	case "secretFits":
		return out(utf8.RuneCountInString(in.Text) >= in.Minimum)
	case "newRecoveryCodes":
		var codes []string
		for range max(in.Count, 1) {
			a.made++
			codes = append(codes, fmt.Sprintf("code-%d", a.made))
		}
		return out(strings.Join(codes, "\n"))
	case "afterSignin":
		changed := map[string]string{}
		if in.Strategy == "recovery" {
			if left, ok := withoutCode(in.Inputs["code"], in.Stored["secret"]); ok {
				changed["secret"] = left
			}
		}
		return out(changed)
	case "newToken":
		a.made++
		return out(fmt.Sprintf("token-%d", a.made))
	case "find", "firstAccount":
		var role, equals string
		var made map[string]string
		switch in.Strategy {
		case "password":
			name, err := input("name")
			if err != nil {
				return "", err
			}
			password, err := input("password")
			if err != nil {
				return "", err
			}
			role, equals, made = "match", name, map[string]string{"match": name, "secret": Hash(password)}
		case "account":
			role, equals, made = "match", Account, map[string]string{"match": Account}
		case "token":
			token, err := input("token")
			if err != nil {
				return "", err
			}
			role, equals = "secret", Hash(token)
		case "recovery":
			name, err := input("name")
			if err != nil {
				return "", err
			}
			if _, err := input("code"); err != nil {
				return "", err
			}
			role, equals = "match", name
		default:
			return "", errors.New("there is no sign-in strategy " + in.Strategy)
		}
		if op == "firstAccount" {
			if made == nil {
				return "", errors.New("the strategy cannot make the first account")
			}
			return out(made)
		}
		return out(map[string]string{"role": role, "equals": equals})
	case "check":
		switch in.Strategy {
		case "password":
			return out(in.Stored["secret"] == Hash(in.Inputs["password"]))
		case "token":
			return out(in.Stored["secret"] == Hash(in.Inputs["token"]))
		case "recovery":
			_, ok := withoutCode(in.Inputs["code"], in.Stored["secret"])
			return out(ok)
		}
		return out(true)
	case "triesAllowed":
		return out(a.tries[in.Key] < in.Tries)
	case "triesFailed":
		a.tries[in.Key]++
		return "null", nil
	case "triesCleared":
		delete(a.tries, in.Key)
		return "null", nil
	case "sessionOpen":
		a.made++
		token := fmt.Sprintf("session-%d", a.made)
		a.sessions[token] = in.Record
		return out(token)
	case "sessionFind":
		return out(a.sessions[in.Token])
	case "sessionClose":
		delete(a.sessions, in.Token)
		return "null", nil
	case "sessionCloseOthers":
		for token, record := range a.sessions {
			if record == in.Record && token != in.Token {
				delete(a.sessions, token)
			}
		}
		return "null", nil
	case "sessionCloseAll":
		for token, record := range a.sessions {
			if record == in.Record {
				delete(a.sessions, token)
			}
		}
		return "null", nil
	}
	return "", errors.New("Clarity.Auth has no operation " + op)
}

// remember answers rememberMake, rememberRecord and rememberCheck: an entry
// readable in place of signed, "remember$<record>$<stored>", that holds
// while the person's stored secret is the same (no expiry in the fake).
func remember(op, given string) (string, error) {
	var in struct {
		Record int64  `json:"record"`
		Stored string `json:"stored"`
		Entry  string `json:"entry"`
	}
	if err := json.Unmarshal([]byte(given), &in); err != nil {
		return "", err
	}
	parts := strings.SplitN(in.Entry, "$", 3)
	var record int64
	if len(parts) == 3 && parts[0] == "remember" {
		fmt.Sscan(parts[1], &record)
	}
	switch op {
	case "rememberMake":
		if in.Stored == "" {
			return "", errors.New("a sign-in is remembered only for a strategy that keeps a secret")
		}
		made, err := json.Marshal(fmt.Sprintf("remember$%d$%s", in.Record, in.Stored))
		return string(made), err
	case "rememberRecord":
		return fmt.Sprint(record), nil
	case "rememberCheck":
		if record == 0 || in.Stored == "" || parts[2] != in.Stored {
			return "0", nil
		}
		return fmt.Sprint(record), nil
	}
	return "", errors.New("Clarity.Auth has no operation " + op)
}
