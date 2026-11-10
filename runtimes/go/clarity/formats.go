package clarity

// The data formats of code (docs/FORMATS-PLAN.md): `read Order from json
// text` and `json of value`, through Clarity.Formats, the one
// implementation every stack calls. The program's own cgo file (formats.go,
// generated only when the code uses a format) calls the library; this
// module stays pure Go. The library reads the format and fits it to what the
// program wants; encoding/json decodes what was fitted. Writing goes the
// other way: encoding/json writes the value, the library writes it in the
// format, in Clarity's one form.

import (
	"encoding/json"
	"errors"
	"fmt"
)

// FormatReader reads input in a format with its options (a JSON object, ""
// for none) and fits it to the kind; what was fitted, as JSON. The program's
// call into Clarity.Formats.
type FormatReader func(format, input, options, kind string) (string, error)

// FormatWriter writes a value, as JSONOf wrote it, in a format with its options.
type FormatWriter func(format, value, options, kind string) (string, error)

// ReadFormat reads input through read and decodes what was fitted as T.
func ReadFormat[T any](read FormatReader, format, input, options, kind string) (T, error) {
	var out T
	fitted, err := read(format, input, options, kind)
	if err != nil {
		return out, err
	}
	if err := json.Unmarshal([]byte(fitted), &out); err != nil {
		return out, fmt.Errorf("what was read does not fit: %w", err)
	}
	return out, nil
}

// WriteFormat writes value in a format through write: JSONOf writes it,
// the library writes it in the format.
func WriteFormat(write FormatWriter, format string, value any, options, kind string) string {
	encoded := JSONOf(value)
	written, err := write(format, encoded, options, kind)
	if err != nil {
		// A kind the generator wrote always describes the value; should it not, the value as JSONOf wrote it.
		return encoded
	}
	return written
}

// FormatProblem is why Clarity.Formats could not read or write, as every stack says it.
func FormatProblem(text string) error {
	return errors.New(text)
}
