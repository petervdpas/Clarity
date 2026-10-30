// Cascading deletes: a record whose records refer to it with `cascade`
// deletes them first, each with its own cascades and data events, all in
// the transaction of its delete, as on the other stacks.

package clarity_test

import (
	"testing"

	. "github.com/petervdpas/Clarity/runtimes/go/clarity"
)

// deletedTasks is what the delete events of the tasks saw, in order.
var deletedTasks []string

type project struct {
	ID   int64
	Name string
}

func (*project) Table() string     { return "project" }
func (*project) Columns() []string { return []string{"name"} }
func (p *project) GetID() int64    { return p.ID }
func (p *project) SetID(id int64)  { p.ID = id }
func (p *project) Values() []any   { return []any{p.Name} }
func (p *project) Load(id int64, v []any) error {
	r := NewReader(v)
	*p = project{id, r.Text(0, "name")}
	return r.Err()
}

// Cascade is as generated code writes it: the tasks of the project, each deleted as any record is.
func (p *project) Cascade(db *Db) error {
	tasks, err := Find[task](db, Query{}.Filter(Compare("project", Eq, p.ID)))
	if err != nil {
		return err
	}
	for i := range tasks {
		if err := Delete(db, &tasks[i]); err != nil {
			return err
		}
	}
	return nil
}

type task struct {
	ID      int64
	Project *int64
	Title   string
}

func (*task) Table() string     { return "task" }
func (*task) Columns() []string { return []string{"project", "title"} }
func (t *task) GetID() int64    { return t.ID }
func (t *task) SetID(id int64)  { t.ID = id }
func (t *task) Values() []any   { return []any{t.Project, t.Title} }
func (t *task) Load(id int64, v []any) error {
	r := NewReader(v)
	*t = task{id, r.Reference(0, "project"), r.Text(1, "title")}
	return r.Err()
}
func (t *task) BeforeDelete(db *Db) error {
	if t.Title == "locked" {
		return Refuse(PlainText("This task is locked"))
	}
	deletedTasks = append(deletedTasks, t.Title)
	return nil
}

func planned(t *testing.T) (*Db, *project) {
	db, err := Memory()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { db.Close() })
	if _, err := db.Migrate([][]string{{
		"CREATE TABLE project (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
		"CREATE TABLE task (id INTEGER PRIMARY KEY, project INTEGER REFERENCES project(id), title TEXT NOT NULL)",
	}}); err != nil {
		t.Fatal(err)
	}
	p := &project{Name: "Model"}
	must(t, Save(db, p))
	for _, title := range []string{"Draw", "Write"} {
		must(t, Save(db, &task{Project: RefTo(p.ID), Title: title}))
	}
	return db, p
}

func TestDeletingARecordDeletesWhatCascadesFromItFirst(t *testing.T) {
	db, p := planned(t)
	deletedTasks = nil
	must(t, Delete(db, p))
	projects, _ := All[project](db)
	tasks, _ := All[task](db)
	equal(t, len(projects), 0)
	equal(t, len(tasks), 0)
	equal(t, deletedTasks, []string{"Draw", "Write"})
}

func TestARefusedChildKeepsAllOfIt(t *testing.T) {
	db, p := planned(t)
	must(t, Save(db, &task{Project: RefTo(p.ID), Title: "locked"}))
	equal(t, refusal(t, Delete(db, p)), "This task is locked")
	projects, _ := All[project](db)
	tasks, _ := All[task](db)
	equal(t, len(projects), 1)
	equal(t, len(tasks), 3)
}
