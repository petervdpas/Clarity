// Command-line programs (DESIGN 5.20): reading a command line against the
// program's commands, writing its help, printing a table and asking on a
// terminal, as conformance/command-line.json says every runtime does. A
// generated program describes itself as a CliProgram and runs the command
// ParseCommandLine finds with the values it read.

package clarity

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/shopspring/decimal"
)

// CliWords are Clarity's own words of the command line, in English: what a
// program shows for a key its translations leave out.
var CliWords = []struct{ Key, English string }{
	{"cliUsage", "Usage"},
	{"cliCommands", "Commands"},
	{"cliOptions", "Options"},
	{"cliCommandWord", "command"},
	{"cliOptionsWord", "options"},
	{"cliHelpOption", "Shows this help"},
	{"cliDefault", "default {value}"},
	{"cliRequiredMark", "required"},
	{"cliUnknownCommand", "There is no command {command}"},
	{"cliUnknownOption", "There is no option {option}"},
	{"cliMissingArgument", "{argument} is missing"},
	{"cliTooManyArguments", "There are more values than arguments"},
	{"cliNeedsValue", "{option} needs a value"},
	{"cliWrongValue", "{name} cannot be {value}"},
	{"cliTooSmall", "{name} is at least {min}"},
	{"cliTooLarge", "{name} is at most {max}"},
	{"cliRequired", "{option} is required"},
	{"cliWrongAnswer", "{value} does not fit; try again"},
	{"cliNoTerminal", "There is nobody to answer: {question}"},
	{"cliChooseNumber", "Type a number from 1 to {count}"},
}

// CliParam is an argument or an option of a command; names as typed
// (`call-after`). Kind is text, integer, decimal, money, date, datetime,
// duration, enum (with its Values) or, for an option, bool.
type CliParam struct {
	Name     string   `json:"name"`
	Kind     string   `json:"kind"`
	Values   []string `json:"values"`
	Default  *string  `json:"default"`
	Min      *string  `json:"min"`
	Max      *string  `json:"max"`
	Required bool     `json:"required"`
	Help     *string  `json:"help"`
}

type CliCommand struct {
	Line      []string   `json:"line"`
	Help      *string    `json:"help"`
	Arguments []CliParam `json:"arguments"`
	Options   []CliParam `json:"options"`
}

type CliProgram struct {
	Name string  `json:"name"`
	Help *string `json:"help"`
	// In the order they are declared.
	Commands []CliCommand `json:"commands"`
}

// CliSay gives Clarity's word of key in the language shown, with its slots
// filled: the translation, or the English when there is none.
type CliSay func(key string, slots ...[2]string) string

// CliWordsIn makes a CliSay from the translations' values.
func CliWordsIn(translated func(key string) string) CliSay {
	return func(key string, slots ...[2]string) string {
		text := translated(key)
		if text == "" {
			text = key
			for _, w := range CliWords {
				if w.Key == key {
					text = w.English
				}
			}
		}
		for _, s := range slots {
			text = strings.ReplaceAll(text, "{"+s[0]+"}", s[1])
		}
		return text
	}
}

// CliParsed is what a command line becomes: a command to run with its
// values (Run), help to write (Help, exit code 0), or a wrong command line
// (Key and Slots of its message, Said written to standard error, exit code 2).
type CliParsed struct {
	Run     bool
	Command int
	// Every argument's value and then every option's, in their order;
	// nil for the empty value.
	Values []*string
	Help   string
	Key    string
	Slots  [][2]string
	Said   string
}

func sameStart(line []string, path []string) bool {
	if len(line) < len(path) {
		return false
	}
	for i := range path {
		if line[i] != path[i] {
			return false
		}
	}
	return true
}

// ParseCommandLine reads args, the words after the program's name.
func ParseCommandLine(program CliProgram, args []string, say CliSay) CliParsed {
	end := len(args)
	for i, a := range args {
		if a == "--" {
			end = i
			break
		}
	}
	askedHelp := false
	var rest []string
	for i, a := range args {
		if i < end && a == "--help" {
			askedHelp = true
			continue
		}
		rest = append(rest, a)
	}
	if len(rest) > 0 && rest[0] == "help" {
		askedHelp = true
		rest = rest[1:]
	}
	var path []string
	at := 0
	command := -1
	for command < 0 {
		if at >= len(rest) || strings.HasPrefix(rest[at], "--") {
			if at < len(rest) && !askedHelp && len(path) == 0 {
				option := strings.SplitN(rest[at], "=", 2)[0]
				return cliWrong(program, path, nil, say, "cliUnknownOption", [2]string{"option", option})
			}
			return CliParsed{Help: CliGroupHelp(program, path, say)}
		}
		line := append(append([]string{}, path...), rest[at])
		found, group := -1, false
		for i, c := range program.Commands {
			if len(c.Line) == len(line) && sameStart(c.Line, line) {
				found = i
			} else if len(c.Line) > len(line) && sameStart(c.Line, line) {
				group = true
			}
		}
		switch {
		case found >= 0:
			command = found
		case group:
			path = line
		default:
			return cliWrong(program, path, nil, say, "cliUnknownCommand", [2]string{"command", strings.Join(line, " ")})
		}
		at++
	}
	c := program.Commands[command]
	if askedHelp {
		return CliParsed{Help: CliCommandHelp(program, c, say)}
	}
	wrong := func(key string, slots ...[2]string) CliParsed {
		return cliWrong(program, nil, &c, say, key, slots...)
	}

	after := rest[at:]
	endsAt := -1
	for i, w := range after {
		if w == "--" {
			endsAt = i
			break
		}
	}
	optionOf := func(word string) (int, *string) {
		name, value, hasValue := strings.Cut(word, "=")
		if !strings.HasPrefix(name, "--") {
			return -1, nil
		}
		for i, o := range c.Options {
			if o.Name == name[2:] {
				if hasValue {
					return i, &value
				}
				return i, nil
			}
		}
		return -1, nil
	}
	for i, w := range after {
		if endsAt >= 0 && i >= endsAt {
			break
		}
		if strings.HasPrefix(w, "--") {
			if index, _ := optionOf(w); index < 0 {
				return wrong("cliUnknownOption", [2]string{"option", strings.SplitN(w, "=", 2)[0]})
			}
		}
	}
	type typed struct {
		index int
		value string
	}
	var given []typed
	var positional []string
	for i := 0; i < len(after); i++ {
		w := after[i]
		if i == endsAt {
			positional = append(positional, after[i+1:]...)
			break
		}
		if !strings.HasPrefix(w, "--") {
			positional = append(positional, w)
			continue
		}
		index, value := optionOf(w)
		option := c.Options[index]
		var v string
		switch {
		case option.Kind == "bool" && value == nil:
			v = "true"
		case value != nil:
			v = *value
		case i+1 < len(after):
			i++
			v = after[i]
		default:
			return wrong("cliNeedsValue", [2]string{"option", "--" + option.Name})
		}
		kept := given[:0]
		for _, g := range given {
			if g.index != index {
				kept = append(kept, g)
			}
		}
		given = append(kept, typed{index, v})
	}

	// Each value in the order it was typed: of its kind, then within its range.
	type read struct {
		name  string
		param CliParam
		value string
	}
	var reads []read
	for i, a := range c.Arguments {
		if i < len(positional) {
			reads = append(reads, read{a.Name, a, positional[i]})
		}
	}
	for _, g := range given {
		o := c.Options[g.index]
		reads = append(reads, read{"--" + o.Name, o, g.value})
	}
	values := map[string]string{}
	for _, r := range reads {
		v, ok := CliReadValue(r.param, r.value)
		if !ok {
			return wrong("cliWrongValue", [2]string{"name", r.name}, [2]string{"value", r.value})
		}
		values[r.name] = v
	}
	for _, r := range reads {
		v := values[r.name]
		if r.param.Min != nil && cliBelow(v, *r.param.Min) {
			return wrong("cliTooSmall", [2]string{"name", r.name}, [2]string{"min", *r.param.Min})
		}
		if r.param.Max != nil && cliBelow(*r.param.Max, v) {
			return wrong("cliTooLarge", [2]string{"name", r.name}, [2]string{"max", *r.param.Max})
		}
	}
	if len(positional) < len(c.Arguments) {
		return wrong("cliMissingArgument", [2]string{"argument", c.Arguments[len(positional)].Name})
	}
	if len(positional) > len(c.Arguments) {
		return wrong("cliTooManyArguments")
	}
	for i, o := range c.Options {
		if !o.Required {
			continue
		}
		found := false
		for _, g := range given {
			found = found || g.index == i
		}
		if !found {
			return wrong("cliRequired", [2]string{"option", "--" + o.Name})
		}
	}
	out := []*string{}
	for _, a := range c.Arguments {
		v := values[a.Name]
		out = append(out, &v)
	}
	for _, o := range c.Options {
		if v, ok := values["--"+o.Name]; ok {
			out = append(out, &v)
		} else if o.Default != nil {
			d := *o.Default
			out = append(out, &d)
		} else if o.Kind == "bool" {
			f := "false"
			out = append(out, &f)
		} else {
			out = append(out, nil)
		}
	}
	return CliParsed{Run: true, Command: command, Values: out}
}

func plainDigits(s string) bool {
	if s == "" {
		return false
	}
	for _, r := range s {
		if r < '0' || r > '9' {
			return false
		}
	}
	return true
}

// CliReadValue gives value as text of param's kind, or false when it is not one.
func CliReadValue(param CliParam, value string) (string, bool) {
	switch param.Kind {
	case "text":
		return value, true
	case "bool":
		return value, value == "true" || value == "false"
	case "integer":
		if !plainDigits(strings.TrimPrefix(value, "-")) {
			return "", false
		}
		n, err := strconv.ParseInt(value, 10, 64)
		return strconv.FormatInt(n, 10), err == nil
	case "decimal", "money":
		whole, part, _ := strings.Cut(strings.TrimPrefix(value, "-"), ".")
		if part == "" && strings.Contains(value, ".") || !plainDigits(whole) || part != "" && !plainDigits(part) {
			return "", false
		}
		d, err := decimal.NewFromString(value)
		if err != nil {
			return "", false
		}
		return d.String(), true
	case "date":
		d, err := time.Parse("2006-01-02", value)
		return d.Format("2006-01-02"), err == nil && len(value) == 10
	case "datetime":
		for _, layout := range []string{"2006-01-02T15:04:05", "2006-01-02T15:04"} {
			if m, err := time.Parse(layout, value); err == nil && len(value) == len(layout) {
				return m.Format("2006-01-02T15:04:05"), true
			}
		}
		return "", false
	case "duration":
		seconds, ok := cliDuration(value)
		return strconv.FormatInt(seconds, 10), ok
	case "enum":
		for _, v := range param.Values {
			if strings.EqualFold(v, value) {
				return v, true
			}
		}
	}
	return "", false
}

// Whole seconds from 45, 3:45 or 1:02:03, as formats.json reads them.
func cliDuration(text string) (int64, bool) {
	parts := strings.Split(strings.TrimSpace(text), ":")
	number := func(s string) (int64, bool) {
		if !plainDigits(s) {
			return 0, false
		}
		n, err := strconv.ParseInt(s, 10, 64)
		return n, err == nil
	}
	two := func(s string) (int64, bool) {
		n, ok := number(s)
		return n, ok && len(s) == 2 && n < 60
	}
	switch len(parts) {
	case 1:
		return number(parts[0])
	case 2:
		m, ok1 := number(parts[0])
		s, ok2 := two(parts[1])
		return m*60 + s, ok1 && ok2
	case 3:
		h, ok1 := number(parts[0])
		m, ok2 := two(parts[1])
		s, ok3 := two(parts[2])
		return h*3600 + m*60 + s, ok1 && ok2 && ok3
	}
	return 0, false
}

func cliBelow(a, b string) bool {
	x, err1 := decimal.NewFromString(a)
	y, err2 := decimal.NewFromString(b)
	return err1 == nil && err2 == nil && x.LessThan(y)
}

func cliWrong(program CliProgram, path []string, command *CliCommand, say CliSay, key string, slots ...[2]string) CliParsed {
	usage := cliGroupUsage(program, path, say)
	if command != nil {
		usage = cliCommandUsage(program, *command, say)
	}
	if slots == nil {
		slots = [][2]string{}
	}
	return CliParsed{Key: key, Slots: slots, Said: say(key, slots...) + "\n" + usage + "\n"}
}

func cliGroupUsage(program CliProgram, path []string, say CliSay) string {
	line := strings.Join(append([]string{program.Name}, path...), " ")
	return fmt.Sprintf("%s: %s <%s> [%s]", say("cliUsage"), line, say("cliCommandWord"), say("cliOptionsWord"))
}

func cliCommandUsage(program CliProgram, c CliCommand, say CliSay) string {
	line := append([]string{program.Name}, c.Line...)
	for _, a := range c.Arguments {
		line = append(line, "<"+a.Name+">")
	}
	return fmt.Sprintf("%s: %s [%s]", say("cliUsage"), strings.Join(line, " "), say("cliOptionsWord"))
}

// A heading and its lines, the left column padded to the longest.
func cliSection(heading string, rows [][2]string) []string {
	width := 0
	for _, r := range rows {
		width = max(width, utf8.RuneCountInString(r[0]))
	}
	out := []string{heading + ":"}
	for _, r := range rows {
		line := "  " + r[0]
		if r[1] != "" {
			line += strings.Repeat(" ", width-utf8.RuneCountInString(r[0])+2) + r[1]
		}
		out = append(out, strings.TrimRight(line, " "))
	}
	return out
}

// CliGroupHelp is the help of the program (path empty) or of a group.
func CliGroupHelp(program CliProgram, path []string, say CliSay) string {
	var lines []string
	if len(path) == 0 && program.Help != nil {
		lines = append(lines, *program.Help, "")
	}
	lines = append(lines, cliGroupUsage(program, path, say), "")
	var commands [][2]string
	for _, c := range program.Commands {
		if len(c.Line) > len(path) && sameStart(c.Line, path) {
			help := ""
			if c.Help != nil {
				help = *c.Help
			}
			commands = append(commands, [2]string{strings.Join(c.Line, " "), help})
		}
	}
	lines = append(lines, cliSection(say("cliCommands"), commands)...)
	lines = append(lines, "")
	lines = append(lines, cliSection(say("cliOptions"), [][2]string{{"--help", say("cliHelpOption")}})...)
	return strings.Join(lines, "\n") + "\n"
}

// CliCommandHelp is the help of one command: what it does, its usage and its options.
func CliCommandHelp(program CliProgram, c CliCommand, say CliSay) string {
	var lines []string
	if c.Help != nil {
		lines = append(lines, *c.Help, "")
	}
	lines = append(lines, cliCommandUsage(program, c, say), "")
	var rows [][2]string
	for _, o := range c.Options {
		left := "--" + o.Name
		if o.Kind != "bool" {
			left += " <" + o.Name + ">"
		}
		var marks []string
		if len(o.Values) > 0 {
			marks = append(marks, strings.Join(o.Values, ", "))
		}
		if o.Default != nil && o.Kind != "bool" {
			marks = append(marks, say("cliDefault", [2]string{"value", *o.Default}))
		}
		if o.Required {
			marks = append(marks, say("cliRequiredMark"))
		}
		right := ""
		if o.Help != nil {
			right = *o.Help
		}
		if len(marks) > 0 {
			if right != "" {
				right += " "
			}
			right += "(" + strings.Join(marks, "; ") + ")"
		}
		rows = append(rows, [2]string{left, right})
	}
	rows = append(rows, [2]string{"--help", say("cliHelpOption")})
	lines = append(lines, cliSection(say("cliOptions"), rows)...)
	return strings.Join(lines, "\n") + "\n"
}

// CliTable is what `print list columns a, b` writes: a line of headings,
// then a line per row, the columns two spaces apart, a column of numbers
// aligned right.
func CliTable(headings []string, right []bool, rows [][]string) string {
	lines := append([][]string{headings}, rows...)
	widths := make([]int, len(headings))
	for _, row := range lines {
		for i := range widths {
			if i < len(row) {
				widths[i] = max(widths[i], utf8.RuneCountInString(row[i]))
			}
		}
	}
	var out strings.Builder
	for _, row := range lines {
		cells := make([]string, len(widths))
		for i, width := range widths {
			cell := ""
			if i < len(row) {
				cell = row[i]
			}
			pad := strings.Repeat(" ", width-utf8.RuneCountInString(cell))
			if i < len(right) && right[i] {
				cells[i] = pad + cell
			} else {
				cells[i] = cell + pad
			}
		}
		out.WriteString(strings.TrimRight(strings.Join(cells, "  "), " ") + "\n")
	}
	return out.String()
}

// CliAsked is what a command asks: yes or no by the words of its buttons
// (Kind "confirm"), a value of a kind with its default ("value"), or one of
// Choices by number from 1 ("choice").
type CliAsked struct {
	Kind    string
	Yes, No string
	Param   CliParam
	Choices []string
}

// CliTerminal is where a command asks: a terminal, or a pipe or script
// with nobody at it.
type CliTerminal interface {
	IsTerminal() bool
	// The next line typed, without its end; false at the end of input.
	ReadLine() (string, bool)
	// Writes to standard error, where questions go.
	Write(text string)
}

// CliConsole is the terminal the program runs in.
type CliConsole struct{ in *bufio.Reader }

func NewCliConsole() *CliConsole { return &CliConsole{in: bufio.NewReader(os.Stdin)} }

// IsTerminal is true for a terminal, or Clarity's Output panel, which says
// so with CLARITY_TERMINAL=1.
func (c *CliConsole) IsTerminal() bool {
	if os.Getenv("CLARITY_TERMINAL") == "1" {
		return true
	}
	info, err := os.Stdin.Stat()
	return err == nil && info.Mode()&os.ModeCharDevice != 0
}

func (c *CliConsole) ReadLine() (string, bool) {
	line, err := c.in.ReadString('\n')
	if err != nil && line == "" {
		return "", false
	}
	return strings.TrimRight(line, "\r\n"), true
}

func (c *CliConsole) Write(text string) { fmt.Fprint(os.Stderr, text) }

// CliAsk asks question and gives the answer as text ("true" or "false" for
// yes or no, the number of a choice), or the exit code the program ends with.
func CliAsk(question string, asked CliAsked, terminal CliTerminal, say CliSay) (string, int) {
	if !terminal.IsTerminal() {
		if asked.Kind == "value" && asked.Param.Default != nil {
			return *asked.Param.Default, 0
		}
		terminal.Write(say("cliNoTerminal", [2]string{"question", question}) + "\n")
		return "", 2
	}
	switch asked.Kind {
	case "confirm":
		for {
			terminal.Write(fmt.Sprintf("%s [%s/%s] ", question, asked.Yes, asked.No))
			line, ok := terminal.ReadLine()
			line = strings.TrimSpace(line)
			switch {
			case !ok || line == "":
				return "false", 0
			case strings.EqualFold(line, asked.Yes):
				return "true", 0
			case strings.EqualFold(line, asked.No):
				return "false", 0
			}
		}
	case "choice":
		var listed strings.Builder
		for i, c := range asked.Choices {
			fmt.Fprintf(&listed, "%d. %s\n", i+1, c)
		}
		terminal.Write(listed.String())
		for {
			terminal.Write(question + " ")
			line, ok := terminal.ReadLine()
			if !ok {
				terminal.Write("\n")
				return "", 1
			}
			if n, err := strconv.Atoi(strings.TrimSpace(line)); err == nil && n >= 1 && n <= len(asked.Choices) {
				return strconv.Itoa(n), 0
			}
			terminal.Write(say("cliChooseNumber", [2]string{"count", strconv.Itoa(len(asked.Choices))}) + "\n")
		}
	}
	param := asked.Param
	shown := ""
	if param.Kind == "enum" {
		shown = strings.Join(param.Values, "/")
	} else if param.Default != nil {
		shown = *param.Default
	}
	for {
		if shown != "" {
			terminal.Write(fmt.Sprintf("%s [%s] ", question, shown))
		} else {
			terminal.Write(question + " ")
		}
		line, ok := terminal.ReadLine()
		if !ok {
			terminal.Write("\n")
			return "", 1
		}
		line = strings.TrimSpace(line)
		if line == "" && param.Default != nil {
			return *param.Default, 0
		}
		if value, ok := CliReadValue(param, line); ok {
			return value, 0
		}
		terminal.Write(say("cliWrongAnswer", [2]string{"value", line}) + "\n")
	}
}

// CliText is a text read from the command line; "" for none.
func CliText(text *string) string {
	if text == nil {
		return ""
	}
	return *text
}

// CliInteger is a whole number (or a duration's seconds) read from the command line; 0 for none.
func CliInteger(text *string) int64 {
	n, _ := strconv.ParseInt(CliText(text), 10, 64)
	return n
}

// CliBool is a flag read from the command line.
func CliBool(text *string) bool { return CliText(text) == "true" }

// CliDecimal is a decimal or money read from the command line; 0 for none.
func CliDecimal(text *string) decimal.Decimal {
	d, err := decimal.NewFromString(CliText(text))
	if err != nil {
		return decimal.Zero
	}
	return d
}

// CliDate is a date read from the command line; nil for none.
func CliDate(text *string) *Date {
	d, err := ParseDate(CliText(text))
	if err != nil {
		return nil
	}
	return &d
}

// CliDateTime is a moment read from the command line; nil for none.
func CliDateTime(text *string) *DateTime {
	m, err := ParseDateTime(CliText(text))
	if err != nil {
		return nil
	}
	return &m
}

// CliTextOf is a value of code as the command line writes it: 42, 12.5,
// 2026-10-23, an enum value by its name; "" for none.
func CliTextOf(value any) string {
	data, err := json.Marshal(value)
	if err != nil {
		return ""
	}
	var back any
	if json.Unmarshal(data, &back) != nil || back == nil {
		return ""
	}
	if text, ok := back.(string); ok {
		return text
	}
	return string(data)
}

// CliPickLanguage is the language to show for the system's locale
// (nl_NL.UTF-8), among the project's translations; nil for the language the
// project is written in (base), or one it has no translation for.
func CliPickLanguage(locale string, available []string, base string) *string {
	tag := locale
	if at := strings.IndexAny(tag, ".@"); at >= 0 {
		tag = tag[:at]
	}
	tag = strings.ReplaceAll(tag, "_", "-")
	primary := func(t string) string { return strings.ToLower(strings.SplitN(t, "-", 2)[0]) }
	if tag == "" || tag == "C" || tag == "POSIX" || primary(tag) == primary(base) {
		return nil
	}
	for _, a := range available {
		if a == tag {
			return &a
		}
	}
	for _, a := range available {
		if primary(a) == primary(tag) {
			return &a
		}
	}
	return nil
}

// CliKeptSettings are the settings kept, each one that still fits, over
// fresh, which has the defaults.
func CliKeptSettings[T any](kept map[string]any, fresh T) T {
	data, _ := json.Marshal(fresh)
	values := map[string]any{}
	_ = json.Unmarshal(data, &values)
	for name, value := range kept {
		if _, known := values[name]; !known {
			continue
		}
		tried := map[string]any{}
		for k, v := range values {
			tried[k] = v
		}
		tried[name] = value
		var check T
		if data, err := json.Marshal(tried); err == nil && json.Unmarshal(data, &check) == nil {
			values = tried
		}
	}
	var out T
	data, _ = json.Marshal(values)
	if json.Unmarshal(data, &out) != nil {
		return fresh
	}
	return out
}

// cliTranslated gives a key's value in the language shown.
func cliTranslated(key string) string {
	value, _ := KeyValue(key)
	return value
}

// CliSession is what a running command prints, says and asks through:
// standard output, standard error and the terminal, in the language shown.
type CliSession struct {
	Terminal CliTerminal
}

// NewCliSession asks on the terminal the program runs in.
func NewCliSession() *CliSession { return &CliSession{Terminal: NewCliConsole()} }

func (s *CliSession) say() CliSay { return CliWordsIn(cliTranslated) }

// Print is `print value`: one line on standard output.
func (s *CliSession) Print(text Text) { fmt.Println(text.Show()) }

// PrintTable is `print list columns a, b`: the records as a table on standard output.
func (s *CliSession) PrintTable(headings []Text, right []bool, rows [][]string) {
	shown := make([]string, len(headings))
	for i, h := range headings {
		shown[i] = h.Show()
	}
	fmt.Print(CliTable(shown, right, rows))
}

// Message is `message`: one line on standard error.
func (s *CliSession) Message(text Text) { fmt.Fprintln(os.Stderr, text.Show()) }

// CliYesNo is Clarity's word for yes or no, as a table shows a bool.
func CliYesNo(value bool) string {
	key, english := "no", "No"
	if value {
		key, english = "yes", "Yes"
	}
	if v := cliTranslated(key); v != "" {
		return v
	}
	return english
}

func (s *CliSession) answer(question Text, asked CliAsked) string {
	answer, code := CliAsk(question.Show(), asked, s.Terminal, s.say())
	if code != 0 {
		// Nobody to answer, or no answer: the program ends here.
		os.Exit(code)
	}
	return answer
}

// Confirm is `ask ... yes @a no @b`: whether the person said yes. Without
// the words the question takes Clarity's OK and Cancel.
func (s *CliSession) Confirm(question Text, yes, no *Text) bool {
	word := func(text *Text, key, english string) string {
		if text != nil {
			return text.Show()
		}
		if v := cliTranslated(key); v != "" {
			return v
		}
		return english
	}
	return s.answer(question, CliAsked{Kind: "confirm", Yes: word(yes, "ok", "OK"), No: word(no, "cancel", "Cancel")}) == "true"
}

// Ask is `ask ... for type [default value]`: the value typed, as text of its
// kind; the generated code takes it as its type.
func (s *CliSession) Ask(question Text, kind string, values []string, fallback *string) *string {
	answer := s.answer(question, CliAsked{Kind: "value", Param: CliParam{Kind: kind, Values: values, Default: fallback}})
	return &answer
}

// Choose is `ask ... for Entity from list`: the position, from 0, of the record chosen.
func (s *CliSession) Choose(question Text, choices []string) int {
	n, _ := strconv.Atoi(s.answer(question, CliAsked{Kind: "choice", Choices: choices}))
	return max(n-1, 0)
}

// RunCliProgram runs a command-line program: reads the command line, writes
// help or what is wrong with it, or runs the command it names with its
// values, and gives the exit code its run ends with (DESIGN 5.20).
func RunCliProgram(program CliProgram, run func(cli *CliSession, command int, values []*string) (int, error)) int {
	parsed := ParseCommandLine(program, os.Args[1:], CliWordsIn(cliTranslated))
	switch {
	case parsed.Run:
		code, err := run(NewCliSession(), parsed.Command, parsed.Values)
		if err != nil {
			fmt.Fprintln(os.Stderr, err.Error())
			return 1
		}
		return min(max(code, 0), 255)
	case parsed.Key != "":
		fmt.Fprint(os.Stderr, parsed.Said)
		return 2
	default:
		fmt.Print(parsed.Help)
		return 0
	}
}

// CliKeepSettings keeps the settings when a command changed them, as a
// window does.
func CliKeepSettings(program string, before, after any) {
	was, _ := json.Marshal(before)
	now, _ := json.Marshal(after)
	if string(was) == string(now) {
		return
	}
	values := map[string]any{}
	_ = json.Unmarshal(now, &values)
	delete(values, "id")
	if err := KeepSettings(program, values); err != nil {
		fmt.Fprintln(os.Stderr, err)
	}
}

// CliWord is a button's word, which Confirm takes or leaves out.
func CliWord(text Text) *Text { return &text }

// CliDefault is the default of a question, as text of its kind.
func CliDefault(text string) *string { return &text }

// CliHelp is a help text of the program, shown in the language shown.
func CliHelp(text Text) *string {
	shown := text.Show()
	return &shown
}
