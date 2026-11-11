// The database side of undo (DESIGN 5.29): the changes of each handler run
// that changed something, by the step's id. The screen keeps the edits on
// its bus, each with the id of the step its handler made, and asks for that
// step to be undone or done again. The last UndoSteps are kept.

package clarity

import (
	"fmt"
	"reflect"
	"strings"
)

// UndoSteps is how many undo steps are kept: the oldest goes when a newer
// one comes, so a long session does not keep every row it ever changed.
const UndoSteps = 100

// UndoChangedSinceText is why an undo is refused: what it would take back
// was changed since, in Clarity's own words with their English.
func UndoChangedSinceText() Text {
	return KeyedText("undoChangedSince", "This was changed since, so it cannot be taken back")
}

// UndoTooOldText is why an undo is refused when its step is older than the ones kept.
func UndoTooOldText() Text {
	return KeyedText("undoTooOld", "This was done too long ago to take back")
}

// change is a row of a table as it was before a change and as the change
// left it; nil where it was not there. A versioned row (version) carries
// its version in each state, which undo compares instead of the whole row.
type change struct {
	source        string
	table         string
	key           string
	columns       []string
	version       string
	id            int64
	before        []any
	after         []any
	beforeVersion *int64
	afterVersion  *int64
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
	// held is the sources the running step holds a transaction in: what one
	// handler changes is kept or left whole.
	held []string
}

func (h *history) record(c change) {
	if h.recording != nil {
		*h.recording = append(*h.recording, c)
	}
}

// Step runs a handler as one step, which undo takes back whole: its id,
// when it changed anything, is LastStep. Everything it changes is one
// transaction in each source it changes: kept when it ends, left whole when
// it fails, so it leaves nothing behind. A step inside a step is part of it.
func (db *Db) Step(run func() error) error {
	// An application without a database has nothing to undo.
	if db == nil || db.history.recording != nil {
		return run()
	}
	changes := []change{}
	db.history.recording = &changes
	err := run()
	db.history.recording = nil
	held := db.history.held
	db.history.held = nil
	if kept := db.release(held, err == nil); err == nil {
		err = kept
	}
	if err == nil && len(changes) > 0 {
		if db.history.steps == nil {
			db.history.steps = map[int64]*step{}
		}
		db.history.last++
		db.history.steps[db.history.last] = &step{changes: changes}
		delete(db.history.steps, db.history.last-UndoSteps)
	}
	return err
}

// enlist makes the running step hold a transaction in source, from its
// first change there; outside a step, nothing.
func (db *Db) enlist(source string) error {
	if db.history.recording == nil {
		return nil
	}
	s, err := db.source(source)
	if err != nil {
		return err
	}
	if contains(db.history.held, s.name) {
		return nil
	}
	if s.depth == 0 {
		if err := s.driver.batch(s.driver.dialect().begin()); err != nil {
			return err
		}
	}
	s.depth++
	db.history.held = append(db.history.held, s.name)
	return nil
}

// release ends the transactions a step held: kept or rolled back. When one
// cannot be kept, the ones not kept yet are rolled back too.
func (db *Db) release(held []string, keep bool) error {
	var failed error
	for _, name := range held {
		s, err := db.source(name)
		if err != nil {
			continue
		}
		s.depth--
		if s.depth > 0 {
			continue
		}
		d := s.driver.dialect()
		if keep {
			if err := s.driver.batch(d.commit()); err != nil {
				s.driver.batch(d.rollback())
				keep, failed = false, err
			}
		} else {
			s.driver.batch(d.rollback())
		}
	}
	return failed
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
// when it is undone already.
func (db *Db) Undo(id int64) (bool, error) { return db.ReplaySteps([]int64{id}, true) }

// Redo does step id again after an undo; false when it is not undone.
func (db *Db) Redo(id int64) (bool, error) { return db.ReplaySteps([]int64{id}, false) }

// ReplaySteps takes back (back) or does again the steps of one edit, in the
// order given, as one: all of them or, refused, none. A step older than the
// ones kept is refused (UndoTooOldText); false when none of them is in the
// state to be replayed.
func (db *Db) ReplaySteps(ids []int64, back bool) (bool, error) {
	if db == nil {
		return false, nil
	}
	var taken []*step
	for _, id := range ids {
		s, ok := db.history.steps[id]
		switch {
		// A step never made is nothing to replay; one made and no longer kept is refused.
		case !ok && id != 0 && id <= db.history.last:
			return false, Refuse(UndoTooOldText())
		case ok && s.undone != back:
			taken = append(taken, s)
		}
	}
	if len(taken) == 0 {
		return false, nil
	}
	if err := db.replay(taken, back); err != nil {
		return false, err
	}
	for _, s := range taken {
		s.undone = back
	}
	return true, nil
}

// replay puts the rows of steps back or forward, after checking that each
// row is still as the steps left it (by its version, or the whole row where
// there is none), all in one transaction per source, checks included. Each
// row's new version is noted where the next replay checks it.
func (db *Db) replay(steps []*step, back bool) error {
	var order []*change
	var sources []string
	for _, s := range steps {
		for i := range s.changes {
			at := i
			if back {
				at = len(s.changes) - 1 - i
			}
			c := &s.changes[at]
			order = append(order, c)
			if !contains(sources, c.source) {
				sources = append(sources, c.source)
			}
		}
	}
	return db.within(sources, func() error {
		// Each row as the steps left it (its last change undone, its first redone) must still be so.
		seen := map[string]bool{}
		for _, c := range order {
			at := fmt.Sprint(c.source, "\x00", c.table, "\x00", c.id)
			if seen[at] {
				continue
			}
			seen[at] = true
			row, version, err := db.state(*c, c.id)
			if err != nil {
				return err
			}
			var unchanged bool
			if c.version != "" {
				expected := c.beforeVersion
				if back {
					expected = c.afterVersion
				}
				unchanged = sameVersion(version, expected)
			} else {
				expected := c.before
				if back {
					expected = c.after
				}
				unchanged = sameRow(row, expected)
			}
			if !unchanged {
				return Refuse(UndoChangedSinceText())
			}
		}
		for _, c := range order {
			values, version := c.after, c.afterVersion
			if back {
				values, version = c.before, c.beforeVersion
			}
			if err := db.put(*c, values, version); err != nil {
				return err
			}
		}
		// The version each row has now is what the next replay finds: noted on its last change in this order.
		noted := map[string]bool{}
		for i := len(order) - 1; i >= 0; i-- {
			c := order[i]
			at := fmt.Sprint(c.source, "\x00", c.table, "\x00", c.id)
			if c.version == "" || noted[at] {
				continue
			}
			noted[at] = true
			_, version, err := db.state(*c, c.id)
			if err != nil {
				return err
			}
			if back {
				c.beforeVersion = version
			} else {
				c.afterVersion = version
			}
		}
		return nil
	})
}

func sameVersion(a, b *int64) bool {
	if a == nil || b == nil {
		return a == b
	}
	return *a == *b
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

// state is a row's raw values (the columns of c) and its version (by
// c.version), or nil when it is not there.
func (db *Db) state(c change, id int64) ([]any, *int64, error) {
	s, err := db.source(c.source)
	if err != nil {
		return nil, nil, err
	}
	d := s.driver.dialect()
	names := []string{d.Quote(c.key)}
	for _, col := range c.columns {
		names = append(names, d.Quote(col))
	}
	if c.version != "" {
		names = append(names, d.Quote(c.version))
	}
	rows, err := s.driver.rows("SELECT "+strings.Join(names, ", ")+" FROM "+d.Quote(c.table)+" WHERE "+d.Quote(c.key)+" = "+d.Mark(1), []any{id})
	if err != nil || len(rows) == 0 {
		return nil, nil, err
	}
	row := rows[0][1:]
	var version *int64
	if c.version != "" {
		n, _ := integer(row[len(row)-1])
		version = &n
		row = row[:len(row)-1]
	}
	return append([]any{}, row...), version, nil
}

// put makes a row so: gone, or with these values and its own id. A
// versioned row put back gets a version past both its current one and
// version, the one of the state it returns to, so a copy read before is
// refused.
func (db *Db) put(c change, values []any, version *int64) error {
	now, current, err := db.state(c, c.id)
	if err != nil {
		return err
	}
	var next int64 = 1
	if current != nil && *current+1 > next {
		next = *current + 1
	}
	if version != nil && *version+1 > next {
		next = *version + 1
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
		var sets []string
		params := append([]any{}, values...)
		for i, col := range c.columns {
			sets = append(sets, d.Quote(col)+" = "+d.Mark(i+1))
		}
		if c.version != "" {
			params = append(params, next)
			sets = append(sets, d.Quote(c.version)+" = "+d.Mark(len(params)))
		}
		if len(sets) == 0 {
			return nil
		}
		params = append(params, c.id)
		_, err = s.driver.execute("UPDATE "+table+" SET "+strings.Join(sets, ", ")+" WHERE "+key+" = "+d.Mark(len(params)), params)
	default:
		names := []string{key}
		params := append([]any{c.id}, values...)
		for _, col := range c.columns {
			names = append(names, d.Quote(col))
		}
		if c.version != "" {
			names = append(names, d.Quote(c.version))
			params = append(params, next)
		}
		marks := make([]string, len(params))
		for i := range params {
			marks[i] = d.Mark(i + 1)
		}
		insert := "INSERT INTO " + table + " (" + strings.Join(names, ", ") + ") VALUES (" + strings.Join(marks, ", ") + ")"
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
