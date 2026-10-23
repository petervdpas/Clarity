// Files in records (U1c), against the cases every runtime shares
// (conformance/record-files.json), and kept in SQLite or in a file store.

package clarity

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func recordFileCases(t *testing.T) map[string]json.RawMessage {
	text, err := os.ReadFile("../../../conformance/record-files.json")
	must(t, err)
	var cases map[string]json.RawMessage
	must(t, json.Unmarshal(text, &cases))
	return cases
}

func TestAFileInAStoreIsKeptUnderItsNameInAFolderOfItsContents(t *testing.T) {
	cases := recordFileCases(t)
	var paths []struct{ Text, Chosen, Path string }
	must(t, json.Unmarshal(cases["storePaths"], &paths))
	for _, c := range paths {
		if got := StorePath([]byte(c.Text), c.Chosen); got != c.Path {
			t.Errorf("%v: %q", c, got)
		}
	}
	var names []struct{ Path, Name string }
	must(t, json.Unmarshal(cases["names"], &names))
	for _, c := range names {
		if got := NameOf(c.Path); got != c.Name {
			t.Errorf("%v: %q", c, got)
		}
	}
}

func TestACopyGoesToTheDownloadsFolderUnderAFreeName(t *testing.T) {
	cases := recordFileCases(t)
	var downloads []struct {
		OS     string            `json:"os"`
		Env    map[string]string `json:"env"`
		Folder *string           `json:"folder"`
	}
	must(t, json.Unmarshal(cases["downloads"], &downloads))
	for _, c := range downloads {
		got, ok := DownloadsIn(c.OS, func(name string) string { return c.Env[name] })
		if (c.Folder == nil) == ok || (ok && got != *c.Folder) {
			t.Errorf("%v: %q %v", c, got, ok)
		}
	}
	var copies []struct {
		Name  string   `json:"name"`
		Taken []string `json:"taken"`
		Saved string   `json:"saved"`
	}
	must(t, json.Unmarshal(cases["copies"], &copies))
	for _, c := range copies {
		taken := func(n string) bool {
			for _, t := range c.Taken {
				if t == n {
					return true
				}
			}
			return false
		}
		if got := FreeName(c.Name, taken); got != c.Saved {
			t.Errorf("%v: %q", c, got)
		}
	}
}

type product struct {
	ID     int64
	Manual *RecordFile
	Sheet  *RecordFile
}

func (*product) Table() string { return "product" }
func (*product) Columns() []string {
	return []string{"manual", "manual_name", "sheet", "sheet_name"}
}
func (*product) Pictures() []PictureColumn {
	return []PictureColumn{{Column: "manual", Name: "manual_name"}, {Column: "sheet", Store: "documents", Name: "sheet_name"}}
}
func (p *product) GetID() int64   { return p.ID }
func (p *product) SetID(id int64) { p.ID = id }
func (p *product) Values() []any {
	return []any{FileContent(p.Manual), FileNameValue(p.Manual), FileContent(p.Sheet), FileNameValue(p.Sheet)}
}
func (p *product) Load(id int64, v []any) error {
	r := NewReader(v)
	*p = product{id, r.File(0, "manual"), r.File(2, "sheet")}
	return r.Err()
}

func TestAFileIsKeptWithItsNameInTheDatabaseOrItsStore(t *testing.T) {
	store := t.TempDir()
	db, err := Memory()
	must(t, err)
	t.Cleanup(func() { db.Close() })
	_, err = db.Migrate([][]string{{"CREATE TABLE product (id INTEGER PRIMARY KEY, manual BLOB, manual_name TEXT NOT NULL DEFAULT '', sheet TEXT, sheet_name TEXT NOT NULL DEFAULT '')"}})
	must(t, err)
	must(t, db.AddStore("documents", store))
	pdf := []byte("%PDF-1.7 not a picture")
	p := &product{Manual: &RecordFile{"manual.pdf", NewPicture(pdf)}, Sheet: &RecordFile{"sheet.pdf", NewPicture(pdf)}}
	must(t, Save(db, p))
	read, err := Get[product](db, p.ID)
	must(t, err)
	if read.Manual == nil || read.Manual.Name != "manual.pdf" || read.Sheet == nil || read.Sheet.Name != "sheet.pdf" {
		t.Fatalf("read %+v", read)
	}
	got, err := PictureOf[product](db, p.ID, "manual")
	must(t, err)
	if !bytes.Equal(got, pdf) {
		t.Errorf("manual %q", got)
	}
	kept, err := os.ReadFile(filepath.Join(store, filepath.FromSlash(StorePath(pdf, "sheet.pdf"))))
	must(t, err)
	if !bytes.Equal(kept, pdf) {
		t.Errorf("sheet in the store %q", kept)
	}
	read.Manual = nil
	must(t, Save(db, &read))
	again, err := Get[product](db, p.ID)
	must(t, err)
	if again.Manual != nil {
		t.Errorf("emptied, still %+v", again.Manual)
	}
}

func TestAFileTravelsWithItsName(t *testing.T) {
	text, err := json.Marshal(&RecordFile{"manual.pdf", StoredPicture()})
	must(t, err)
	if string(text) != `{"name":"manual.pdf","stored":true}` {
		t.Errorf("%s", text)
	}
	var f RecordFile
	must(t, json.Unmarshal([]byte(`{"name":"C:\\docs\\a.pdf","data":"JVBERg=="}`), &f))
	if f.Name != "a.pdf" || string(f.Content.Bytes) != "%PDF" {
		t.Errorf("%+v", f)
	}
}
