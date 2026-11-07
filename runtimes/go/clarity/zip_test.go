package clarity

import (
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// zip(paths, texts) alike in every runtime (conformance/zip.json).
func TestAZipIsWrittenAsEveryRuntimeWritesIt(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "zip.json"))
	if err != nil {
		t.Skip("conformance/zip.json is not beside the runtime")
	}
	var doc struct {
		Cases []struct {
			Name  string   `json:"name"`
			Paths []string `json:"paths"`
			Texts []string `json:"texts"`
			Hex   string   `json:"hex"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &doc); err != nil {
		t.Fatal(err)
	}
	for _, c := range doc.Cases {
		if got := hex.EncodeToString(Zip(c.Paths, c.Texts)); got != c.Hex {
			t.Errorf("%s: %s, want %s", c.Name, got, c.Hex)
		}
	}
}
