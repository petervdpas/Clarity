// Where an application keeps its settings, as every runtime finds it
// (conformance/settings.json, `file`), and keeping them.

package clarity

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestTheSettingsFileIsWhereEveryRuntimePutsIt(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/settings.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases struct {
		File []struct {
			OS      string            `json:"os"`
			Env     map[string]string `json:"env"`
			Program string            `json:"program"`
			File    *string           `json:"file"`
		} `json:"file"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.File {
		found, ok := SettingsFileIn(c.OS, func(name string) string { return c.Env[name] }, c.Program)
		if c.File == nil {
			if ok {
				t.Errorf("%v: found %q, wanted none", c, found)
			}
		} else if !ok || found != *c.File {
			t.Errorf("%v: found %q, wanted %q", c, found, *c.File)
		}
	}
}

func TestSettingsKeptAreReadBack(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", dir)
	// Windows keeps settings under APPDATA, the others under XDG_CONFIG_HOME.
	t.Setenv("APPDATA", dir)
	program := "clarity-settings-test"
	if len(ReadSettings(program)) != 0 {
		t.Fatal("nothing kept yet")
	}
	values := map[string]any{"pageSize": float64(20), "language": "nl"}
	if err := KeepSettings(program, values); err != nil {
		t.Fatal(err)
	}
	if got := ReadSettings(program); !reflect.DeepEqual(got, values) {
		t.Errorf("read back %v, wanted %v", got, values)
	}
}

func TestAFolderSettingIsWhereEveryRuntimePutsAStore(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/settings.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases struct {
		Folders []struct {
			OS             string `json:"os"`
			SettingsFolder string `json:"settingsFolder"`
			Store          string `json:"store"`
			Kept           any    `json:"kept"`
			Default        string `json:"default"`
			Folder         string `json:"folder"`
		} `json:"folders"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Folders {
		if found := StoreFolderIn(c.OS, c.SettingsFolder, c.Store, c.Kept, c.Default); found != c.Folder {
			t.Errorf("%v: found %q, wanted %q", c, found, c.Folder)
		}
	}
}

func TestAStoreReadsTheFolderAHandlerSetAsOnEveryStack(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/settings.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases struct {
		Now []struct {
			Held    map[string]any `json:"held"`
			Kept    map[string]any `json:"kept"`
			Setting string         `json:"setting"`
			Value   string         `json:"value"`
		} `json:"now"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	if len(cases.Now) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range cases.Now {
		if found := SettingNow(c.Held, c.Kept, c.Setting); found != c.Value {
			t.Errorf("%v: found %q, wanted %q", c, found, c.Value)
		}
	}
}

// A file store whose folder is a setting finds its files in the folder a
// handler set, before the settings are kept.
func TestAStoreFollowsTheFolderAHandlerSet(t *testing.T) {
	root := t.TempDir()
	old, now := filepath.Join(root, "old"), filepath.Join(root, "new")
	if err := os.MkdirAll(now, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(now, "a.md"), []byte("new"), 0o644); err != nil {
		t.Fatal(err)
	}
	db, err := Memory()
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if err := db.AddStoreSetting("ledger", "clarity-held-test", "ledgerFolder", old); err != nil {
		t.Fatal(err)
	}
	count := func() int {
		files, err := db.Files("ledger")
		if err != nil {
			t.Fatal(err)
		}
		return len(files)
	}
	if count() != 0 {
		t.Fatal("the old folder is empty")
	}
	db.HoldSettings(map[string]any{"ledgerFolder": now})
	if count() != 1 {
		t.Fatal("the folder the handler set holds a.md")
	}
	db.HoldSetting("ledgerFolder", old)
	if count() != 0 {
		t.Fatal("the folder set last is read")
	}
}

func TestKeepingSettingsLeavesTheKeysTheProgramDoesNotKeep(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/settings.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases struct {
		Merge []struct {
			What    string         `json:"what"`
			OnFile  map[string]any `json:"onFile"`
			Kept    map[string]any `json:"kept"`
			Written map[string]any `json:"written"`
		} `json:"merge"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Merge {
		if got := MergedSettings(c.OnFile, c.Kept); !reflect.DeepEqual(got, c.Written) {
			t.Errorf("%s: %v, want %v", c.What, got, c.Written)
		}
	}
}
