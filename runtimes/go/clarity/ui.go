// The binding between a window on the screen and its handlers in the
// backend. The screen sends a window's state with each event; the handler
// runs on it and the reply carries the new state, the messages to show and
// the windows to open or close. The JSON is the Rust runtime's, so the same
// renderer serves every stack.

package clarity

import (
	"encoding/json"
	"fmt"
)

// PartState is what a handler changed of a part; unset fields keep the design.
type PartState struct {
	Visible  *bool `json:"visible,omitempty"`
	Enabled  *bool `json:"enabled,omitempty"`
	Caption  *Text `json:"caption,omitempty"`
	Error    *Text `json:"error,omitempty"`
	Readonly *bool `json:"readonly,omitempty"`
	// Text is what an unbound text input holds; the screen sends it with each event.
	Text *string `json:"text,omitempty"`
	// Checked is whether an unbound checkbox or switch is on.
	Checked *bool `json:"checked,omitempty"`
	// Drawing is a canvas's drawing: its shapes, in order (conformance/drawing.json).
	Drawing *[]map[string]any `json:"drawing,omitempty"`
}

// WindowState is one open window: its data by name, its parts' properties
// and the selected row of each table or list.
type WindowState struct {
	Window   string               `json:"window"`
	Data     map[string]any       `json:"data"`
	Parts    map[string]PartState `json:"parts"`
	Selected map[string]int       `json:"selected"`
}

func NewWindowState(window string, data map[string]any) WindowState {
	if data == nil {
		data = map[string]any{}
	}
	return WindowState{Window: window, Data: data, Parts: map[string]PartState{}, Selected: map[string]int{}}
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
	State    WindowState   `json:"state"`
	Messages []Message     `json:"messages"`
	Open     []OpenRequest `json:"open"`
	Close    bool          `json:"close"`
}

// RefOption is a record a reference field can point at, as a dropdown shows it.
type RefOption struct {
	ID   int64  `json:"id"`
	Text string `json:"text"`
}

// Ui is what a handler can do to the screen.
type Ui struct {
	parts    map[string]PartState
	selected map[string]int
	messages []Message
	open     []OpenRequest
	close    bool
	err      error
}

func NewUi(state WindowState) *Ui {
	ui := &Ui{parts: map[string]PartState{}, selected: map[string]int{}, messages: []Message{}, open: []OpenRequest{}}
	for k, v := range state.Parts {
		ui.parts[k] = v
	}
	for k, v := range state.Selected {
		ui.selected[k] = v
	}
	return ui
}

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
		State:    WindowState{Window: window, Data: values, Parts: u.parts, Selected: u.selected},
		Messages: u.messages,
		Open:     u.open,
		Close:    u.close,
	}, nil
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
