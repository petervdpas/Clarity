package clarity

import "fmt"

// applySteps sets a test's database up with steps as clarity-migrate leaves it
// (docs/ledger/migrations.md): each step's statements run, and the step is
// counted in clarity_schema. The runtime itself does not migrate; it checks
// (CheckSteps). It gives how many steps it ran.
func applySteps(db *Db, source string, steps [][]string) (int, error) {
	s, err := db.source(source)
	if err != nil {
		return 0, err
	}
	done, err := schemaVersion(s.driver)
	if err != nil {
		return 0, err
	}
	// A step may build a table again that others refer to, as clarity-migrate allows.
	if s.driver.dialect() == Sqlite {
		if err := s.driver.batch("PRAGMA foreign_keys = OFF"); err != nil {
			return 0, err
		}
		defer s.driver.batch("PRAGMA foreign_keys = ON")
	}
	for i := done; i < len(steps); i++ {
		for _, statement := range steps[i] {
			if err := s.driver.batch(statement); err != nil {
				return i - done, err
			}
		}
		if err := s.driver.batch(fmt.Sprintf("INSERT INTO clarity_schema (version) VALUES (%d)", i+1)); err != nil {
			return i - done, err
		}
	}
	return max(0, len(steps)-done), nil
}
