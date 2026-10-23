// Lists of values as fields (U1c): `list<text>`, `list<Genre>`, kept in
// one column as compact JSON (conformance/list-fields.json).

package clarity

import (
	"bytes"
	"encoding/json"
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

// Holds is `list contains item` in code: decimals by their value, dates by
// the day.
func Holds[T any](list []T, item T) bool {
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
