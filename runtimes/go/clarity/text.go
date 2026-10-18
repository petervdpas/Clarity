// Text the screen shows (a message, a refusal, a caption, an error): the
// text as written in Clarity with a typed value per slot, so the screen
// fills each slot in the person's format. On the wire it is a plain string,
// or the text with its slot values, as from the Rust runtime.

package clarity

import (
	"encoding/json"
	"strings"

	"github.com/shopspring/decimal"
)

// Slot is the value of one slot, with what the screen needs to format it.
type Slot struct {
	Kind  string `json:"kind"`
	Value any    `json:"value"`
	Scale *int   `json:"scale,omitempty"`
	// Format is the named format of the field the value comes from.
	Format *string `json:"format,omitempty"`
	// Enum is the enum of an enum value, for its label.
	Enum *string `json:"enum,omitempty"`
}

func format(name string) *string {
	if name == "" {
		return nil
	}
	return &name
}

func SlotText(value string) Slot { return Slot{Kind: "text", Value: value} }

// SlotInteger is a whole number; format is the field's named format, or "".
func SlotInteger(value int64, format_ string) Slot {
	return Slot{Kind: "integer", Value: value, Format: format(format_)}
}

// SlotDecimal is a decimal, as text so no precision is lost; scale is the
// field's decimals, or -1 for a value of its own.
func SlotDecimal(value decimal.Decimal, scale int, format_ string) Slot {
	s := Slot{Kind: "decimal", Value: value.String(), Format: format(format_)}
	if scale >= 0 {
		s.Scale = &scale
	}
	return s
}

func SlotMoney(value decimal.Decimal, format_ string) Slot {
	return Slot{Kind: "money", Value: value.String(), Format: format(format_)}
}

func SlotDate(value *Date, format_ string) Slot {
	s := Slot{Kind: "date", Format: format(format_)}
	if value != nil {
		s.Value = value.String()
	}
	return s
}

func SlotDateTime(value *DateTime, format_ string) Slot {
	s := Slot{Kind: "dateTime", Format: format(format_)}
	if value != nil {
		s.Value = value.String()
	}
	return s
}

// SlotDuration is a duration, in whole seconds.
func SlotDuration(seconds int64) Slot { return Slot{Kind: "duration", Value: seconds} }

func SlotBool(value bool) Slot { return Slot{Kind: "bool", Value: value} }

// SlotEnum is an enum value, by its name; the screen shows its label.
func SlotEnum(value string, enum string) Slot { return Slot{Kind: "enum", Value: value, Enum: &enum} }

// plain is the value as plain text, for places that are not a screen.
func (s Slot) plain() string {
	switch v := s.Value.(type) {
	case nil:
		return ""
	case bool:
		if v {
			return "yes"
		}
		return "no"
	case string:
		return v
	default:
		out, _ := json.Marshal(v)
		return string(out)
	}
}

// Text is text to show, with values in slots: Saved {customer.name}.
type Text struct {
	// Template is the text as written in Clarity; {{ and }} are braces.
	Template string
	// Values are the value of each slot, by its path as written: customer.name.
	Values map[string]Slot
}

// NewText is text as written in Clarity, before its slots get values.
func NewText(template string) Text { return Text{Template: template} }

// PlainText is text shown as it is: a brace in it is no slot.
func PlainText(text string) Text {
	return NewText(strings.NewReplacer("{", "{{", "}", "}}").Replace(text))
}

// With gives the slot at path its value.
func (t Text) With(path string, value Slot) Text {
	values := make(map[string]Slot, len(t.Values)+1)
	for k, v := range t.Values {
		values[k] = v
	}
	values[path] = value
	t.Values = values
	return t
}

// Show is the text with its slots filled plainly, without a person's
// format: for logs, and for code that reads a caption back.
func (t Text) Show() string {
	var out strings.Builder
	rest := t.Template
	for {
		at := strings.IndexAny(rest, "{}")
		if at < 0 {
			break
		}
		out.WriteString(rest[:at])
		brace := rest[at]
		after := rest[at+1:]
		// {{ and }} are braces.
		if len(after) > 0 && after[0] == brace {
			out.WriteByte(brace)
			rest = after[1:]
			continue
		}
		if brace == '}' {
			out.WriteByte('}')
			rest = after
			continue
		}
		end := strings.IndexByte(after, '}')
		if end < 0 {
			end = len(after)
		}
		if slot, ok := t.Values[after[:end]]; ok {
			out.WriteString(slot.plain())
		}
		if end < len(after) {
			rest = after[end+1:]
		} else {
			rest = ""
		}
	}
	out.WriteString(rest)
	return out.String()
}

// MarshalJSON writes a plain string, or the text with its slot values.
func (t Text) MarshalJSON() ([]byte, error) {
	if len(t.Values) == 0 {
		return json.Marshal(t.Show())
	}
	return json.Marshal(struct {
		Text   string          `json:"text"`
		Values map[string]Slot `json:"values"`
	}{t.Template, t.Values})
}

func (t *Text) UnmarshalJSON(data []byte) error {
	var plain string
	if err := json.Unmarshal(data, &plain); err == nil {
		*t = PlainText(plain)
		return nil
	}
	var wire struct {
		Text   string          `json:"text"`
		Values map[string]Slot `json:"values"`
	}
	if err := json.Unmarshal(data, &wire); err != nil {
		return err
	}
	*t = Text{wire.Text, wire.Values}
	return nil
}
