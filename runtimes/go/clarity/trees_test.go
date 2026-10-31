// Trees of U1c against the cases every runtime shares (conformance/trees.json).

package clarity

import (
	"encoding/json"
	"os"
	"testing"
)

func treeRows(rows []map[string]any) []TreeRow {
	out := make([]TreeRow, len(rows))
	for i, r := range rows {
		out[i] = TreeRow{ID: r["id"], Parent: r["parent"]}
	}
	return out
}

func TestATreeShowsItsRowsAsEveryRuntimeShowsThem(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/trees.json")
	must(t, err)
	var file struct {
		Cases []struct {
			Name  string           `json:"name"`
			Rows  []map[string]any `json:"rows"`
			Open  []any            `json:"open"`
			Shows json.RawMessage  `json:"shows"`
		} `json:"cases"`
		Reveal []struct {
			Rows  []map[string]any `json:"rows"`
			At    int              `json:"at"`
			Opens []any            `json:"opens"`
		} `json:"reveal"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Cases) == 0 || len(file.Reveal) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range file.Cases {
		var want any
		must(t, json.Unmarshal(c.Shows, &want))
		if got := string(mustJSON(t, ShownTree(treeRows(c.Rows), c.Open))); got != string(mustJSON(t, want)) {
			t.Errorf("%s: %s, want %s", c.Name, got, mustJSON(t, want))
		}
	}
	for _, c := range file.Reveal {
		if got := string(mustJSON(t, Above(treeRows(c.Rows), c.At))); got != string(mustJSON(t, c.Opens)) {
			t.Errorf("reveal %d: %s, want %s", c.At, got, mustJSON(t, c.Opens))
		}
	}
}
