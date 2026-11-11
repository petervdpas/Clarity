// Signing in on the command line (docs/APP-CAPABILITIES-PLAN.md, step 4):
// when cli is among the signin block's targets, a command signs in before it
// runs: account by itself when the computer's user has a record, --token
// (or CLARITY_TOKEN) for token, --user name with the password read from the
// terminal without echo for password.

package clarity

import (
	"fmt"
	"os"
	"strings"
)

// CliSignin are the sign-in options of a command line.
type CliSignin struct {
	User  string
	Token string
}

// CliSigninFlags takes `--user name` and `--token value` (also written
// `--user=name`) out of args, before the command line is read against the
// program's commands: the options and the rest of args.
func CliSigninFlags(args []string) (CliSignin, []string) {
	var flags CliSignin
	rest := make([]string, 0, len(args))
	for i := 0; i < len(args); i++ {
		arg := args[i]
		into := map[string]*string{"--user": &flags.User, "--token": &flags.Token}
		name, value, written := strings.Cut(arg, "=")
		if target, ok := into[name]; ok {
			if written {
				*target = value
				continue
			}
			if i+1 < len(args) {
				*target = args[i+1]
				i++
				continue
			}
		}
		rest = append(rest, arg)
	}
	return flags, rest
}

// SigninCli signs a command in before it runs, when the program's people
// sign in on cli: the record signed in, 0 when the command line needs no
// sign-in, or Clarity's refusal. A token (flags.Token, else
// getenv("CLARITY_TOKEN")) signs in with token; a user with password, its
// password from readSecret; else the computer's user with account. Each
// only when the project names that strategy; nothing to sign in with is
// refused with signinRefused.
func (db *Db) SigninCli(flags CliSignin, getenv func(string) string, readSecret func() (string, error)) (int64, error) {
	if db == nil || db.people == nil || !db.people.source.on("cli") {
		return 0, nil
	}
	db.people.cli = true
	uses := func(name string) bool { return usesStrategy(db.people.source, name) }
	token := flags.Token
	if token == "" && getenv != nil {
		token = getenv("CLARITY_TOKEN")
	}
	switch {
	case token != "" && uses("token"):
		return db.Signin("token", map[string]string{"token": token})
	case flags.User != "" && uses("password"):
		if readSecret == nil {
			return 0, SigninRefused()
		}
		password, err := readSecret()
		if err != nil {
			return 0, SigninRefused()
		}
		return db.Signin("password", map[string]string{"name": flags.User, "password": password})
	case uses("account"):
		return db.Signin("account", map[string]string{})
	}
	return 0, SigninRefused()
}

// Signin is SigninCli for the command that runs: CLARITY_TOKEN from the
// environment, the password typed on the terminal without echo after
// Clarity's word cliPassword.
func (s *CliSession) Signin(db *Db, flags CliSignin) (int64, error) {
	return db.SigninCli(flags, os.Getenv, func() (string, error) {
		return CliReadSecret(CliWordsIn(cliTranslated)("cliPassword") + ": ")
	})
}

// CliReadSecret writes prompt to standard error and reads a line from the
// terminal without showing what is typed. Refused when standard input is
// no terminal, so a password never comes from a pipe by accident.
func CliReadSecret(prompt string) (string, error) {
	fmt.Fprint(os.Stderr, prompt)
	line, err := readHidden(os.Stdin)
	fmt.Fprintln(os.Stderr)
	if err != nil {
		return "", err
	}
	return strings.TrimRight(line, "\r\n"), nil
}
