package clarity_test

import (
	"fmt"

	. "github.com/petervdpas/Clarity/runtimes/go/clarity"
)

// applySteps sets a test's default database up with steps as clarity-migrate
// leaves it (docs/ledger/migrations.md), for the tests outside the package:
// each step's statements run, and the step is counted in clarity_schema. The
// runtime itself does not migrate; it checks (CheckSteps).
func applySteps(db *Db, source string, steps [][]string) (int, error) {
	if source != "" {
		return 0, fmt.Errorf("applySteps sets up the default database only, not %s", source)
	}
	done, err := db.SchemaVersion()
	if err != nil {
		return 0, err
	}
	// A step may build a table again that others refer to, as clarity-migrate allows.
	if dialect, _ := db.Dialect(""); dialect == Sqlite {
		if err := db.Batch("", "PRAGMA foreign_keys = OFF"); err != nil {
			return 0, err
		}
		defer db.Batch("", "PRAGMA foreign_keys = ON")
	}
	for i := done; i < len(steps); i++ {
		for _, statement := range steps[i] {
			if err := db.Batch("", statement); err != nil {
				return i - done, err
			}
		}
		if err := db.Batch("", fmt.Sprintf("INSERT INTO clarity_schema (version) VALUES (%d)", i+1)); err != nil {
			return i - done, err
		}
	}
	return max(0, len(steps)-done), nil
}
