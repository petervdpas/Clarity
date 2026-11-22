package clarity

import (
	"encoding/json"
	"os"
	"testing"
)

// `contains` outside a query, as every stack does it (conformance/contains.json).
func TestTextContainsAPartCaseIgnoredAsEveryStackDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/contains.json")
	must(t, err)
	var file struct {
		Cases []struct {
			Text     string `json:"text"`
			Part     string `json:"part"`
			Contains bool   `json:"contains"`
		} `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	for _, c := range file.Cases {
		if got := TextContains(c.Text, c.Part); got != c.Contains {
			t.Errorf("%q contains %q: %v, want %v", c.Text, c.Part, got, c.Contains)
		}
	}
}
