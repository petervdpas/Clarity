// The database side of undo (DESIGN 5.29), as on the other stacks:
// everything one handler run changed is one step, with an id the screen's
// edit keeps. Undo puts the rows back as they were, with their own ids,
// cascades included; redo does it again; a handler that fails leaves
// nothing behind; an undo whose rows changed since is refused.

package clarity_test

import (
	"errors"
	"reflect"
	"testing"

	. "github.com/petervdpas/Clarity/runtimes/go/clarity"
)

type shape struct {
	ID   int64
	Name string
	X    int64
}

func (*shape) Table() string     { return "shape" }
func (*shape) Columns() []string { return []string{"name", "x"} }
func (s *shape) GetID() int64    { return s.ID }
func (s *shape) SetID(id int64)  { s.ID = id }
func (s *shape) Values() []any   { return []any{s.Name, s.X} }
func (s *shape) Load(id int64, v []any) error {
	r := NewReader(v)
	*s = shape{id, r.Text(0, "name"), r.Integer(1, "x")}
	return r.Err()
}
func (s *shape) Cascade(db *Db) error {
	notes, err := Find[note](db, Query{}.Filter(Compare("shape", Eq, s.ID)))
	if err != nil {
		return err
	}
	for i := range notes {
		if err := Delete(db, &notes[i]); err != nil {
			return err
		}
	}
	return nil
}

type note struct {
	ID    int64
	Shape *int64
	Text  string
}

func (*note) Table() string     { return "note" }
func (*note) Columns() []string { return []string{"shape", "text"} }
func (n *note) GetID() int64    { return n.ID }
func (n *note) SetID(id int64)  { n.ID = id }
func (n *note) Values() []any   { return []any{n.Shape, n.Text} }
func (n *note) Load(id int64, v []any) error {
	r := NewReader(v)
	*n = note{id, r.Reference(0, "shape"), r.Text(1, "text")}
	return r.Err()
}

func drawn(t *testing.T) *Db {
	db, err := Memory()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { db.Close() })
	if _, err := db.Migrate([][]string{{
		"CREATE TABLE shape (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, x INTEGER NOT NULL)",
		"CREATE TABLE note (id INTEGER PRIMARY KEY AUTOINCREMENT, shape INTEGER REFERENCES shape(id), text TEXT NOT NULL)",
	}}); err != nil {
		t.Fatal(err)
	}
	return db
}

func shapes(t *testing.T, db *Db) []shape {
	all, err := All[shape](db)
	if err != nil {
		t.Fatal(err)
	}
	return all
}

func newShape(t *testing.T, db *Db, name string, x int64) *shape {
	s := &shape{Name: name, X: x}
	if err := Save(db, s); err != nil {
		t.Fatal(err)
	}
	return s
}

func TestAStepOfSavesIsUndoneAndDoneAgainWithTheSameIds(t *testing.T) {
	db := drawn(t)
	var a, b *shape
	if err := db.Step(func() error { a = newShape(t, db, "A", 10); return nil }); err != nil {
		t.Fatal(err)
	}
	made := db.LastStep()
	if err := db.Step(func() error {
		a.X = 300
		if err := Save(db, a); err != nil {
			return err
		}
		b = newShape(t, db, "B", 20)
		return nil
	}); err != nil {
		t.Fatal(err)
	}
	moved := db.LastStep()
	if done, err := db.Undo(moved); !done || err != nil {
		t.Fatal(done, err)
	}
	if got := shapes(t, db); !reflect.DeepEqual(got, []shape{{a.ID, "A", 10}}) {
		t.Fatal(got)
	}
	if done, _ := db.Undo(moved); done {
		t.Fatal("undone twice")
	}
	if done, err := db.Redo(moved); !done || err != nil {
		t.Fatal(done, err)
	}
	if got := shapes(t, db); !reflect.DeepEqual(got, []shape{*a, *b}) {
		t.Fatal(got)
	}
	db.Undo(moved)
	db.Undo(made)
	if got := shapes(t, db); len(got) != 0 {
		t.Fatal(got)
	}
}

func TestADeleteWithItsCascadeComesBackWhole(t *testing.T) {
	db := drawn(t)
	a := newShape(t, db, "A", 10)
	n := &note{Shape: &a.ID, Text: "keep"}
	if err := Save(db, n); err != nil {
		t.Fatal(err)
	}
	if err := db.Step(func() error { return Delete(db, a) }); err != nil {
		t.Fatal(err)
	}
	if done, err := db.Undo(db.LastStep()); !done || err != nil {
		t.Fatal(done, err)
	}
	notes, _ := All[note](db)
	if got := shapes(t, db); !reflect.DeepEqual(got, []shape{*a}) || len(notes) != 1 || notes[0].ID != n.ID {
		t.Fatal(got, notes)
	}
}

func TestAHandlerThatFailsLeavesNothingAndIsNoStep(t *testing.T) {
	db := drawn(t)
	err := db.Step(func() error {
		newShape(t, db, "A", 10)
		return errors.New("no")
	})
	if err == nil || len(shapes(t, db)) != 0 || db.LastStep() != 0 {
		t.Fatal(err, shapes(t, db), db.LastStep())
	}
	db.Step(func() error { _, err := All[shape](db); return err })
	if db.LastStep() != 0 {
		t.Fatal("a step that changed nothing")
	}
}

func TestAnUndoWhoseRowsChangedSinceIsRefused(t *testing.T) {
	db := drawn(t)
	a := newShape(t, db, "A", 10)
	db.Step(func() error { a.X = 50; return Save(db, a) })
	if err := db.Batch("", "UPDATE shape SET x = 99"); err != nil {
		t.Fatal(err)
	}
	var refused *RefusedError
	if _, err := db.Undo(db.LastStep()); !errors.As(err, &refused) || refused.Text.Key != "undoChangedSince" {
		t.Fatal(err)
	}
	if got := shapes(t, db); got[0].X != 99 {
		t.Fatal(got)
	}
}
