// Lists of values, against the cases every stack's runtime shares
// (conformance/lists.json).

package clarity

import (
	"encoding/json"
	"os"
	"slices"
	"testing"
)

type listCases struct {
	Items []struct {
		List  []int64 `json:"list"`
		At    int64   `json:"at"`
		Gives *int64  `json:"gives"`
		Fails string  `json:"fails"`
	} `json:"items"`
}

func readListCases(t *testing.T) listCases {
	text, err := os.ReadFile("../../../conformance/lists.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases listCases
	if err := json.Unmarshal(text, &cases); err != nil || len(cases.Items) == 0 {
		t.Fatalf("no cases: %v", err)
	}
	return cases
}

func TestAnItemIsReadByItsPositionFrom1(t *testing.T) {
	for _, c := range readListCases(t).Items {
		got, err := At(c.List, c.At)
		switch {
		case c.Gives != nil && (err != nil || got != *c.Gives):
			t.Errorf("%v at %d: %d, %v", c.List, c.At, got, err)
		case c.Gives == nil && (err == nil || err.Error() != c.Fails):
			t.Errorf("%v at %d: %v, not %q", c.List, c.At, err, c.Fails)
		}
	}
}

func TestAnItemIsSetByItsPositionFrom1(t *testing.T) {
	list := []int64{10, 20, 30}
	if err := SetAt(list, 2, 99); err != nil || !slices.Equal(list, []int64{10, 99, 30}) {
		t.Fatalf("%v %v", list, err)
	}
	if err := SetAt(list, 4, 1); err == nil || err.Error() != "there is no item 4 in a list of 3" {
		t.Fatalf("%v", err)
	}
}
