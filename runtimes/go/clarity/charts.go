package clarity

// A chart's groups (U1c), decided here in the runtime
// (conformance/charts.json, `groups`): a list's rows grouped by a field,
// each group counted, summed or averaged, in the order of their values. The
// axis and the colors of a pie's slices are how it is drawn, the renderer's.

import (
	"encoding/json"
	"math"
	"slices"
	"strconv"
	"strings"
)

// ChartRow is a row of a chart: the value it is grouped by, and its number.
type ChartRow struct {
	By any `json:"by"`
	Of any `json:"of"`
}

// ChartGroup is a group of a chart: its value, and its count, sum or average.
type ChartGroup struct {
	Key   any     `json:"key"`
	Value float64 `json:"value"`
}

func chartNumber(value any) (float64, bool) {
	switch v := value.(type) {
	case float64:
		return v, true
	case int64:
		return float64(v), true
	case int:
		return float64(v), true
	case json.Number:
		n, err := v.Float64()
		return n, err == nil
	case string:
		n, err := strconv.ParseFloat(strings.TrimSpace(v), 64)
		return n, err == nil && strings.TrimSpace(v) != ""
	}
	return 0, false
}

func round12(n float64) float64 { return math.Round(n*1e12) / 1e12 }

// ChartGroups are the groups of rows by the value they have (byKind: text,
// number, date, enum or bool, with an enum's values in order), each the
// count, sum or average (measure) of its numbers. Rows with no value are left out.
func ChartGroups(rows []ChartRow, byKind, measure string, order []any) []ChartGroup {
	type group struct {
		key          any
		count, numbs int
		sum          float64
	}
	var groups []*group
	at := map[string]*group{}
	for _, row := range rows {
		if row.By == nil || row.By == "" {
			continue
		}
		g := at[idKey(row.By)]
		if g == nil {
			g = &group{key: row.By}
			at[idKey(row.By)] = g
			groups = append(groups, g)
		}
		g.count++
		if n, ok := chartNumber(row.Of); ok {
			g.sum += n
			g.numbs++
		}
	}
	slices.SortStableFunc(groups, func(a, b *group) int {
		switch byKind {
		case "number":
			x, _ := chartNumber(a.key)
			y, _ := chartNumber(b.key)
			return compareNumbers(x, y)
		case "enum":
			return slices.IndexFunc(order, func(v any) bool { return sameID(v, a.key) }) - slices.IndexFunc(order, func(v any) bool { return sameID(v, b.key) })
		case "bool":
			return boolNumber(a.key) - boolNumber(b.key)
		}
		sa, sb := asText(a.key), asText(b.key)
		if byKind != "date" {
			if c := strings.Compare(strings.ToLower(sa), strings.ToLower(sb)); c != 0 {
				return c
			}
		}
		return strings.Compare(sa, sb)
	})
	out := []ChartGroup{}
	for _, g := range groups {
		value := float64(g.count)
		switch measure {
		case "sum":
			value = g.sum
		case "average":
			value = 0
			if g.numbs > 0 {
				value = g.sum / float64(g.numbs)
			}
		}
		out = append(out, ChartGroup{Key: g.key, Value: round12(value)})
	}
	return out
}

func boolNumber(v any) int {
	if v == true {
		return 1
	}
	return 0
}
