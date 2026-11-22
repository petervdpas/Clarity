package clarity

import (
	"encoding/json"
	"os"
	"strconv"
	"testing"

	"github.com/shopspring/decimal"
)

// A value written into JSON as every stack writes it for parts, reports, the
// wire and `json of` (conformance/values-json.json): JSONOf, in the types a
// generated record holds.

func valueOf(t *testing.T, kind string, value any) any {
	text, _ := value.(string)
	switch kind {
	case "decimal", "money":
		d, err := decimal.NewFromString(text)
		must(t, err)
		return d
	case "integer":
		n, err := strconv.ParseInt(text, 10, 64)
		must(t, err)
		return n
	case "date":
		if value == nil {
			return (*Date)(nil)
		}
		d, err := ParseDate(text)
		must(t, err)
		return &d
	case "datetime":
		if value == nil {
			return (*DateTime)(nil)
		}
		d, err := ParseDateTime(text)
		must(t, err)
		return &d
	case "ref":
		if value == nil {
			return (*int64)(nil)
		}
		n, err := strconv.ParseInt(text, 10, 64)
		must(t, err)
		return &n
	case "bool":
		return value.(bool)
	case "text":
		return text
	case "list":
		// A generated record's held list as it starts: no items yet.
		var items []int64
		return items
	}
	t.Fatalf("no kind %s", kind)
	return nil
}

func TestEveryValueIsWrittenAsEveryStackWritesIt(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/values-json.json")
	must(t, err)
	var file struct {
		Cases []struct {
			Name  string `json:"name"`
			Kind  string `json:"kind"`
			Value any    `json:"value"`
			JSON  string `json:"json"`
		} `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	for _, c := range file.Cases {
		if got := JSONOf(valueOf(t, c.Kind, c.Value)); got != c.JSON {
			t.Errorf("%s: %s, want %s", c.Name, got, c.JSON)
		}
	}
}
