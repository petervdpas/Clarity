// The binding between a window on the screen and its handlers in the
// backend. The screen sends a window's state with each event; the handler
// runs on it and the reply carries the new state, the messages to show and
// the windows to open or close. The JSON is the Rust runtime's, so the same
// renderer serves every stack.

package clarity

import (
	"encoding/json"
	"fmt"
	"math"
	"sync"
	"sync/atomic"
)

// PartState is what a handler changed of a part; unset fields keep the design.
type PartState struct {
	Visible  *bool `json:"visible,omitempty"`
	Enabled  *bool `json:"enabled,omitempty"`
	Caption  *Text `json:"caption,omitempty"`
	Error    *Text `json:"error,omitempty"`
	Readonly *bool `json:"readonly,omitempty"`
	// Folded is whether a foldable panel or form is folded, as the person or a
	// handler left it; nil keeps the design (conformance/folding.json).
	Folded *bool `json:"folded,omitempty"`
	// Collapsed is whether a collapsible sidebar is folded to its button, as the
	// person or a handler left it; nil keeps the design (conformance/sidebars.json).
	Collapsed *bool `json:"collapsed,omitempty"`
	// Grid is a canvas's grid in its units (0 for none), and Snap whether code
	// snaps to it, as a handler set them; nil keeps the design.
	Grid *int64 `json:"grid,omitempty"`
	Snap *bool  `json:"snap,omitempty"`
	// Background is what a canvas's clear() fills it with, as a handler set
	// it; empty is the theme's panel.
	Background *string `json:"background,omitempty"`
	// DotSize and DotColor are a canvas's grid dots, as a handler set them:
	// pixels across, and color (empty is the theme's border).
	DotSize  *int64  `json:"dotSize,omitempty"`
	DotColor *string `json:"dotColor,omitempty"`
	// Tool, Brush and Ink are the tool in hand of a painting or vector
	// canvas, how wide it paints and its color, as a handler set them
	// (DESIGN 5.31); nil keeps the design.
	Tool  *string `json:"tool,omitempty"`
	Brush *int64  `json:"brush,omitempty"`
	Ink   *string `json:"ink,omitempty"`
	// Maximized is whether a maximizable sidebar fills the window; nil is not.
	Maximized *bool `json:"maximized,omitempty"`
	// Tab is the open tab of a ribbon, counted from 1; nil is the first
	// (conformance/ribbons.json).
	Tab *int64 `json:"tab,omitempty"`
	// Step is the step a wizard (a steps part) is on, counted from 1; nil is
	// the first (conformance/steps.json).
	Step *int64 `json:"step,omitempty"`
	// Open is the open section of an accordion, counted from 1, 0 for none;
	// nil is the first (conformance/accordions.json).
	Open *int64 `json:"open,omitempty"`
	// Text is what an unbound text input holds; the screen sends it with each event.
	Text *string `json:"text,omitempty"`
	// Checked is whether an unbound checkbox or switch is on.
	Checked *bool `json:"checked,omitempty"`
	// Drawing is a canvas's drawing: its shapes, in order (conformance/drawing.json).
	Drawing *[]map[string]any `json:"drawing,omitempty"`
	// Pointer is where a canvas was last clicked, in its units: x and y, and
	// whatever else the screen tells about the pointer, kept as it came.
	Pointer map[string]any `json:"pointer,omitempty"`
	// Values are a custom part's properties by name, as code set them
	// (conformance/custom-parts.json).
	Values map[string]any `json:"values,omitempty"`
}

// Hear is a hear a handler gave a custom part: arena.restart(2).
type Hear struct {
	Part   string `json:"part"`
	Hear   string `json:"hear"`
	Values []any  `json:"values"`
}

// WindowState is one open window: its data by name, its parts' properties
// and the selected row of each table or list.
type WindowState struct {
	Window string `json:"window"`
	// Instance is the number of this open window, its own while it is open:
	// what belongs to it, such as an engine's state per window, goes by it.
	Instance uint64               `json:"instance"`
	Data     map[string]any       `json:"data"`
	Parts    map[string]PartState `json:"parts"`
	Selected map[string]int       `json:"selected"`
	// Views are what the screen shows of the list of each table, tree and
	// chart, by the part's name (DESIGN 1.7): how it is shown, as the screen
	// sends it, and what shows, as the runtime fills it after every reply.
	Views map[string]map[string]any `json:"views,omitempty"`
}

// nextInstance is the number the next window that opens gets.
var nextInstance atomic.Uint64

// NewWindowState is a window that opens, with a number of its own.
func NewWindowState(window string, data map[string]any) WindowState {
	if data == nil {
		data = map[string]any{}
	}
	return WindowState{Window: window, Instance: nextInstance.Add(1), Data: data, Parts: map[string]PartState{}, Selected: map[string]int{}}
}

var (
	closedMu sync.Mutex
	closed   []func(instance uint64)
)

// OnWindowClosed has listen hear of every window that closes, by its
// number: an engine with state per window drops what it had of it.
func OnWindowClosed(listen func(instance uint64)) {
	closedMu.Lock()
	defer closedMu.Unlock()
	closed = append(closed, listen)
}

// WindowClosed tells what listens that window instance has closed, once its
// own `on close` has run.
func WindowClosed(instance uint64) {
	closedMu.Lock()
	listeners := append([]func(uint64){}, closed...)
	closedMu.Unlock()
	for _, listen := range listeners {
		listen(instance)
	}
}

// Decode reads the window's data into the struct of its window.
func (s WindowState) Decode(into any) error {
	text, err := json.Marshal(s.Data)
	if err != nil {
		return &InvalidError{err.Error()}
	}
	if err := json.Unmarshal(text, into); err != nil {
		return &InvalidError{fmt.Sprintf("the data of %s does not fit it: %v", s.Window, err)}
	}
	return nil
}

// Level is how a message shows: info goes away by itself.
type Level string

const (
	Info    Level = "info"
	Warning Level = "warning"
	Error   Level = "error"
)

type Message struct {
	Level Level `json:"level"`
	Text  Text  `json:"text"`
}

// OpenRequest is open Window with name = value, ...
type OpenRequest struct {
	Window string                     `json:"window"`
	With   map[string]json.RawMessage `json:"with"`
}

// Reply is what the screen does after a handler ran.
type Reply struct {
	State    WindowState `json:"state"`
	Messages []Message   `json:"messages"`
	// Shown holds the texts the handler put in text values, by the value's
	// name: the screen fills them in the person's language and format.
	Shown map[string]Text `json:"shown,omitempty"`
	Open  []OpenRequest   `json:"open"`
	Close bool            `json:"close"`
	// Asks are the questions to ask when the handler has finished, in order.
	Asks []Ask `json:"asks,omitempty"`
	// Result is what a dialog closes with, for the handler that asked for it.
	Result json.RawMessage `json:"result,omitempty"`
	// Hears are the hears given to custom parts, in order.
	Hears []Hear `json:"hears,omitempty"`
	// Copy is the text the handler copied (copy text), for the screen to put on the clipboard.
	Copy *Text `json:"copy,omitempty"`
	// Step is the id of the database step the handler made (DESIGN 5.29),
	// which the screen's edit keeps to undo it by; 0 when it changed nothing.
	Step int64 `json:"step,omitempty"`
	// Refused says the runtime took nothing of what the person typed (an
	// item for a list that is no value of its kind): the screen keeps the
	// text and marks it.
	Refused bool `json:"refused,omitempty"`
}

// RefOption is a record a reference field can point at, as a dropdown shows it.
type RefOption struct {
	ID   int64  `json:"id"`
	Text string `json:"text"`
	// Fields are the fields a search dropdown shows of the record (U1c), by name.
	Fields map[string]any `json:"fields,omitempty"`
}

// Ui is what a handler can do to the screen.
type Ui struct {
	instance uint64
	parts    map[string]PartState
	selected map[string]int
	views    map[string]map[string]any
	messages []Message
	shown    map[string]Text
	open     []OpenRequest
	asks     []Ask
	result   json.RawMessage
	hears    []Hear
	copied   *Text
	close    bool
	err      error
}

func NewUi(state WindowState) *Ui {
	ui := &Ui{instance: state.Instance, parts: map[string]PartState{}, selected: map[string]int{}, views: state.Views, messages: []Message{}, open: []OpenRequest{}}
	for k, v := range state.Parts {
		ui.parts[k] = v
	}
	for k, v := range state.Selected {
		ui.selected[k] = v
	}
	return ui
}

// Instance is the number of the open window the handler runs in.
func (u *Ui) Instance() uint64 { return u.instance }

func (u *Ui) part(name string, change func(*PartState)) {
	p := u.parts[name]
	change(&p)
	u.parts[name] = p
}

func (u *Ui) Visible(part string) bool {
	if v := u.parts[part].Visible; v != nil {
		return *v
	}
	return true
}

func (u *Ui) SetVisible(part string, value bool) {
	u.part(part, func(p *PartState) { p.Visible = &value })
}

func (u *Ui) Enabled(part string) bool {
	if v := u.parts[part].Enabled; v != nil {
		return *v
	}
	return true
}

func (u *Ui) SetEnabled(part string, value bool) {
	u.part(part, func(p *PartState) { p.Enabled = &value })
}

// Caption is the caption a handler set, or design.
func (u *Ui) Caption(part, design string) string {
	if c := u.parts[part].Caption; c != nil {
		return c.Show()
	}
	return design
}

// BeginDrawing starts an empty drawing on a canvas, before its draw handler runs.
func (u *Ui) BeginDrawing(part string) {
	u.part(part, func(p *PartState) { p.Drawing = &[]map[string]any{} })
}

// Draw adds a shape to a canvas's drawing: {"op": "line", ...}.
func (u *Ui) Draw(part string, shape map[string]any) {
	u.part(part, func(p *PartState) {
		if p.Drawing == nil {
			p.Drawing = &[]map[string]any{}
		}
		*p.Drawing = append(*p.Drawing, shape)
	})
}

func (u *Ui) SetCaption(part string, value Text) {
	u.part(part, func(p *PartState) { p.Caption = &value })
}

// Error is the message a handler set on an input, or empty text.
func (u *Ui) Error(part string) string {
	if e := u.parts[part].Error; e != nil {
		return e.Show()
	}
	return ""
}

func (u *Ui) SetError(part string, value Text) { u.part(part, func(p *PartState) { p.Error = &value }) }

// Readonly is whether an input is read-only: as a handler set it, or design.
func (u *Ui) Readonly(part string, design bool) bool {
	if r := u.parts[part].Readonly; r != nil {
		return *r
	}
	return design
}

func (u *Ui) SetReadonly(part string, value bool) {
	u.part(part, func(p *PartState) { p.Readonly = &value })
}

// Folded is whether a foldable part is folded: as the person or a handler left it, or design.
func (u *Ui) Folded(part string, design bool) bool {
	if f := u.parts[part].Folded; f != nil {
		return *f
	}
	return design
}

func (u *Ui) SetFolded(part string, value bool) {
	u.part(part, func(p *PartState) { p.Folded = &value })
}

// Collapsed is whether a collapsible sidebar is folded to its button: as left, or design.
func (u *Ui) Collapsed(part string, design bool) bool {
	if c := u.parts[part].Collapsed; c != nil {
		return *c
	}
	return design
}

func (u *Ui) SetCollapsed(part string, value bool) {
	u.part(part, func(p *PartState) { p.Collapsed = &value })
}

// Grid is a canvas's grid: canvas.grid, as its line says until a handler sets it.
func (u *Ui) Grid(part string, design int64) int64 {
	if g := u.parts[part].Grid; g != nil {
		return *g
	}
	return design
}

func (u *Ui) SetGrid(part string, value int64) {
	u.part(part, func(p *PartState) { p.Grid = &value })
}

// Snap is whether code snaps to a canvas's grid: canvas.snap.
func (u *Ui) Snap(part string, design bool) bool {
	if s := u.parts[part].Snap; s != nil {
		return *s
	}
	return design
}

func (u *Ui) SetSnap(part string, value bool) {
	u.part(part, func(p *PartState) { p.Snap = &value })
}

// Background is what a canvas's clear() fills it with: canvas.background.
func (u *Ui) Background(part string, design string) string {
	if b := u.parts[part].Background; b != nil {
		return *b
	}
	return design
}

func (u *Ui) SetBackground(part string, value string) {
	u.part(part, func(p *PartState) { p.Background = &value })
}

// DotSize is how many pixels across a canvas's grid dots are: canvas.dotSize.
func (u *Ui) DotSize(part string, design int64) int64 {
	if d := u.parts[part].DotSize; d != nil {
		return *d
	}
	return design
}

func (u *Ui) SetDotSize(part string, value int64) {
	u.part(part, func(p *PartState) { p.DotSize = &value })
}

// DotColor is the color of a canvas's grid dots: canvas.dotColor.
func (u *Ui) DotColor(part string, design string) string {
	if c := u.parts[part].DotColor; c != nil {
		return *c
	}
	return design
}

func (u *Ui) SetDotColor(part string, value string) {
	u.part(part, func(p *PartState) { p.DotColor = &value })
}

// Tool is the tool in hand of a painting or vector canvas: canvas.tool.
func (u *Ui) Tool(part string, design string) string {
	if t := u.parts[part].Tool; t != nil {
		return *t
	}
	return design
}

func (u *Ui) SetTool(part string, value string) {
	u.part(part, func(p *PartState) { p.Tool = &value })
}

// Brush is how wide a painting or vector canvas paints: canvas.brush.
func (u *Ui) Brush(part string, design int64) int64 {
	if b := u.parts[part].Brush; b != nil {
		return *b
	}
	return design
}

func (u *Ui) SetBrush(part string, value int64) {
	u.part(part, func(p *PartState) { p.Brush = &value })
}

// Ink is the color a painting or vector canvas paints with: canvas.ink.
func (u *Ui) Ink(part string, design string) string {
	if i := u.parts[part].Ink; i != nil {
		return *i
	}
	return design
}

func (u *Ui) SetInk(part string, value string) {
	u.part(part, func(p *PartState) { p.Ink = &value })
}

// ShapeChosen is the place of the shape a person touched last on a vector
// canvas, counted from 1; 0 when none (DESIGN 5.31).
func (u *Ui) ShapeChosen(part string) int64 {
	if i, ok := u.selected[part]; ok && i >= 0 {
		return int64(i) + 1
	}
	return 0
}

// Maximized is whether a maximizable sidebar fills the window.
func (u *Ui) Maximized(part string) bool {
	if m := u.parts[part].Maximized; m != nil {
		return *m
	}
	return false
}

func (u *Ui) SetMaximized(part string, value bool) {
	u.part(part, func(p *PartState) { p.Maximized = &value })
}

// Tab is the open tab of a ribbon, counted from 1.
func (u *Ui) Tab(part string) int64 {
	if t := u.parts[part].Tab; t != nil {
		return *t
	}
	return 1
}

func (u *Ui) SetTab(part string, value int64) {
	u.part(part, func(p *PartState) { p.Tab = &value })
}

// Step is the step a wizard is on, counted from 1.
func (u *Ui) Step(part string) int64 {
	if s := u.parts[part].Step; s != nil {
		return *s
	}
	return 1
}

func (u *Ui) SetStep(part string, value int64) {
	u.part(part, func(p *PartState) { p.Step = &value })
}

// OpenSection is the open section of an accordion, counted from 1; 0 when none is open.
func (u *Ui) OpenSection(part string) int64 {
	if o := u.parts[part].Open; o != nil {
		return *o
	}
	return 1
}

func (u *Ui) SetOpenSection(part string, value int64) {
	u.part(part, func(p *PartState) { p.Open = &value })
}

// X is where a canvas was last clicked, across: canvas.x; 0 before any click.
func (u *Ui) X(part string) int64 { return u.pointer(part, "x") }

// Y is where a canvas was last clicked, down: canvas.y.
func (u *Ui) Y(part string) int64 { return u.pointer(part, "y") }

// FromX is where a drag on a canvas started, across: canvas.fromX; 0 without one.
func (u *Ui) FromX(part string) int64 { return u.pointer(part, "fromX") }

// FromY is where a drag on a canvas started, down: canvas.fromY.
func (u *Ui) FromY(part string) int64 { return u.pointer(part, "fromY") }

// Bent is the name of the link a person bent on a canvas: canvas.bent; empty without one.
func (u *Ui) Bent(part string) string {
	name, _ := u.parts[part].Pointer["bent"].(string)
	return name
}

// Via is the bend points of that link now, x, y, x, y, ...: canvas.via.
func (u *Ui) Via(part string) []int64 {
	points, _ := u.parts[part].Pointer["via"].([]any)
	out := make([]int64, 0, len(points))
	for _, p := range points {
		if at, ok := p.(float64); ok {
			out = append(out, int64(math.Round(at)))
		}
	}
	return out
}

func (u *Ui) pointer(part, key string) int64 {
	if at, ok := u.parts[part].Pointer[key].(float64); ok {
		return int64(math.Floor(at))
	}
	return 0
}

// Text is the text of an unbound text input.
func (u *Ui) Text(part string) string {
	if t := u.parts[part].Text; t != nil {
		return *t
	}
	return ""
}

func (u *Ui) SetText(part string, value string) { u.part(part, func(p *PartState) { p.Text = &value }) }

// Checked is whether an unbound checkbox or switch is on.
func (u *Ui) Checked(part string) bool {
	if c := u.parts[part].Checked; c != nil {
		return *c
	}
	return false
}

func (u *Ui) SetChecked(part string, value bool) {
	u.part(part, func(p *PartState) { p.Checked = &value })
}

// Selected is the selected record of a table or list showing rows, as a copy to work on.
func Selected[T any](u *Ui, part string, rows []T) (*T, error) {
	i, ok := u.selected[part]
	if !ok || i < 0 || i >= len(rows) {
		return nil, &InvalidError{fmt.Sprintf("no row of %s is selected", part)}
	}
	row := rows[i]
	return &row, nil
}

// Deselect forgets the selected row of a table or list, whose rows were
// read again or replaced: they are other records.
func (u *Ui) Deselect(part string) { delete(u.selected, part) }

func (u *Ui) Message(level Level, text Text) { u.messages = append(u.messages, Message{level, text}) }

// Copy is copy text: the screen puts it, as it shows, on the clipboard when
// the handler is done; the last copy counts.
func (u *Ui) Copy(text Text) { u.copied = &text }

// SetShown says the text value data shows text, translated and formatted by
// the screen.
func (u *Ui) SetShown(data string, text Text) {
	if u.shown == nil {
		u.shown = map[string]Text{}
	}
	u.shown[data] = text
}

// Open asks the screen to open window with its data filled by name, as the
// values are now.
func (u *Ui) Open(window string, with map[string]any) {
	values := map[string]json.RawMessage{}
	for name, value := range with {
		text, err := json.Marshal(value)
		if err != nil && u.err == nil {
			u.err = &InvalidError{err.Error()}
		}
		values[name] = text
	}
	u.open = append(u.open, OpenRequest{window, values})
}

// Close asks the screen to close the window.
func (u *Ui) Close() { u.close = true }

// Reply is the reply for window, with its data after the handler.
func (u *Ui) Reply(window string, data any) (Reply, error) {
	if u.err != nil {
		return Reply{}, u.err
	}
	values := map[string]any{}
	if data != nil {
		text, err := json.Marshal(data)
		if err != nil {
			return Reply{}, &InvalidError{err.Error()}
		}
		if err := json.Unmarshal(text, &values); err != nil {
			return Reply{}, &InvalidError{err.Error()}
		}
	}
	return Reply{
		State:    WindowState{Window: window, Instance: u.instance, Data: values, Parts: u.parts, Selected: u.selected, Views: u.views},
		Messages: u.messages,
		Shown:    u.shown,
		Open:     u.open,
		Close:    u.close,
		Asks:     u.asks,
		Result:   u.result,
		Hears:    u.hears,
		Copy:     u.copied,
	}, nil
}

// PartValue is a property of a custom part as code set it; false when it was not set.
func (u *Ui) PartValue(part, name string) (any, bool) {
	value, ok := u.parts[part].Values[name]
	return value, ok
}

func (u *Ui) SetPartValue(part, name string, value any) {
	u.part(part, func(p *PartState) {
		values := map[string]any{}
		for k, v := range p.Values {
			values[k] = v
		}
		values[name] = value
		p.Values = values
	})
}

// PartValueAs is a property of a custom part as type T: as code set it,
// else fallback, the value written in the window or declared, as JSON.
func PartValueAs[T any](u *Ui, part, name, fallback string) (T, error) {
	var out T
	text := []byte(fallback)
	if value, ok := u.PartValue(part, name); ok {
		given, err := json.Marshal(value)
		if err != nil {
			return out, &InvalidError{err.Error()}
		}
		text = given
	}
	if err := json.Unmarshal(text, &out); err != nil {
		return out, &InvalidError{fmt.Sprintf("%s.%s: %v", part, name, err)}
	}
	return out, nil
}

// Hear gives a custom part a hear, sent with the reply.
func (u *Ui) Hear(part, hear string, values ...any) {
	if values == nil {
		values = []any{}
	}
	u.hears = append(u.hears, Hear{Part: part, Hear: hear, Values: values})
}

// Opened is the state of window when it opens: its loaded data with with
// filled in by name. Data that does not fit the window (a wrong with) is
// refused, so a handler never sees it.
func Opened(window string, data any, with map[string]any) (WindowState, error) {
	state := NewWindowState(window, nil)
	text, err := json.Marshal(data)
	if err != nil {
		return state, &InvalidError{err.Error()}
	}
	if err := json.Unmarshal(text, &state.Data); err != nil {
		return state, &InvalidError{err.Error()}
	}
	for name, value := range with {
		state.Data[name] = value
	}
	if err := state.Decode(data); err != nil {
		return state, err
	}
	return state, nil
}
