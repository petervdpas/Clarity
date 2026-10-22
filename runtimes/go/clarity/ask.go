// A question a handler asks: ask ... then handler(values) (DESIGN 5.14).
// The screen asks it in Clarity's own dialog when the handler has finished;
// the answer runs the handler named, with the values written for it and,
// for a value, the answer last. The JSON is the Rust runtime's.

package clarity

import "encoding/json"

// Ask is a question in a reply.
type Ask struct {
	Question Text              `json:"question"`
	Form     AskForm           `json:"form"`
	Handler  string            `json:"handler"`
	Values   []json.RawMessage `json:"values"`
}

// AskForm is what a question asks for: yes or no, or a value of a type.
type AskForm struct {
	// Kind is confirm or value.
	Kind string
	// Yes and No are the buttons of a confirm; nil says OK and Cancel.
	Yes, No *Text
	// Type is the value's: text, integer, decimal, money, date, datetime,
	// duration, enum or record; Of names the enum or entity.
	Type string
	Of   *string
	// Only narrows the choices: the records' ids or the enum's values.
	Only []any
	// Default is where a typed value starts.
	Default any
}

// Confirm is a question with yes and no; the handler runs on yes.
func Confirm(yes, no *Text) AskForm { return AskForm{Kind: "confirm", Yes: yes, No: no} }

// ValueForm is a question for a value of type, of the enum or entity of.
func ValueForm(kind string, of *string, only []any, def any) AskForm {
	return AskForm{Kind: "value", Type: kind, Of: of, Only: only, Default: def}
}

func (f AskForm) MarshalJSON() ([]byte, error) {
	if f.Kind == "confirm" {
		return json.Marshal(struct {
			Kind string `json:"kind"`
			Yes  *Text  `json:"yes"`
			No   *Text  `json:"no"`
		}{f.Kind, f.Yes, f.No})
	}
	return json.Marshal(struct {
		Kind    string  `json:"kind"`
		Type    string  `json:"type"`
		Of      *string `json:"of"`
		Only    []any   `json:"only"`
		Default any     `json:"default"`
	}{f.Kind, f.Type, f.Of, f.Only, f.Default})
}

// Ask asks question when the handler has finished; its answer runs handler
// with values and, for a value, the answer.
func (u *Ui) Ask(question Text, form AskForm, handler string, values ...any) {
	raw := make([]json.RawMessage, 0, len(values))
	for _, value := range values {
		text, err := json.Marshal(value)
		if err != nil && u.err == nil {
			u.err = &InvalidError{err.Error()}
		}
		raw = append(raw, text)
	}
	u.asks = append(u.asks, Ask{question, form, handler, raw})
}

// Answer is an answer as the value a handler takes: text, a number, a date,
// an enum's value.
func Answer[T any](raw json.RawMessage) (T, error) {
	var value T
	if err := json.Unmarshal(raw, &value); err != nil {
		return value, &InvalidError{"an answer that is no value of its type: " + err.Error()}
	}
	return value, nil
}

// AnswerRecord is a record answered: written whole for the handler, as it
// was (unsaved changes too), or chosen by its id and read from the database.
func AnswerRecord[T any, P interface {
	*T
	Record
}](db *Db, raw json.RawMessage) (T, error) {
	var id int64
	if json.Unmarshal(raw, &id) == nil {
		return Get[T, P](db, id)
	}
	return Answer[T](raw)
}

// Button is the text of a confirm's button, as the form holds it.
func Button(text Text) *Text { return &text }

// Named is the enum or entity a question for a value asks for.
func Named(name string) *string { return &name }

// IDs are the ids of records, as a question's choices narrowed to them.
func IDs[T any, P interface {
	*T
	Record
}](records []T) []any {
	ids := make([]any, 0, len(records))
	for i := range records {
		ids = append(ids, P(&records[i]).GetID())
	}
	return ids
}
