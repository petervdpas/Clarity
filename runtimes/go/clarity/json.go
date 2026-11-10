package clarity

// Clarity's JSON: a value as Clarity writes values everywhere, which the
// window wire, the data formats (formats.go) and calls to web APIs use.

import (
	"bytes"
	"encoding/json"
	"strings"
)

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
