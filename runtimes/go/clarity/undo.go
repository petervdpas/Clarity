// The database side of undo (DESIGN 5.29): the changes of each handler run
// that changed something, by the step's id. The screen keeps the edits on
// its bus, each with the id of the step its handler made, and asks for that
// step to be undone or done again. Kept until the application closes.

package clarity

import (
	"fmt"
	"reflect"
	"strings"
)

// change is a row of a table as it was before a change and as the change
// left it; nil where it was not there.
type change struct {
	source  string
	table   string
	key     string
	columns []string
	id      int64
	before  []any
	after   []any
}

type step struct {
	changes []change
	undone  bool
}

type history struct {
	// recording is the changes of the step that runs; nil outside a step.
	recording *[]change
	steps     map[int64]*step
	last      int64
}

func (h *history) record(c change) {
	if h.recording != nil {
		*h.recording = append(*h.recording, c)
	}
}

// Step runs a handler as one step, which undo takes back whole: its id,
// when it changed anything, is LastStep. When it fails, what it changed is
// put back at once, so it leaves nothing behind. A step inside a step is
// part of it.
func (db *Db) Step(run func() error) error {
	// An application without a database has nothing to undo.
	if db == nil || db.history.recording != nil {
		return run()
	}
	changes := []change{}
	db.history.recording = &changes
	err := run()
	db.history.recording = nil
	switch {
	case err != nil:
		for i := len(changes) - 1; i >= 0; i-- {
			db.put(changes[i], changes[i].before)
		}
	case len(changes) > 0:
		if db.history.steps == nil {
			db.history.steps = map[int64]*step{}
		}
		db.history.last++
		db.history.steps[db.history.last] = &step{changes: changes}
	}
	return err
}

// LastStep is the id of the last step a handler made, to undo it later by; 0 before any.
func (db *Db) LastStep() int64 {
	if db == nil {
		return 0
	}
	return db.history.last
}

// Undo takes back step id: every row it changed as it was, with its own
// id. Refused, changing nothing, when one of its rows changed since; false
// when there is no such step, or it is undone already.
func (db *Db) Undo(id int64) (bool, error) { return db.replayStep(id, true) }

// Redo does step id again after an undo; false when it is not undone.
func (db *Db) Redo(id int64) (bool, error) { return db.replayStep(id, false) }

func (db *Db) replayStep(id int64, back bool) (bool, error) {
	if db == nil {
		return false, nil
	}
	s, ok := db.history.steps[id]
	if !ok || s.undone == back {
		return false, nil
	}
	if err := db.replay(s.changes, back); err != nil {
		return false, err
	}
	s.undone = back
	return true, nil
}

// replay puts a step's rows back or forward, after checking that each row
// is still as the step left it, in one transaction per source.
func (db *Db) replay(changes []change, back bool) error {
	order := make([]change, len(changes))
	copy(order, changes)
	if back {
		for i, j := 0, len(order)-1; i < j; i, j = i+1, j-1 {
			order[i], order[j] = order[j], order[i]
		}
	}
	// Each row as the step left it (its last change undone, its first redone) must still be so.
	seen := map[string]bool{}
	var sources []string
	for _, c := range order {
		at := fmt.Sprint(c.source, "\x00", c.table, "\x00", c.id)
		if !seen[at] {
			seen[at] = true
			expected := c.before
			if back {
				expected = c.after
			}
			now, err := db.row(c.source, c.table, c.key, c.columns, c.id)
			if err != nil {
				return err
			}
			if !sameRow(now, expected) {
				return Refuse(KeyText("undoChangedSince"))
			}
		}
		if !contains(sources, c.source) {
			sources = append(sources, c.source)
		}
	}
	return db.within(sources, func() error {
		for _, c := range order {
			values := c.after
			if back {
				values = c.before
			}
			if err := db.put(c, values); err != nil {
				return err
			}
		}
		return nil
	})
}

func contains(list []string, s string) bool {
	for _, x := range list {
		if x == s {
			return true
		}
	}
	return false
}

func sameRow(a, b []any) bool {
	if (a == nil) != (b == nil) {
		return false
	}
	return reflect.DeepEqual(a, b)
}

// within runs run in a transaction of each of sources.
func (db *Db) within(sources []string, run func() error) error {
	if len(sources) == 0 {
		return run()
	}
	return db.atomic(sources[0], func() error { return db.within(sources[1:], run) })
}

// row is a row's raw values (its columns), or nil when it is not there.
func (db *Db) row(source, table, key string, columns []string, id int64) ([]any, error) {
	s, err := db.source(source)
	if err != nil {
		return nil, err
	}
	d := s.driver.dialect()
	names := d.Quote(key)
	if len(columns) > 0 {
		quoted := make([]string, len(columns))
		for i, c := range columns {
			quoted[i] = d.Quote(c)
		}
		names = strings.Join(quoted, ", ")
	}
	rows, err := s.driver.rows("SELECT "+names+" FROM "+d.Quote(table)+" WHERE "+d.Quote(key)+" = "+d.Mark(1), []any{id})
	if err != nil || len(rows) == 0 {
		return nil, err
	}
	if len(columns) == 0 {
		return []any{}, nil
	}
	return rows[0], nil
}

// put makes a row so: gone, or with these values and its own id.
func (db *Db) put(c change, values []any) error {
	now, err := db.row(c.source, c.table, c.key, c.columns, c.id)
	if err != nil {
		return err
	}
	s, err := db.source(c.source)
	if err != nil {
		return err
	}
	d := s.driver.dialect()
	table, key := d.Quote(c.table), d.Quote(c.key)
	switch {
	case values == nil && now != nil:
		_, err = s.driver.execute("DELETE FROM "+table+" WHERE "+key+" = "+d.Mark(1), []any{c.id})
	case values == nil:
	case now != nil:
		if len(c.columns) == 0 {
			return nil
		}
		sets := make([]string, len(c.columns))
		for i, col := range c.columns {
			sets[i] = d.Quote(col) + " = " + d.Mark(i+1)
		}
		_, err = s.driver.execute("UPDATE "+table+" SET "+strings.Join(sets, ", ")+" WHERE "+key+" = "+d.Mark(len(c.columns)+1), append(append([]any{}, values...), c.id))
	default:
		names := []string{key}
		marks := []string{d.Mark(1)}
		for i, col := range c.columns {
			names = append(names, d.Quote(col))
			marks = append(marks, d.Mark(i+2))
		}
		insert := "INSERT INTO " + table + " (" + strings.Join(names, ", ") + ") VALUES (" + strings.Join(marks, ", ") + ")"
		params := append([]any{c.id}, values...)
		// SQL Server takes a key of its own counting only when asked to.
		if d == Mssql {
			if err := s.driver.batch("SET IDENTITY_INSERT " + table + " ON"); err != nil {
				return err
			}
			_, err = s.driver.execute(insert, params)
			s.driver.batch("SET IDENTITY_INSERT " + table + " OFF")
		} else {
			_, err = s.driver.execute(insert, params)
		}
	}
	return err
}
