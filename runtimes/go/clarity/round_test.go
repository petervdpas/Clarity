// round(value, places) in code, against the cases every stack's runtime
// shares (conformance/rounding.json).

package clarity

import (
	"encoding/json"
	"os"
	"testing"

	"github.com/shopspring/decimal"
)

func TestEveryRoundingCaseRoundsAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/rounding.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []struct {
			Value  string `json:"value"`
			Places int64  `json:"places"`
			Gives  string `json:"gives"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &file); err != nil || len(file.Cases) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	for _, c := range file.Cases {
		got := Round(decimal.RequireFromString(c.Value), c.Places)
		if !got.Equal(decimal.RequireFromString(c.Gives)) {
			t.Errorf("round(%s, %d) = %s, want %s", c.Value, c.Places, got, c.Gives)
		}
	}
}
