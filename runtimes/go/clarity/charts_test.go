// A chart's groups (U1c) against the cases every runtime shares
// (conformance/charts.json, `groups`). The axis and the slices' colors are
// how a chart is drawn, the renderer's.

package clarity

import (
	"encoding/json"
	"os"
	"testing"
)

func TestAChartGroupsItsRowsAsEveryRuntimeGroupsThem(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/charts.json")
	must(t, err)
	var file struct {
		Groups []struct {
			Name    string          `json:"name"`
			By      string          `json:"by"`
			Order   []any           `json:"order"`
			Measure string          `json:"measure"`
			Rows    []ChartRow      `json:"rows"`
			Gives   json.RawMessage `json:"gives"`
		} `json:"groups"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Groups) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range file.Groups {
		var want any
		must(t, json.Unmarshal(c.Gives, &want))
		if got := string(mustJSON(t, ChartGroups(c.Rows, c.By, c.Measure, c.Order))); got != string(mustJSON(t, want)) {
			t.Errorf("%s: %s, want %s", c.Name, got, mustJSON(t, want))
		}
	}
}
