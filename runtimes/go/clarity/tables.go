package clarity

// How a table's rows show, decided here in the runtime
// (conformance/table-order.json, table-pages.json): text ignoring case,
// numbers as numbers, a choice by the label it shows, an empty value last
// either way, equal rows in the order of the data; then cut into pages.
// Rows keep their place in the data; only the order they show in changes.

import (
	"fmt"
	"slices"
	"strconv"
	"strings"

	"golang.org/x/text/collate"
	"golang.org/x/text/language"
)

// TableColumn is a column a table sorts by: its field, its kind (text,
// number, bool, choice) and, for a choice, each value with its label.
type TableColumn struct {
	Field   string  `json:"field"`
	Kind    string  `json:"kind"`
	Choices [][]any `json:"choices,omitempty"`
}

// TableOrder is a table's order: by a field, desc the other way; nil is the order of the data.
type TableOrder struct {
	Field string `json:"field"`
	Desc  bool   `json:"desc"`
}

// NextOrder is what a click on the header of field makes of order: by it, then the other way.
func NextOrder(order *TableOrder, field string) *TableOrder {
	if order != nil && order.Field == field {
		return &TableOrder{Field: field, Desc: !order.Desc}
	}
	return &TableOrder{Field: field}
}

// sortKey is what a value sorts by: a number, or text; nil when it is empty.
func sortKey(column TableColumn, value any) any {
	if value == nil || value == "" {
		return nil
	}
	switch column.Kind {
	case "choice":
		label := asText(value)
		for _, choice := range column.Choices {
			if len(choice) == 2 && sameID(choice[0], value) {
				label = fmt.Sprint(choice[1])
				break
			}
		}
		if label == "" {
			return nil
		}
		return label
	case "number":
		n, err := strconv.ParseFloat(strings.TrimSpace(asText(value)), 64)
		if err != nil {
			return nil
		}
		return n
	case "bool":
		if value == true {
			return 1.0
		}
		return 0.0
	}
	return asText(value)
}

// ShownOrder is the places in the data of rows, in the order they show.
func ShownOrder(rows []map[string]any, columns []TableColumn, order *TableOrder) []int {
	places := make([]int, len(rows))
	for i := range places {
		places[i] = i
	}
	if order == nil {
		return places
	}
	at := slices.IndexFunc(columns, func(c TableColumn) bool { return c.Field == order.Field })
	if at < 0 {
		return places
	}
	keys := make([]any, len(rows))
	for i, row := range rows {
		keys[i] = sortKey(columns[at], row[order.Field])
	}
	// Ignoring case and accents, as the screens' collators do.
	text := collate.New(language.Und, collate.Loose)
	slices.SortStableFunc(places, func(a, b int) int {
		ka, kb := keys[a], keys[b]
		if ka == nil || kb == nil {
			switch {
			case ka == nil && kb == nil:
				return 0
			case ka == nil:
				return 1
			}
			return -1
		}
		by := 0
		na, numA := ka.(float64)
		nb, numB := kb.(float64)
		if numA && numB {
			by = compareNumbers(na, nb)
		} else {
			by = text.CompareString(asText(ka), asText(kb))
		}
		if order.Desc {
			by = -by
		}
		return by
	})
	return places
}

func compareNumbers(a, b float64) int {
	switch {
	case a < b:
		return -1
	case a > b:
		return 1
	}
	return 0
}

// Paged is the page of a table shown a page at a time: the page shown, how
// many pages there are, the first and last row shown counted from 1 (0 and
// 0 for none), and how many rows there are. Text is the bar's text in plain
// digits, which the screen writes in its own.
type Paged struct {
	Page  int    `json:"page"`
	Pages int    `json:"pages"`
	From  int    `json:"from"`
	To    int    `json:"to"`
	Count int    `json:"count"`
	Text  string `json:"-"`
}

// PageOf is page page of count rows, size to a page, within the pages there are.
func PageOf(count, size, page int) Paged {
	size = max(size, 1)
	pages := max(1, (count+size-1)/size)
	if count == 0 {
		return Paged{Page: 1, Pages: 1, Text: "0 / 0"}
	}
	shown := min(max(page, 1), pages)
	from := (shown-1)*size + 1
	to := min(shown*size, count)
	return Paged{Page: shown, Pages: pages, From: from, To: to, Count: count, Text: fmt.Sprintf("%d-%d / %d", from, to, count)}
}

// PageWith is the page that shows the row at place at of the shown order, counted from 0.
func PageWith(size, at int) int {
	return at/max(size, 1) + 1
}
