// A save never overwrites someone else's change (conformance/versions.json):
// the version read with every record, the checked save and delete, a save
// anyway, reload, undo by the version, and a file source changed on disk.

package clarity_test

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	. "github.com/petervdpas/Clarity/runtimes/go/clarity"
)

type item struct {
	ID      int64
	Name    string
	Version int64
}

func (*item) Table() string         { return "item" }
func (*item) Columns() []string     { return []string{"name"} }
func (i *item) GetID() int64        { return i.ID }
func (i *item) SetID(id int64)      { i.ID = id }
func (i *item) Values() []any       { return []any{i.Name} }
func (*item) VersionColumn() string { return "row_version" }
func (i *item) GetVersion() int64   { return i.Version }
func (i *item) SetVersion(v int64)  { i.Version = v }
func (i *item) Load(id int64, v []any) error {
	r := NewReader(v)
	i.ID, i.Name = id, r.Text(0, "name")
	return r.Err()
}

type versionsFile struct {
	Refusal     struct{ Key, English string }
	UndoRefusal struct{ Key, English string }
	Table       string
	Cases       []struct {
		Name  string
		Steps []versionStep
	}
	File struct {
		Format string
		Fields json.RawMessage
		Text   string
		Steps  []versionStep
	}
}

type versionStep struct {
	Do      string
	Copy    string
	Of      string
	Name    string
	Is      int64
	Version int64
	Gone    bool
	Text    string
	Expect  string
}

func readVersions(t *testing.T) versionsFile {
	raw, err := os.ReadFile("../../../conformance/versions.json")
	if err != nil {
		t.Fatal(err)
	}
	var file versionsFile
	if err := json.Unmarshal(raw, &file); err != nil {
		t.Fatal(err)
	}
	return file
}

// expect checks that err is what step expects: ok, refused with refusal's words, or not found.
func expect(t *testing.T, name string, step versionStep, err error, key, english string) {
	t.Helper()
	var refused *RefusedError
	var missing *NotFoundError
	switch want := step.Expect; {
	case want == "" && err == nil:
	case want == "refused" && errors.As(err, &refused):
		if refused.Text.Key != key || refused.Text.Template != english {
			t.Fatalf("%s: %+v refused with %q %q", name, step, refused.Text.Key, refused.Text.Template)
		}
	case want == "notFound" && errors.As(err, &missing):
	default:
		t.Fatalf("%s: %+v gave %v, expected %q", name, step, err, want)
	}
}

func TestVersionsConformance(t *testing.T) {
	file := readVersions(t)
	for _, c := range file.Cases {
		db, err := Memory()
		if err != nil {
			t.Fatal(err)
		}
		if err := db.Batch("", file.Table); err != nil {
			t.Fatal(err)
		}
		copies := map[string]*item{}
		var last int64
		key, english := file.Refusal.Key, file.Refusal.English
		for _, step := range c.Steps {
			switch step.Do {
			case "insert":
				it := &item{Name: step.Name}
				expect(t, c.Name, step, Save(db, it), key, english)
				copies[step.Copy] = it
			case "read":
				found, err := Get[item](db, copies[step.Of].ID)
				if err != nil {
					t.Fatal(err)
				}
				copies[step.Copy] = &found
			case "set":
				copies[step.Copy].Name = step.Name
			case "save":
				expect(t, c.Name, step, Save(db, copies[step.Copy]), key, english)
			case "saveAnyway":
				expect(t, c.Name, step, SaveAnyway(db, copies[step.Copy]), key, english)
			case "delete":
				expect(t, c.Name, step, Delete(db, copies[step.Copy]), key, english)
			case "reload":
				expect(t, c.Name, step, Reload[item](db, copies[step.Copy]), key, english)
			case "version":
				if copies[step.Copy].Version != step.Is {
					t.Fatalf("%s: %+v holds %d", c.Name, step, copies[step.Copy].Version)
				}
			case "stored":
				rows, err := db.Rows("", "SELECT name, row_version FROM item WHERE id = ?", copies[step.Copy].ID)
				if err != nil {
					t.Fatal(err)
				}
				if step.Gone {
					if len(rows) != 0 {
						t.Fatalf("%s: %+v is still there", c.Name, step)
					}
					continue
				}
				if len(rows) != 1 || rows[0][0] != step.Name || rows[0][1] != step.Version {
					t.Fatalf("%s: %+v is stored as %v", c.Name, step, rows)
				}
			case "stepSave":
				it := copies[step.Copy]
				expect(t, c.Name, step, db.Step(func() error { return Save(db, it) }), key, english)
				last = db.LastStep()
			case "undo":
				_, err := db.Undo(last)
				expect(t, c.Name, step, err, file.UndoRefusal.Key, file.UndoRefusal.English)
			case "redo":
				_, err := db.Redo(last)
				expect(t, c.Name, step, err, file.UndoRefusal.Key, file.UndoRefusal.English)
			default:
				t.Fatalf("%s: no step %s", c.Name, step.Do)
			}
		}
		db.Close()
	}
}

type line struct {
	ID   int64
	Name string
}

func (*line) Source() string    { return "lines" }
func (*line) Table() string     { return "line" }
func (*line) Columns() []string { return []string{"name"} }
func (l *line) GetID() int64    { return l.ID }
func (l *line) SetID(id int64)  { l.ID = id }
func (l *line) Values() []any   { return []any{l.Name} }
func (l *line) Load(id int64, v []any) error {
	r := NewReader(v)
	*l = line{id, r.Text(0, "name")}
	return r.Err()
}

// A CSV of one column with a header, enough for the file case, so the test needs no library.
func readOneColumn(_ string, input []byte, _, _ string) (string, error) {
	lines := strings.Split(strings.TrimRight(string(input), "\n"), "\n")
	rows := [][]any{}
	for _, l := range lines[1:] {
		rows = append(rows, []any{l})
	}
	out, err := json.Marshal(map[string]any{"columns": []string{lines[0]}, "rows": rows})
	return string(out), err
}

func writeOneColumn(_ string, _ []byte, table, _, _ string) ([]byte, error) {
	var t struct {
		Columns []string
		Rows    [][]any
	}
	if err := json.Unmarshal([]byte(table), &t); err != nil {
		return nil, err
	}
	out := t.Columns[0] + "\n"
	for _, r := range t.Rows {
		out += r[0].(string) + "\n"
	}
	return []byte(out), nil
}

func TestAFileChangedOnDiskRefusesTheSave(t *testing.T) {
	file := readVersions(t)
	path := filepath.Join(t.TempDir(), "lines.csv")
	if err := os.WriteFile(path, []byte(file.File.Text), 0o644); err != nil {
		t.Fatal(err)
	}
	db, err := Connect()
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	source := FileSource{Name: "lines", Format: file.File.Format, Path: path, Options: "{}", Table: "line", Key: "id", Fields: string(file.File.Fields)}
	if err := db.AddFile(source, readOneColumn, writeOneColumn); err != nil {
		t.Fatal(err)
	}
	all, err := All[line](db)
	if err != nil {
		t.Fatal(err)
	}
	first := &all[0]
	for _, step := range file.File.Steps {
		switch step.Do {
		case "save":
			first.Name = step.Name
			expect(t, "file", step, Save(db, first), file.Refusal.Key, file.Refusal.English)
		case "rewrite":
			bytes, _ := os.ReadFile(path)
			// A later modification time, as a program writing the same bytes gives.
			time.Sleep(20 * time.Millisecond)
			os.WriteFile(path, bytes, 0o644)
		case "write":
			os.WriteFile(path, []byte(step.Text), 0o644)
		case "contents":
			got, _ := os.ReadFile(path)
			if string(got) != step.Text {
				t.Fatalf("file holds %q", got)
			}
		default:
			t.Fatalf("file: no step %s", step.Do)
		}
	}
}

// worded is an item whose entity words the refusal itself, about the record
// as stored now and what the database holds (as {current.changedBy.name} needs).
type worded struct{ item }

func (w *worded) SavedSince(db *Db) Text {
	all, _ := All[worded](db)
	return PlainText(fmt.Sprintf("%s saved it; %d records", w.Name, len(all)))
}

func TestAnEntityWordsTheRefusalWithTheDatabaseAtHand(t *testing.T) {
	db, err := Memory()
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if err := db.Batch("", readVersions(t).Table); err != nil {
		t.Fatal(err)
	}
	first := &worded{item{Name: "first"}}
	if err := Save(db, first); err != nil {
		t.Fatal(err)
	}
	second, err := Get[worded](db, first.ID)
	if err != nil {
		t.Fatal(err)
	}
	first.Name = "Anna"
	if err := Save(db, first); err != nil {
		t.Fatal(err)
	}
	for _, err := range []error{Save(db, &second), Delete(db, &second)} {
		var refused *RefusedError
		if !errors.As(err, &refused) || refused.Text.Show() != "Anna saved it; 1 records" {
			t.Fatalf("not refused in the entity's words: %v", err)
		}
	}
}
