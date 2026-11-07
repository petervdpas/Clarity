package clarity

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

// Text functions alike in every runtime (conformance/texts.json).
func TestTextFunctionsReadAsEveryRuntimeReadsThem(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "texts.json"))
	if err != nil {
		t.Skip("conformance/texts.json is not beside the runtime")
	}
	var cases struct {
		Cases []struct {
			Function string `json:"function"`
			Args     []any  `json:"args"`
			Result   any    `json:"result"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Cases {
		s := func(i int) string { return c.Args[i].(string) }
		n := func(i int) int64 { return int64(c.Args[i].(float64)) }
		var got any
		switch c.Function {
		case "words":
			got = TextWords(s(0))
		case "split":
			got = TextSplit(s(0), s(1))
		case "trim":
			got = TextTrim(s(0))
		case "lower":
			got = TextLower(s(0))
		case "upper":
			got = TextUpper(s(0))
		case "replace":
			got = TextReplace(s(0), s(1), s(2))
		case "startsWith":
			got = TextStartsWith(s(0), s(1))
		case "endsWith":
			got = TextEndsWith(s(0), s(1))
		case "slice":
			got = TextSlice(s(0), n(1), n(2))
		case "indexOf":
			got = TextIndexOf(s(0), s(1))
		}
		// Through JSON, so a list, a number and a text compare as the case file has them.
		round, _ := json.Marshal(got)
		var back any
		_ = json.Unmarshal(round, &back)
		if !reflect.DeepEqual(back, c.Result) {
			t.Errorf("%s%v: %v, not %v", c.Function, c.Args, back, c.Result)
		}
	}
}
