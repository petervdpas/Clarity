package clarity

// A backup before migration steps change a database (docs/ledger/backups.md),
// named and pruned as every stack does it (conformance/backups.json).

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

func TestBackupsAreNamedAndPrunedAsEveryStackDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/backups.json")
	if err != nil {
		t.Fatal(err)
	}
	var c struct {
		Stamps []struct {
			Seconds int64
			Stamp   string
		}
		Names []struct {
			Database string
			Seconds  int64
			Step     int
			File     string
		}
		Kept  int
		Prune struct {
			Database string
			Files    []string
			Left     []string
		}
	}
	if err := json.Unmarshal(text, &c); err != nil {
		t.Fatal(err)
	}
	for _, s := range c.Stamps {
		if got := BackupStamp(s.Seconds); got != s.Stamp {
			t.Errorf("stamp of %d: %s, wanted %s", s.Seconds, got, s.Stamp)
		}
	}
	for _, n := range c.Names {
		if got := BackupFileName(n.Database, n.Seconds, n.Step); got != n.File {
			t.Errorf("name: %s, wanted %s", got, n.File)
		}
	}
	if c.Kept != KeptAutomatic {
		t.Errorf("kept %d, wanted %d", KeptAutomatic, c.Kept)
	}
	dir := t.TempDir()
	os.MkdirAll(filepath.Join(dir, "backups"), 0o755)
	for _, f := range c.Prune.Files {
		os.WriteFile(filepath.Join(dir, "backups", f), nil, 0o644)
	}
	PruneBackups(filepath.Join(dir, c.Prune.Database+".db"))
	entries, _ := os.ReadDir(filepath.Join(dir, "backups"))
	var left []string
	for _, e := range entries {
		left = append(left, e.Name())
	}
	sort.Strings(left)
	if strings.Join(left, "\n") != strings.Join(c.Prune.Left, "\n") {
		t.Errorf("left after the prune:\n%s\nwanted:\n%s", strings.Join(left, "\n"), strings.Join(c.Prune.Left, "\n"))
	}
}
