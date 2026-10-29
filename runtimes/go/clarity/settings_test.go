// Where an application keeps its settings, as every runtime finds it
// (conformance/settings.json, `file`), and keeping them.

package clarity

import (
	"encoding/json"
	"os"
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
