// A text by its key, filled in the backend in the language the screen
// picked, as every runtime does (conformance/translations.json, `code`).

package clarity

import (
	"encoding/json"
	"os"
	"testing"
)

func TestCodeTextsAreTranslatedAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/translations.json")
	if err != nil {
		t.Fatal(err)
	}
	var all struct {
		Code struct {
			Model json.RawMessage `json:"model"`
			Cases []struct {
				What     string            `json:"what"`
				Language *string           `json:"language"`
				Key      string            `json:"key"`
				Values   map[string]string `json:"values"`
				Gives    string            `json:"gives"`
			} `json:"cases"`
		} `json:"code"`
	}
	if err := json.Unmarshal(text, &all); err != nil {
		t.Fatal(err)
	}
	LoadTranslations(all.Code.Model)
	defer SetLanguage(nil)
	if len(all.Code.Cases) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range all.Code.Cases {
		SetLanguage(c.Language)
		shown := KeyText(c.Key)
		for path, value := range c.Values {
			shown = shown.With(path, SlotText(value))
		}
		if got := shown.Show(); got != c.Gives {
			t.Errorf("%s: %q, want %q", c.What, got, c.Gives)
		}
	}
}
