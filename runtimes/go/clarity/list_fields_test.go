// Lists of values as fields (U1c), kept in one column as JSON, against the
// cases every runtime shares (conformance/list-fields.json).

package clarity

import (
	"encoding/json"
	"os"
	"testing"

	"github.com/shopspring/decimal"
)

type listCase struct {
	Item   string
	Items  json.RawMessage
	Column *string
}

func listFieldCases(t *testing.T, part string) []listCase {
	text, err := os.ReadFile("../../../conformance/list-fields.json")
	must(t, err)
	var all map[string]json.RawMessage
	must(t, json.Unmarshal(text, &all))
	var cases []listCase
	must(t, json.Unmarshal(all[part], &cases))
	return cases
}

func same(t *testing.T, c listCase, got any) {
	t.Helper()
	text, err := json.Marshal(got)
	must(t, err)
	var a, b any
	must(t, json.Unmarshal(text, &a))
	must(t, json.Unmarshal(c.Items, &b))
	if string(mustJSON(t, a)) != string(mustJSON(t, b)) {
		t.Errorf("%s %v: %s", c.Item, c.Column, text)
	}
}

func mustJSON(t *testing.T, v any) []byte {
	text, err := json.Marshal(v)
	must(t, err)
	return text
}

func keptAs[T any](t *testing.T, c listCase) {
	var items []T
	must(t, json.Unmarshal(c.Items, &items))
	if got := ListValue(items); got != *c.Column {
		t.Errorf("%s: %s, not %s", c.Item, got, *c.Column)
	}
}

func readAs[T any](t *testing.T, c listCase) {
	values := []any{nil}
	if c.Column != nil {
		values[0] = *c.Column
	}
	read := NewReader(values)
	items := ReadList[T](read, 0, "tags")
	must(t, read.Err())
	same(t, c, items)
}

func TestAListIsKeptAsCompactJSON(t *testing.T) {
	for _, c := range listFieldCases(t, "kept") {
		switch c.Item {
		case "text", "enum":
			keptAs[string](t, c)
		case "integer":
			keptAs[int64](t, c)
		case "decimal":
			keptAs[decimal.Decimal](t, c)
		case "date":
			keptAs[*Date](t, c)
		}
	}
}

func TestAColumnGivesItsItems(t *testing.T) {
	for _, c := range listFieldCases(t, "read") {
		switch c.Item {
		case "text", "enum":
			readAs[string](t, c)
		case "integer":
			readAs[int64](t, c)
		case "decimal":
			readAs[decimal.Decimal](t, c)
		case "date":
			readAs[*Date](t, c)
		}
	}
}

func TestAListHoldsItemsByTheirValue(t *testing.T) {
	day := func(text string) *Date { d, _ := ParseDate(text); return &d }
	if !Holds([]decimal.Decimal{decimal.RequireFromString("1.50")}, decimal.RequireFromString("1.5")) {
		t.Error("1.50 is 1.5")
	}
	if !Holds([]*Date{day("2026-10-23")}, day("2026-10-23")) || Holds([]*Date{day("2026-10-23")}, nil) {
		t.Error("dates by the day")
	}
	if !Holds([]string{"jazz"}, "jazz") || Holds([]string{"jazz"}, "Jazz") {
		t.Error("text as it is")
	}
}
