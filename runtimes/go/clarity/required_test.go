// Required fields: a save refuses a record whose required field is empty,
// as every runtime does (conformance/required.json).

package clarity_test

import (
	"encoding/json"
	"errors"
	"os"
	"testing"

	. "github.com/petervdpas/Clarity/runtimes/go/clarity"
)

func TestRequiredFieldsAreCheckedAsEveryRuntimeDoes(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/required.json")
	must(t, err)
	var file struct {
		Refusal struct {
			Key string `json:"key"`
		} `json:"refusal"`
		Cases []struct {
			Name     string          `json:"name"`
			Required []RequiredField `json:"required"`
			Values   map[string]any  `json:"values"`
			Refused  *string         `json:"refused"`
		} `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	for _, c := range file.Cases {
		var columns []string
		var values []any
		for column, value := range c.Values {
			// A number in the cases is a whole number.
			if n, ok := value.(float64); ok {
				value = int64(n)
			}
			columns = append(columns, column)
			values = append(values, value)
		}
		err := MissingRequired(c.Required, columns, values)
		if c.Refused == nil {
			if err != nil {
				t.Errorf("%s: refused: %v", c.Name, err)
			}
			continue
		}
		var refused *RefusedError
		if !errors.As(err, &refused) {
			t.Errorf("%s: not refused: %v", c.Name, err)
			continue
		}
		if refused.Text.Key != file.Refusal.Key {
			t.Errorf("%s: key %q", c.Name, refused.Text.Key)
		}
		if got := refused.Text.Values["field"].Value; got != *c.Refused {
			t.Errorf("%s: field %v, want %q", c.Name, got, *c.Refused)
		}
	}
}

// member is a record with a required name and a required reference.
type member struct {
	ID    int64
	Name  string
	Group *int64
}

func (*member) Table() string     { return "member" }
func (*member) Columns() []string { return []string{"name", "group_id"} }
func (m *member) GetID() int64    { return m.ID }
func (m *member) SetID(id int64)  { m.ID = id }
func (m *member) Values() []any   { return []any{m.Name, m.Group} }
func (m *member) Load(id int64, v []any) error {
	r := NewReader(v)
	m.ID, m.Name, m.Group = id, r.Text(0, "name"), r.Reference(1, "group_id")
	return r.Err()
}
func (*member) Required() []RequiredField {
	return []RequiredField{{Column: "name", Label: "Name"}, {Column: "group_id", Label: "@groupLabel"}}
}

// guest is a member whose before save fills an empty name.
type guest struct{ member }

func (g *guest) BeforeSave(db *Db) error {
	if g.Name == "" {
		g.Name = "Guest"
	}
	return nil
}

func TestASaveRefusesARecordWithAnEmptyRequiredField(t *testing.T) {
	db, err := Connect(Source{"", Sqlite, ":memory:"})
	must(t, err)
	defer db.Close()
	_, err = db.Migrate([][]string{{"CREATE TABLE member (id INTEGER PRIMARY KEY, name TEXT NOT NULL, group_id INTEGER)"}})
	must(t, err)
	LoadTranslations([]byte(`{"project":{"translations":{"en":{"groupLabel":"Group"}}}}`))
	defer LoadTranslations(nil)

	one := int64(1)
	empty := &member{Name: "", Group: &one}
	var refused *RefusedError
	if err := Save(db, empty); !errors.As(err, &refused) || refused.Text.Key != "fieldRequired" || refused.Text.Values["field"].Value != "Name" {
		t.Fatalf("an empty name was not refused: %v", err)
	}
	// A label written as a key takes the key's value.
	noGroup := &member{Name: "Ada"}
	if err := Save(db, noGroup); !errors.As(err, &refused) || refused.Text.Values["field"].Value != "Group" {
		t.Fatalf("no group was not refused: %v", err)
	}
	// Before save may fill a required field; a refused one keeps a new record new.
	filled := &guest{member{Group: &one}}
	must(t, Save(db, filled))
	if filled.ID == 0 {
		t.Fatal("a filled record was not saved")
	}
	unfilled := &guest{member{Name: "", Group: nil}}
	if err := Save(db, unfilled); !errors.As(err, &refused) || unfilled.ID != 0 {
		t.Fatalf("a record with no group was saved: %v, id %d", err, unfilled.ID)
	}
	stored, err := All[member](db)
	must(t, err)
	if len(stored) != 1 || stored[0].Name != "Guest" {
		t.Fatalf("stored %v", stored)
	}
}
