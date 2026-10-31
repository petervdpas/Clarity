package clarity

// The shown rows of tables, lists, trees and charts (DESIGN 1.7), decided
// here in the runtime: the window's state carries a view per such part,
// how it shows its list as the screen sends it, and after every reply the
// runtime fills in what shows. A view event (a sort, a page, a tree row
// opened or closed) changes the view only, and is no edit.

import (
	"encoding/json"
	"fmt"
	"math"
	"slices"
	"strings"
)

// tableView is a table's view: its list, its columns, its order, the rows
// to a page (nil: no pages) and the page it is at.
type tableView struct {
	List    string        `json:"list"`
	Columns []TableColumn `json:"columns"`
	Order   *TableOrder   `json:"order"`
	Page    *int          `json:"page"`
	At      int           `json:"at"`
}

// treeView is a tree's view: its list, its parent and id fields, and the ids shown open.
type treeView struct {
	List   string `json:"list"`
	Parent string `json:"parent"`
	Key    string `json:"key"`
	Open   []any  `json:"open"`
}

// chartView is a chart's view: its list, the field it groups by and its
// kind, the measure and its field, and an enum's values in their order.
type chartView struct {
	List    string `json:"list"`
	By      string `json:"by"`
	ByKind  string `json:"byKind"`
	Measure string `json:"measure"`
	Of      string `json:"of"`
	Values  []any  `json:"values"`
}

func viewAs(view map[string]any, into any) error {
	text, err := json.Marshal(view)
	if err == nil {
		err = json.Unmarshal(text, into)
	}
	if err != nil {
		return &InvalidError{fmt.Sprintf("a view does not fit its kind: %v", err)}
	}
	return nil
}

// rowsAt are the rows of the list at path; a row that is no record has no fields.
func rowsAt(data map[string]any, path string) []map[string]any {
	value, _ := ValueAt(data, path)
	list, _ := value.([]any)
	rows := make([]map[string]any, len(list))
	for i, row := range list {
		rows[i], _ = row.(map[string]any)
		if rows[i] == nil {
			rows[i] = map[string]any{}
		}
	}
	return rows
}

func treeRowsOf(rows []map[string]any, v treeView) []TreeRow {
	key := v.Key
	if key == "" {
		key = "id"
	}
	out := make([]TreeRow, len(rows))
	for i, row := range rows {
		out[i] = TreeRow{ID: row[key]}
		if v.Parent != "" {
			out[i].Parent = row[v.Parent]
		}
	}
	return out
}

// FillViews fills in what each view of state shows. chosen names the parts
// whose chosen row changed: a table turns to the page that shows it, a tree
// opens the rows above it.
func FillViews(state WindowState, chosen map[string]bool) error {
	for part, view := range state.Views {
		if view == nil {
			continue
		}
		row, picked := state.Selected[part]
		picked = picked && chosen[part]
		switch view["kind"] {
		case "table":
			var v tableView
			if err := viewAs(view, &v); err != nil {
				return err
			}
			rows := rowsAt(state.Data, v.List)
			order := ShownOrder(rows, v.Columns, v.Order)
			if v.Page == nil {
				view["shown"] = order
				delete(view, "pages")
				continue
			}
			at := v.At
			if picked {
				if place := slices.Index(order, row); place >= 0 {
					at = PageWith(*v.Page, place)
				}
			}
			paged := PageOf(len(order), *v.Page, at)
			view["at"] = paged.Page
			view["pages"] = paged
			view["shown"] = order[max(paged.From-1, 0):paged.To]
		case "tree":
			var v treeView
			if err := viewAs(view, &v); err != nil {
				return err
			}
			rows := treeRowsOf(rowsAt(state.Data, v.List), v)
			open := v.Open
			if open == nil {
				open = []any{}
			}
			if picked {
				for _, id := range Above(rows, row) {
					if !slices.ContainsFunc(open, func(o any) bool { return sameID(o, id) }) {
						open = append(open, id)
					}
				}
			}
			view["open"] = open
			view["shown"] = ShownTree(rows, open)
		case "numbers":
			var v struct {
				List   string `json:"list"`
				Points *int   `json:"points"`
			}
			if err := viewAs(view, &v); err != nil {
				return err
			}
			view["numbers"] = NumbersOf(state.Data, v.List, v.Points)
		case "chart":
			var v chartView
			if err := viewAs(view, &v); err != nil {
				return err
			}
			rows := rowsAt(state.Data, v.List)
			chart := make([]ChartRow, len(rows))
			for i, r := range rows {
				chart[i] = ChartRow{By: r[v.By]}
				if v.Of != "" {
					chart[i].Of = r[v.Of]
				}
			}
			view["groups"] = ChartGroups(chart, v.ByKind, v.Measure, v.Values)
		}
	}
	return nil
}

// ViewEvent changes the view of part by the person's action: `sort` by a
// field (the same field again: the other way, from the first page), `page`
// to a page, `open` or `close` a tree row by its id.
func ViewEvent(state WindowState, part string, action map[string]json.RawMessage) error {
	// No action: the screen declared a view, which every reply fills.
	if len(action) == 0 {
		return nil
	}
	view := state.Views[part]
	if view == nil {
		return &InvalidError{fmt.Sprintf("the window shows no rows in %s", part)}
	}
	read := func(name string, into any) error {
		if err := json.Unmarshal(action[name], into); err != nil {
			return &InvalidError{fmt.Sprintf("a view's %s does not fit: %s", name, action[name])}
		}
		return nil
	}
	switch {
	case action["sort"] != nil:
		// Only a sortable table sorts by a header.
		if view["sortable"] != true {
			return nil
		}
		var v tableView
		var field string
		if err := viewAs(view, &v); err != nil {
			return err
		}
		if err := read("sort", &field); err != nil {
			return err
		}
		view["order"] = NextOrder(v.Order, field)
		view["at"] = 1
	case action["page"] != nil:
		var page int
		if err := read("page", &page); err != nil {
			return err
		}
		view["at"] = page
	case action["open"] != nil || action["close"] != nil:
		var v treeView
		if err := viewAs(view, &v); err != nil {
			return err
		}
		var id any
		opens := action["open"] != nil
		name := "close"
		if opens {
			name = "open"
		}
		if err := read(name, &id); err != nil {
			return err
		}
		open := slices.DeleteFunc(slices.Clone(v.Open), func(o any) bool { return sameID(o, id) })
		if open == nil {
			open = []any{}
		}
		if opens {
			open = append(open, id)
		}
		view["open"] = open
	default:
		return &InvalidError{"a view event sorts, turns a page, or opens or closes a row"}
	}
	return nil
}

// NumbersOf are the numbers of the list at path a chart of numbers draws:
// its last points (all when points is nil or 0), decimals as their value;
// what is no number is left out.
func NumbersOf(data map[string]any, path string, points *int) []float64 {
	value, _ := ValueAt(data, path)
	list, _ := value.([]any)
	numbers := []float64{}
	for _, item := range list {
		if text, ok := item.(string); ok && strings.TrimSpace(text) == "" {
			continue
		}
		if n, ok := chartNumber(item); ok && !math.IsInf(n, 0) && !math.IsNaN(n) {
			numbers = append(numbers, n)
		}
	}
	if points != nil && *points > 0 && len(numbers) > *points {
		numbers = numbers[len(numbers)-*points:]
	}
	return numbers
}
