package clarity

// A backup before migration steps change a database (docs/BACKUP-PLAN.md),
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

func TestADatabaseWithASchemaIsBackedUpBeforeNewSteps(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "shop.db")
	first := []string{"CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT '')"}
	second := []string{"ALTER TABLE item ADD COLUMN price INTEGER NOT NULL DEFAULT 0"}
	db, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := db.Migrate([][]string{first}); err != nil {
		t.Fatal(err)
	}
	if err := db.Batch("", "INSERT INTO item (name) VALUES ('kept')"); err != nil {
		t.Fatal(err)
	}
	db.Close()
	if _, err := os.Stat(filepath.Join(dir, "backups")); !os.IsNotExist(err) {
		t.Fatal("a new database needs no backup")
	}
	db, _ = Open(path)
	if n, err := db.Migrate([][]string{first, second}); err != nil || n != 1 {
		t.Fatal(n, err)
	}
	db.Close()
	entries, _ := os.ReadDir(filepath.Join(dir, "backups"))
	if len(entries) != 1 || !strings.HasPrefix(entries[0].Name(), "shop-") || !strings.HasSuffix(entries[0].Name(), "-before-migration-0002.db") {
		t.Fatalf("backups: %v", entries)
	}
	copy, _ := Open(filepath.Join(dir, "backups", entries[0].Name()))
	defer copy.Close()
	if v, _ := copy.SchemaVersion(); v != 1 {
		t.Errorf("the backup is at step %d, wanted 1", v)
	}
	if err := copy.Batch("", "SELECT price FROM item"); err == nil {
		t.Error("step 2 is not in the backup")
	}
}
