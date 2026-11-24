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

// Every runtime finds a program's data folder alike (conformance/data-folder.json, folders).
func TestTheDataFolderIsWhereEveryRuntimeFindsIt(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "data-folder.json"))
	if err != nil {
		t.Skip("conformance/data-folder.json is not beside the runtime")
	}
	var cases struct {
		Folders []struct {
			Os      string            `json:"os"`
			Env     map[string]string `json:"env"`
			Program string            `json:"program"`
			Folder  string            `json:"folder"`
		} `json:"folders"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Folders {
		got := DataFolderIn(c.Os, c.Program, func(name string) string { return c.Env[name] })
		if strings.ReplaceAll(got, "\\", "/") != c.Folder {
			t.Errorf("%s %v %s: %q, not %q", c.Os, c.Env, c.Program, got, c.Folder)
		}
	}
}

// Every runtime places a file store alike, and an unset variable is said in
// Clarity's own words (conformance/data-folder.json, stores).
func TestAFileStoreIsWhereEveryRuntimePlacesIt(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "data-folder.json"))
	if err != nil {
		t.Skip("conformance/data-folder.json is not beside the runtime")
	}
	var cases struct {
		Stores []struct {
			Os       string            `json:"os"`
			Env      map[string]string `json:"env"`
			Location string            `json:"location"`
			Folder   string            `json:"folder"`
			Unset    string            `json:"unset"`
		} `json:"stores"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Stores {
		got, err := DataFolderStoreIn(c.Os, c.Location, func(name string) string { return c.Env[name] })
		if c.Unset != "" {
			said, ok := Said(err)
			if !ok || said.Key != "variableNotSet" || said.Show() != "The environment variable "+c.Unset+" must be set" {
				t.Errorf("%s %v %s: %v", c.Os, c.Env, c.Location, err)
			}
			continue
		}
		if err != nil || strings.ReplaceAll(got, "\\", "/") != c.Folder {
			t.Errorf("%s %v %s: %q %v, not %q", c.Os, c.Env, c.Location, got, err, c.Folder)
		}
	}
}
