// A table's shown order and pages (LANGUAGE.md, Tables) against the cases
// every runtime shares (conformance/table-order.json, table-pages.json).

package clarity

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

func TestATablesRowsShowInTheOrderEveryRuntimeShows(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/table-order.json")
	must(t, err)
	var file struct {
		Columns []TableColumn    `json:"columns"`
		Rows    []map[string]any `json:"rows"`
		Cases   []struct {
			Name     string            `json:"name"`
			Sortable bool              `json:"sortable"`
			Order    *TableOrder       `json:"order"`
			Steps    []json.RawMessage `json:"steps"`
			Expect   struct {
				Shown []int `json:"shown"`
			} `json:"expect"`
		} `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Cases) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range file.Cases {
		order := c.Order
		for _, raw := range c.Steps {
			var step []any
			must(t, json.Unmarshal(raw, &step))
			// A click on a row is the screen's; a click on a header sorts.
			if step[0] == "sort" && c.Sortable {
				order = NextOrder(order, step[1].(string))
			}
		}
		if got := ShownOrder(file.Rows, file.Columns, order); !reflect.DeepEqual(got, c.Expect.Shown) {
			t.Errorf("%s: %v, want %v", c.Name, got, c.Expect.Shown)
		}
	}
}

func TestATableIsPagedAsEveryRuntimePagesIt(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/table-pages.json")
	must(t, err)
	var file struct {
		Pages []struct {
			Count, Size, Page, Shows, Pages, From, To int
			Text                                      string
		} `json:"pages"`
		Selected []struct{ Size, At, Page int } `json:"selected"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Pages) == 0 || len(file.Selected) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range file.Pages {
		want := Paged{Page: c.Shows, Pages: c.Pages, From: c.From, To: c.To, Count: c.Count, Text: c.Text}
		if got := PageOf(c.Count, c.Size, c.Page); got != want {
			t.Errorf("page %d of %d by %d: %+v, want %+v", c.Page, c.Count, c.Size, got, want)
		}
	}
	for _, c := range file.Selected {
		if got := PageWith(c.Size, c.At); got != c.Page {
			t.Errorf("row %d by %d: page %d, want %d", c.At, c.Size, got, c.Page)
		}
	}
}
