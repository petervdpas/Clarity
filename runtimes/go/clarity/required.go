// Required fields (LANGUAGE.md, Fields): a save refuses a record whose
// required field is empty, after the entity's before save and before
// anything is written, as every runtime does (conformance/required.json).

package clarity

import (
	"reflect"
	"strings"
)

// RequiredField is a stored field a record needs a value in: its column (a
// file's content column) and its label as written, a key as @key or the
// text itself.
type RequiredField struct {
	Column string `json:"column"`
	Label  string `json:"label"`
}

// Requirer is a record whose entity has required fields, in its order.
type Requirer interface{ Required() []RequiredField }

// MissingRequired is the refusal of a record with these columns and values
// whose required field is empty (no value, or empty text; 0 and false are
// values): the first such field in the entity's order, named by its label in
// the person's language. Nil when every required field has a value.
func MissingRequired(required []RequiredField, columns []string, values []any) error {
	for _, field := range required {
		for i, column := range columns {
			if column != field.Column || i >= len(values) || !empty(values[i]) {
				continue
			}
			return Refuse(KeyText("fieldRequired").With("field", SlotText(labelText(field.Label))))
		}
	}
	return nil
}

// labelText is a label as people read it: a label written as @key takes the
// key's value; a plain one is itself.
func labelText(label string) string {
	if key, ok := strings.CutPrefix(label, "@"); ok {
		return TextOf(key)
	}
	return label
}

// empty says whether a stored value is no value: nil, a nil pointer, or empty text.
func empty(value any) bool {
	switch v := value.(type) {
	case nil:
		return true
	case string:
		return v == ""
	}
	r := reflect.ValueOf(value)
	switch r.Kind() {
	case reflect.Pointer, reflect.Interface, reflect.Map, reflect.Slice:
		return r.IsNil()
	}
	return false
}

// checkRequired refuses a record whose required field is empty.
func checkRequired(record Record) error {
	r, ok := record.(Requirer)
	if !ok {
		return nil
	}
	return MissingRequired(r.Required(), record.Columns(), record.Values())
}
