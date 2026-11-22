package clarity

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

// An app checks its database's migration steps at start and does not migrate
// (docs/ledger/migrations.md): behind or ahead, it stops with one English line
// (conformance/migrate-check.json).
func TestADatabaseIsCheckedAsTheCasesSay(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "conformance", "migrate-check.json"))
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []struct {
			Source string  `json:"source"`
			Had    int     `json:"had"`
			Steps  int     `json:"steps"`
			Line   *string `json:"line"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	for _, c := range file.Cases {
		db, err := Connect(Source{Name: c.Source, Dialect: Sqlite, At: ":memory:"})
		if err != nil {
			t.Fatal(err)
		}
		steps := make([][]string, c.Had)
		for n := range steps {
			steps[n] = []string{fmt.Sprintf("CREATE TABLE t%d (id INTEGER PRIMARY KEY)", n)}
		}
		if _, err := applySteps(db, c.Source, steps); err != nil {
			t.Fatal(err)
		}
		checked := db.CheckSteps(c.Source, c.Steps)
		switch {
		case c.Line == nil && checked != nil:
			t.Errorf("%+v: %v", c, checked)
		case c.Line != nil && (checked == nil || checked.Error() != *c.Line):
			t.Errorf("%+v: %v, want %q", c, checked, *c.Line)
		}
		db.Close()
	}
}
