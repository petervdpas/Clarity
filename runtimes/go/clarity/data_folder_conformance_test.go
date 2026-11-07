package clarity

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Every runtime places an SQLite database alike (conformance/data-folder.json).
func TestADatabaseOpensWhereEveryRuntimeOpensIt(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "data-folder.json"))
	if err != nil {
		t.Skip("conformance/data-folder.json is not beside the runtime")
	}
	var cases struct {
		Cases []struct {
			Os       string            `json:"os"`
			Env      map[string]string `json:"env"`
			Location string            `json:"location"`
			File     string            `json:"file"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Cases {
		goos := c.Os
		if goos == "macos" {
			goos = "darwin"
		}
		got := DataFolderSqliteIn(goos, c.Location, func(name string) string { return c.Env[name] })
		if strings.ReplaceAll(got, "\\", "/") != c.File {
			t.Errorf("%s %v %s: %q, not %q", c.Os, c.Env, c.Location, got, c.File)
		}
	}
}
