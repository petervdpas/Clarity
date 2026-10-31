// Lists of values as fields (U1c): `list<text>`, `list<Genre>`, kept in
// one column as compact JSON (conformance/list-fields.json).

package clarity

import (
	"bytes"
	"encoding/json"
	"regexp"
	"slices"
	"strconv"
	"strings"

	"github.com/shopspring/decimal"
)

// ListValue is the column text of a list: compact JSON, text as it is.
func ListValue[T any](items []T) string {
	if items == nil {
		return "[]"
	}
	var out bytes.Buffer
	encoder := json.NewEncoder(&out)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(items); err != nil {
		return "[]"
	}
	return strings.TrimSuffix(out.String(), "\n")
}

// ReadList reads the items of a list column; none when it is null or empty.
func ReadList[T any](r *Reader, i int, column string) []T {
	items := []T{}
	text := strings.TrimSpace(r.Text(i, column))
	if text == "" {
		return items
	}
	if err := json.Unmarshal([]byte(text), &items); err != nil {
		r.fail(column, text)
		return []T{}
	}
	return items
}

// ListHolds is `list contains item` in code: decimals by their value, dates by
// the day.
func ListHolds[T any](list []T, item T) bool {
	for _, x := range list {
		if sameItem(any(x), any(item)) {
			return true
		}
	}
	return false
}

func sameItem(a, b any) bool {
	switch x := a.(type) {
	case decimal.Decimal:
		return x.Equal(b.(decimal.Decimal))
	case *Date:
		y := b.(*Date)
		if x == nil || y == nil {
			return x == y
		}
		return *x == *y
	}
	return a == b
}

var (
	wholeItem   = regexp.MustCompile(`^-?\d+$`)
	decimalItem = regexp.MustCompile(`^-?\d+(\.\d+)?$`)
)

// AddTyped is what an item typed into the tags input does to a list of
// items of kind (text, integer, decimal, date or enum, with the enum's
// values): the list it gives, with the item at its end unless empty text or
// the list has it; refused when the text is no value of the kind, and the
// list stays. Integers go as numbers, decimals and dates as text.
func AddTyped(kind string, items []any, typed string, values []string) ([]any, bool) {
	text := strings.TrimSpace(typed)
	if text == "" {
		return items, false
	}
	var item any = text
	switch kind {
	case "integer":
		n, err := strconv.ParseInt(text, 10, 64)
		if !wholeItem.MatchString(text) || err != nil {
			return items, true
		}
		item = n
	case "decimal":
		if !decimalItem.MatchString(text) {
			return items, true
		}
	case "date":
		if _, err := ParseDate(text); err != nil {
			return items, true
		}
	case "enum":
		if !slices.Contains(values, text) {
			return items, true
		}
	}
	for _, x := range items {
		if sameTyped(kind, x, item) {
			return items, false
		}
	}
	return append(slices.Clone(items), item), false
}

// sameTyped says whether an item of a list as the screen sends it is item:
// numbers and decimals by their value.
func sameTyped(kind string, a, b any) bool {
	if kind == "integer" || kind == "decimal" {
		x, okA := decimal.NewFromString(strings.TrimSpace(asText(a)))
		y, okB := decimal.NewFromString(strings.TrimSpace(asText(b)))
		return okA == nil && okB == nil && x.Equal(y)
	}
	return asText(a) == asText(b)
}

// asText is an item as text: text as it is, a number as JSON writes it.
func asText(v any) string {
	if s, ok := v.(string); ok {
		return s
	}
	text, _ := json.Marshal(v)
	return string(text)
}
