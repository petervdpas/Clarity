package clarity

// Reading JSON into records (DESIGN 5.23) against the cases every runtime
// shares (conformance/json.json): what is read is fitted to the entity.

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

func kindOf(field map[string]any, word string) JSONKind {
	kind := JSONKind{Is: word}
	if values, has := field["values"].([]any); has {
		for _, v := range values {
			kind.Values = append(kind.Values, v.(string))
		}
	}
	if item, has := field["item"].(string); has && word == "values" {
		inner := kindOf(field, item)
		kind.Item = &inner
	}
	return kind
}

func TestJSONIsFittedAsEveryRuntimeFitsIt(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/json.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []map[string]any `json:"cases"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	for _, c := range file.Cases {
		var fields []JSONField
		for _, f := range c["fields"].([]any) {
			field := f.(map[string]any)
			fields = append(fields, JSONField{Name: field["name"].(string), Kind: kindOf(field, field["kind"].(string))})
		}
		at, _ := c["at"].(string)
		many, _ := c["many"].(bool)
		got, err := FitJSON(c["json"].(string), at, many, c["entity"].(string), fields, c["base"])
		name := c["name"].(string)
		if want, isError := c["error"].(string); isError {
			if err == nil || err.Error() != want {
				t.Errorf("%s: %v, want %q", name, err, want)
			}
			continue
		}
		if err != nil {
			t.Errorf("%s: %v", name, err)
			continue
		}
		var gotValue, wantValue any
		json.Unmarshal([]byte(JSONOf(got)), &gotValue)
		json.Unmarshal([]byte(JSONOf(c["fitted"])), &wantValue)
		if !reflect.DeepEqual(gotValue, wantValue) {
			t.Errorf("%s: %s, want %s", name, JSONOf(got), JSONOf(c["fitted"]))
		}
	}
}
