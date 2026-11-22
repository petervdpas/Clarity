package clarity

// Clarity's JSON: a value as Clarity writes values everywhere, which the
// window wire, the data formats (formats.go) and calls to web APIs use, the
// same text on every stack (conformance/values-json.json). encoding/json
// writes most of it; two things it writes otherwise are written here: a
// decimal keeps the places it holds ("1250.50", where shopspring's own
// writer drops trailing zeros), and a list with no items is [] (where a nil
// slice would be null).

import (
	"bytes"
	"encoding/json"
	"reflect"
	"sort"
	"strings"

	"github.com/shopspring/decimal"
)

// JSONOf is the JSON of a value, as Clarity writes values everywhere.
func JSONOf(value any) string {
	var out bytes.Buffer
	if err := writeJSON(&out, reflect.ValueOf(value)); err != nil {
		return ""
	}
	return out.String()
}

// DecimalText is a decimal as Clarity writes it: with the places it holds,
// 1250.50 as "1250.50" and 100.00 as "100.00".
func DecimalText(d decimal.Decimal) string {
	if places := -d.Exponent(); places > 0 {
		return d.StringFixed(places)
	}
	return d.String()
}

var (
	decimalType   = reflect.TypeOf(decimal.Decimal{})
	marshalerType = reflect.TypeOf((*json.Marshaler)(nil)).Elem()
)

// leaf writes v as encoding/json does, without escaping <, > and & for HTML.
func leaf(out *bytes.Buffer, v any) error {
	var one bytes.Buffer
	encoder := json.NewEncoder(&one)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(v); err != nil {
		return err
	}
	out.WriteString(strings.TrimSuffix(one.String(), "\n"))
	return nil
}

func writeJSON(out *bytes.Buffer, v reflect.Value) error {
	if !v.IsValid() {
		out.WriteString("null")
		return nil
	}
	if v.Type() == decimalType {
		return leaf(out, DecimalText(v.Interface().(decimal.Decimal)))
	}
	switch v.Kind() {
	case reflect.Pointer, reflect.Interface:
		if v.IsNil() {
			out.WriteString("null")
			return nil
		}
	}
	// A value with a writer of its own (a date, a picture, a file) writes itself.
	if v.Type().Implements(marshalerType) {
		return leaf(out, v.Interface())
	}
	if v.Kind() != reflect.Pointer && v.CanAddr() && reflect.PointerTo(v.Type()).Implements(marshalerType) {
		return leaf(out, v.Addr().Interface())
	}
	switch v.Kind() {
	case reflect.Pointer, reflect.Interface:
		return writeJSON(out, v.Elem())
	case reflect.Struct:
		return writeStruct(out, v)
	case reflect.Slice:
		if v.Type().Elem().Kind() == reflect.Uint8 {
			return leaf(out, v.Interface())
		}
		return writeItems(out, v)
	case reflect.Array:
		return writeItems(out, v)
	case reflect.Map:
		return writeMap(out, v)
	}
	return leaf(out, v.Interface())
}

func writeItems(out *bytes.Buffer, v reflect.Value) error {
	out.WriteByte('[')
	for i := 0; i < v.Len(); i++ {
		if i > 0 {
			out.WriteByte(',')
		}
		if err := writeJSON(out, v.Index(i)); err != nil {
			return err
		}
	}
	out.WriteByte(']')
	return nil
}

func writeMap(out *bytes.Buffer, v reflect.Value) error {
	if v.IsNil() {
		out.WriteString("null")
		return nil
	}
	if v.Type().Key().Kind() != reflect.String {
		return leaf(out, v.Interface())
	}
	keys := make([]string, 0, v.Len())
	for _, k := range v.MapKeys() {
		keys = append(keys, k.String())
	}
	// In the order of their names, as encoding/json writes a map.
	sort.Strings(keys)
	out.WriteByte('{')
	for i, k := range keys {
		if i > 0 {
			out.WriteByte(',')
		}
		if err := leaf(out, k); err != nil {
			return err
		}
		out.WriteByte(':')
		if err := writeJSON(out, v.MapIndex(reflect.ValueOf(k).Convert(v.Type().Key()))); err != nil {
			return err
		}
	}
	out.WriteByte('}')
	return nil
}

// writeStruct writes a struct's exported fields in their order, each by the
// name its json tag gives (none: its own), leaving out `-` and an empty
// `omitempty`, and the fields of an embedded struct as its own, as
// encoding/json does.
func writeStruct(out *bytes.Buffer, v reflect.Value) error {
	out.WriteByte('{')
	first := true
	var fields func(v reflect.Value) error
	fields = func(v reflect.Value) error {
		t := v.Type()
		for i := 0; i < t.NumField(); i++ {
			f := t.Field(i)
			tag := f.Tag.Get("json")
			if tag == "-" {
				continue
			}
			name, options, _ := strings.Cut(tag, ",")
			if f.Anonymous && name == "" && f.Type.Kind() == reflect.Struct {
				if err := fields(v.Field(i)); err != nil {
					return err
				}
				continue
			}
			if !f.IsExported() {
				continue
			}
			value := v.Field(i)
			if strings.Contains(options, "omitempty") && omitted(value) {
				continue
			}
			if name == "" {
				name = f.Name
			}
			if !first {
				out.WriteByte(',')
			}
			first = false
			if err := leaf(out, name); err != nil {
				return err
			}
			out.WriteByte(':')
			if strings.Contains(options, "string") {
				if err := leaf(out, JSONOf(value.Interface())); err != nil {
					return err
				}
				continue
			}
			if err := writeJSON(out, value); err != nil {
				return err
			}
		}
		return nil
	}
	if err := fields(v); err != nil {
		return err
	}
	out.WriteByte('}')
	return nil
}

// omitted is what `omitempty` leaves out: false, 0, "", nil, and an empty list or map.
func omitted(v reflect.Value) bool {
	switch v.Kind() {
	case reflect.Array, reflect.Map, reflect.Slice, reflect.String:
		return v.Len() == 0
	case reflect.Bool:
		return !v.Bool()
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return v.Int() == 0
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		return v.Uint() == 0
	case reflect.Float32, reflect.Float64:
		return v.Float() == 0
	case reflect.Interface, reflect.Pointer:
		return v.IsNil()
	}
	return false
}
