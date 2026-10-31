// The dropdown's options (U1c) against the cases every runtime shares
// (conformance/dropdown-options.json).

package clarity

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

func TestADropdownOffersWhatEveryRuntimeOffers(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/dropdown-options.json")
	must(t, err)
	var file struct {
		Suggestions []struct {
			Values []string `json:"values"`
			Typed  string   `json:"typed"`
			Gives  []string `json:"gives"`
		} `json:"suggestions"`
		Search struct {
			Options []RefOption `json:"options"`
			Cases   []struct {
				Typed string  `json:"typed"`
				Gives []int64 `json:"gives"`
			} `json:"cases"`
		} `json:"search"`
	}
	must(t, json.Unmarshal(text, &file))
	if len(file.Suggestions) == 0 || len(file.Search.Cases) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range file.Suggestions {
		if got := Suggestions(c.Values, c.Typed); !reflect.DeepEqual(got, c.Gives) {
			t.Errorf("suggestions %q: %v, want %v", c.Typed, got, c.Gives)
		}
	}
	for _, c := range file.Search.Cases {
		ids := []int64{}
		for _, o := range Searched(file.Search.Options, c.Typed, nil) {
			ids = append(ids, o.ID)
		}
		if !reflect.DeepEqual(ids, c.Gives) {
			t.Errorf("search %q: %v, want %v", c.Typed, ids, c.Gives)
		}
	}
	// Only the fields the dropdown shows are searched.
	if got := Searched(file.Search.Options, "gent", []string{"name"}); len(got) != 0 {
		t.Errorf("searched a field it does not show: %v", got)
	}
}
