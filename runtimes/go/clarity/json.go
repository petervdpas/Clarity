package clarity

// JSON in code (DESIGN 5.23): `json of value` and `read Order from json
// text`. A service's JSON is not Clarity's own, so what is read is fitted to
// the entity first: names match ignoring case, underscores and hyphens,
// numbers and strings of numbers stand for each other, what is left out
// keeps its default. Every runtime fits alike (conformance/json.json); the
// record's own decoder then reads what was fitted.

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"strconv"
	"strings"
)

// JSONKind is what a field of an entity takes from JSON: text, integer,
// decimal (and money), bool, date, datetime, enum (one of Values, any case),
// values (a list whose items fit Item), record (an object fitted to Entity's
// Fields over Base, a new record of it), records (a list of such objects),
// or other (never read).
type JSONKind struct {
	Is     string
	Values []string
	Item   *JSONKind
	Entity string
	Fields []JSONField
	Base   any
}

// JSONField is a field of an entity, as JSON is fitted to it.
type JSONField struct {
	Name string
	Kind JSONKind
}

// JSONOf is the JSON of a value, as Clarity writes values everywhere.
func JSONOf(value any) string {
	var out bytes.Buffer
	encoder := json.NewEncoder(&out)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(value); err != nil {
		return ""
	}
	return strings.TrimSuffix(out.String(), "\n")
}

func decode(text string) (any, error) {
	decoder := json.NewDecoder(strings.NewReader(text))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return nil, err
	}
	if decoder.More() {
		return nil, errors.New("more after the value")
	}
	return value, nil
}

// baseOf is a new record as Clarity writes it, to fit over.
func baseOf(record any) map[string]any {
	value, _ := decode(JSONOf(record))
	object, _ := value.(map[string]any)
	if object == nil {
		object = map[string]any{}
	}
	return object
}

// FitJSON is what ReadJSON decodes: text as one record of entity (or, with
// many, a list of them), starting at at (object names with dots between),
// fitted to fields over base, a new record.
func FitJSON(text, at string, many bool, entity string, fields []JSONField, base any) (any, error) {
	value, err := decode(text)
	if err != nil {
		return nil, errors.New("the text is no JSON")
	}
	if at != "" {
		var walked []string
		for _, step := range strings.Split(at, ".") {
			walked = append(walked, step)
			object, _ := value.(map[string]any)
			next, found := object[step]
			if !found {
				return nil, fmt.Errorf("the JSON has no '%s'", strings.Join(walked, "."))
			}
			value = next
		}
	}
	template := baseOf(base)
	if many {
		items, isList := value.([]any)
		if !isList {
			return nil, fmt.Errorf("the JSON is not a list of %s", entity)
		}
		fitted := make([]any, 0, len(items))
		for i, item := range items {
			object, isObject := item.(map[string]any)
			if !isObject {
				return nil, fmt.Errorf("item %d of the list is not one %s", i+1, entity)
			}
			record, err := fitRecord(object, entity, fields, template)
			if err != nil {
				return nil, fmt.Errorf("item %d: %w", i+1, err)
			}
			fitted = append(fitted, record)
		}
		return fitted, nil
	}
	object, isObject := value.(map[string]any)
	if !isObject {
		return nil, fmt.Errorf("the JSON is not one %s", entity)
	}
	return fitRecord(object, entity, fields, template)
}

// ReadJSON reads text as one record of entity, fitted to fields over base.
func ReadJSON[T any](text, at, entity string, fields []JSONField, base T) (*T, error) {
	fitted, err := FitJSON(text, at, false, entity, fields, base)
	if err != nil {
		return nil, err
	}
	record := new(T)
	if err := json.Unmarshal([]byte(JSONOf(fitted)), record); err != nil {
		return nil, fmt.Errorf("the JSON does not fit %s: %v", entity, err)
	}
	return record, nil
}

// ReadJSONList reads text as a list of records of entity, fitted to fields
// over base.
func ReadJSONList[T any](text, at, entity string, fields []JSONField, base T) ([]T, error) {
	fitted, err := FitJSON(text, at, true, entity, fields, base)
	if err != nil {
		return nil, err
	}
	var records []T
	if err := json.Unmarshal([]byte(JSONOf(fitted)), &records); err != nil {
		return nil, fmt.Errorf("the JSON does not fit %s: %v", entity, err)
	}
	if records == nil {
		records = []T{}
	}
	return records, nil
}

// plainName is a name as names are matched: lower case, without
// underscores, hyphens and spaces.
func plainName(name string) string {
	return strings.ToLower(strings.NewReplacer("_", "", "-", "", " ", "").Replace(name))
}

func fitRecord(object map[string]any, entity string, fields []JSONField, template map[string]any) (map[string]any, error) {
	out := make(map[string]any, len(template))
	for name, value := range template {
		out[name] = value
	}
	for _, field := range fields {
		given, found := object[field.Name]
		if !found {
			for name, value := range object {
				if plainName(name) == plainName(field.Name) {
					given, found = value, true
					break
				}
			}
		}
		if !found || given == nil || field.Kind.Is == "other" {
			continue
		}
		// Records inside the record (DESIGN 5.25), each fitted to its own entity.
		if field.Kind.Is == "record" || field.Kind.Is == "records" {
			value, err := fitNested(given, field, entity)
			if err != nil {
				return nil, err
			}
			out[field.Name] = value
			continue
		}
		value, fits := fitValue(given, field.Kind)
		if !fits {
			return nil, fmt.Errorf("field '%s' of %s is %s, not %s", field.Name, entity, wanted(field.Kind), JSONOf(given))
		}
		out[field.Name] = value
	}
	return out, nil
}

// fitNested fits a record inside the record, or a list of them, to the
// field's own entity; an error says where it is.
func fitNested(given any, field JSONField, entity string) (any, error) {
	kind := field.Kind
	template := baseOf(kind.Base)
	if kind.Is == "record" {
		object, isObject := given.(map[string]any)
		if !isObject {
			return nil, fmt.Errorf("field '%s' of %s is one %s, not %s", field.Name, entity, kind.Entity, JSONOf(given))
		}
		record, err := fitRecord(object, kind.Entity, kind.Fields, template)
		if err != nil {
			return nil, fmt.Errorf("field '%s' of %s: %w", field.Name, entity, err)
		}
		return record, nil
	}
	items, isList := given.([]any)
	if !isList {
		return nil, fmt.Errorf("field '%s' of %s is a list of %s, not %s", field.Name, entity, kind.Entity, JSONOf(given))
	}
	fitted := make([]any, 0, len(items))
	for i, item := range items {
		object, isObject := item.(map[string]any)
		if !isObject {
			return nil, fmt.Errorf("field '%s' of %s, item %d is not one %s", field.Name, entity, i+1, kind.Entity)
		}
		record, err := fitRecord(object, kind.Entity, kind.Fields, template)
		if err != nil {
			return nil, fmt.Errorf("field '%s' of %s, item %d: %w", field.Name, entity, i+1, err)
		}
		fitted = append(fitted, record)
	}
	return fitted, nil
}

func wanted(kind JSONKind) string {
	switch kind.Is {
	case "text":
		return "text"
	case "integer":
		return "an integer"
	case "decimal":
		return "a decimal"
	case "bool":
		return "true or false"
	case "date":
		return "a date"
	case "datetime":
		return "a date and time"
	case "enum":
		switch len(kind.Values) {
		case 0:
			return "an enum value"
		case 1:
			return kind.Values[0]
		}
		return strings.Join(kind.Values[:len(kind.Values)-1], ", ") + " or " + kind.Values[len(kind.Values)-1]
	case "values":
		return "a list"
	}
	return "something else"
}

func allDigits(text string) bool {
	if text == "" {
		return false
	}
	for _, c := range text {
		if c < '0' || c > '9' {
			return false
		}
	}
	return true
}

func isDecimalText(text string) bool {
	digits := strings.TrimPrefix(text, "-")
	whole, fraction, dotted := strings.Cut(digits, ".")
	return allDigits(whole) && (!dotted || allDigits(fraction))
}

func isDateText(text string) bool {
	return len(text) == 10 && text[4] == '-' && text[7] == '-' && allDigits(text[0:4]) && allDigits(text[5:7]) && allDigits(text[8:10])
}

// dateTimeText is a date and time as Clarity writes it,
// YYYY-MM-DDTHH:MM:SS, from the same with or without seconds, a fraction of
// a second and a Z.
func dateTimeText(text string) (string, bool) {
	text = strings.TrimSuffix(text, "Z")
	at := strings.IndexAny(text, "T ")
	if at < 0 || !isDateText(text[:at]) {
		return "", false
	}
	date, clock := text[:at], text[at+1:]
	clock, _, _ = strings.Cut(clock, ".")
	parts := strings.Split(clock, ":")
	two := func(p string) bool { return len(p) == 2 && allDigits(p) }
	switch {
	case len(parts) == 2 && two(parts[0]) && two(parts[1]):
		return date + "T" + parts[0] + ":" + parts[1] + ":00", true
	case len(parts) == 3 && two(parts[0]) && two(parts[1]) && two(parts[2]):
		return date + "T" + clock, true
	}
	return "", false
}

func fitValue(given any, kind JSONKind) (any, bool) {
	switch kind.Is {
	case "text":
		switch v := given.(type) {
		case string:
			return v, true
		case json.Number, bool:
			return JSONOf(v), true
		}
	case "integer":
		switch v := given.(type) {
		case json.Number:
			if n, err := v.Int64(); err == nil {
				return n, true
			}
			if f, err := v.Float64(); err == nil && f == float64(int64(f)) {
				return int64(f), true
			}
		case string:
			if n, err := strconv.ParseInt(strings.TrimSpace(v), 10, 64); err == nil {
				return n, true
			}
		}
	case "decimal":
		switch v := given.(type) {
		case json.Number:
			return v.String(), true
		case string:
			if text := strings.TrimSpace(v); isDecimalText(text) {
				return text, true
			}
		}
	case "bool":
		if v, isBool := given.(bool); isBool {
			return v, true
		}
	case "date":
		if v, isText := given.(string); isText {
			if isDateText(v) {
				return v, true
			}
			if d, fits := dateTimeText(v); fits {
				return d[:10], true
			}
		}
	case "datetime":
		if v, isText := given.(string); isText {
			return dateTimeText(v)
		}
	case "enum":
		if v, isText := given.(string); isText {
			for _, value := range kind.Values {
				if strings.EqualFold(value, v) {
					return value, true
				}
			}
		}
	case "values":
		if items, isList := given.([]any); isList && kind.Item != nil {
			fitted := make([]any, 0, len(items))
			for _, item := range items {
				value, fits := fitValue(item, *kind.Item)
				if !fits {
					return nil, false
				}
				fitted = append(fitted, value)
			}
			return fitted, true
		}
	}
	return nil, false
}
