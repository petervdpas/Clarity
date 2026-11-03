// CLARITY_DATA moves every SQLite database of a program into a folder of its
// own, under its own file name: a run in the IDE or by hand (a live check on
// a copy) chooses its data without editing the generated program. The same
// as the Rust and C# runtimes.

package clarity

import (
	"os"
	"path/filepath"
	"testing"
)

func dataEnv(folder string) func(string) string {
	return func(name string) string {
		if name == DataVariable {
			return folder
		}
		return ""
	}
}

func TestClarityDataMovesAnSqliteFileIntoItsFolder(t *testing.T) {
	folder := filepath.Join(os.TempDir(), "copy")
	at := "/home/someone/noughts/data/noughts.db"
	if got := DataFolderSqlite(at, dataEnv(folder)); got != filepath.Join(folder, "noughts.db") {
		t.Fatalf("moved to %s", got)
	}
	// Unset or empty, the file is where the project put it.
	if got := DataFolderSqlite(at, dataEnv("")); got != at {
		t.Fatalf("unset gave %s", got)
	}
	// A location from the environment or a secret setting is as it says, and so is SQLite's database in memory.
	for _, kept := range []string{"env:SHOP_DB", "setting:shop/db", ":memory:"} {
		if got := DataFolderSqlite(kept, dataEnv("/copy")); got != kept {
			t.Fatalf("%s became %s", kept, got)
		}
	}
}

// The process's variable, read when a database opens.
func TestADatabaseOpensInTheFolderClarityDataNames(t *testing.T) {
	folder := t.TempDir()
	t.Setenv(DataVariable, folder)
	elsewhere := filepath.Join(folder, "elsewhere", "shop.db")
	db, err := Connect(Source{"main", Sqlite, elsewhere})
	if err != nil {
		t.Fatal(err)
	}
	db.Close()
	if _, err := os.Stat(filepath.Join(folder, "shop.db")); err != nil {
		t.Fatalf("not in the folder: %v", err)
	}
	if _, err := os.Stat(elsewhere); err == nil {
		t.Fatal("it opened where the project put it")
	}
}
