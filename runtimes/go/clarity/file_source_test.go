//go:build clarityformats

package clarity

// A data source that is a file in a format (docs/FORMATS-PLAN.md): a CSV
// read into a table its entity finds, saves and deletes in, each change
// written back to the file with its own column order and the columns no
// entity maps kept. Built with the tag clarityformats (formats_library.go).

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

type fileLogin struct {
	ID     int64
	Site   string
	Visits int64
	Paid   bool
}

func (*fileLogin) Table() string     { return "login" }
func (*fileLogin) Source() string    { return "logins" }
func (*fileLogin) Columns() []string { return []string{"Site", "visits", "paid"} }
func (l *fileLogin) GetID() int64    { return l.ID }
func (l *fileLogin) SetID(id int64)  { l.ID = id }
func (l *fileLogin) Values() []any   { return []any{l.Site, l.Visits, l.Paid} }
func (l *fileLogin) Load(id int64, v []any) error {
	r := NewReader(v)
	*l = fileLogin{ID: id, Site: r.Text(0, "site"), Visits: r.Integer(1, "visits"), Paid: r.Bool(2, "paid")}
	return r.Err()
}

const fileLoginFields = `[{"name":"Site","kind":"text"},{"name":"visits","kind":"integer"},{"name":"paid","kind":"bool"}]`

func fileLogins(path string) FileSource {
	return FileSource{Name: "logins", Format: "csv", Path: path, Options: `{"separator":";"}`, Table: "login", Key: "id", Fields: fileLoginFields}
}

func TestACsvFileIsADataSourceWrittenBackAfterEachChange(t *testing.T) {
	folder := t.TempDir()
	file := filepath.Join(folder, "logins.csv")
	if err := os.WriteFile(file, []byte("Site;Note;visits;paid\r\nexample.com;keep me;3;true\r\nshop.nl;;1;false\r\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	db, err := Connect()
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if err := db.AddFile(fileLogins(file), libraryReadTable, libraryWriteTable); err != nil {
		t.Fatal(err)
	}

	all, err := All[fileLogin](db)
	if err != nil {
		t.Fatal(err)
	}
	want := []fileLogin{{ID: 1, Site: "example.com", Visits: 3, Paid: true}, {ID: 2, Site: "shop.nl", Visits: 1, Paid: false}}
	if !reflect.DeepEqual(all, want) {
		t.Fatalf("read %+v, want %+v", all, want)
	}

	all[0].Visits = 4
	if err := Save(db, &all[0]); err != nil {
		t.Fatal(err)
	}
	added := fileLogin{Site: "a;b", Visits: 0, Paid: true}
	if err := Save(db, &added); err != nil {
		t.Fatal(err)
	}
	if err := Delete(db, &all[1]); err != nil {
		t.Fatal(err)
	}
	text, err := os.ReadFile(file)
	if err != nil {
		t.Fatal(err)
	}
	if string(text) != "Site;Note;visits;paid\r\nexample.com;keep me;4;true\r\n\"a;b\";;0;true\r\n" {
		t.Fatalf("the file is %q", text)
	}
	if _, err := os.Stat(file + ".new"); !os.IsNotExist(err) {
		t.Fatal("the new file did not replace the old one")
	}

	// A column the entity maps that the file lacks stops the program as it opens.
	if err := os.WriteFile(file, []byte("Site;paid\r\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	again, err := Connect()
	if err != nil {
		t.Fatal(err)
	}
	defer again.Close()
	err = again.AddFile(fileLogins(file), libraryReadTable, libraryWriteTable)
	if err == nil || !strings.Contains(err.Error(), "data source logins: the CSV has no column 'visits'") {
		t.Fatalf("opening gave %v", err)
	}
}

func TestAFileNotThereYetStartsEmptyAndIsMadeAtTheFirstSave(t *testing.T) {
	file := filepath.Join(t.TempDir(), "fresh", "logins.csv")
	db, err := Connect()
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if err := db.AddFile(fileLogins(file), libraryReadTable, libraryWriteTable); err != nil {
		t.Fatal(err)
	}
	if all, err := All[fileLogin](db); err != nil || len(all) != 0 {
		t.Fatalf("read %+v, %v", all, err)
	}
	if err := Save(db, &fileLogin{Site: "new.nl", Visits: 0, Paid: true}); err != nil {
		t.Fatal(err)
	}
	if text, err := os.ReadFile(file); err != nil || string(text) != "Site;visits;paid\r\nnew.nl;0;true\r\n" {
		t.Fatalf("the file is %q, %v", text, err)
	}
	// A readonly source needs its file.
	source := fileLogins(filepath.Join(t.TempDir(), "missing.csv"))
	source.Readonly = true
	err = db.AddFile(source, libraryReadTable, libraryWriteTable)
	if err == nil || !strings.Contains(err.Error(), "data source logins: cannot read") {
		t.Fatalf("opening gave %v", err)
	}
}

type fileOrder struct {
	ID     int64
	Number string
}

func (*fileOrder) Table() string     { return "order" }
func (*fileOrder) Source() string    { return "feed" }
func (*fileOrder) Columns() []string { return []string{"number"} }
func (o *fileOrder) GetID() int64    { return o.ID }
func (o *fileOrder) SetID(id int64)  { o.ID = id }
func (o *fileOrder) Values() []any   { return []any{o.Number} }
func (o *fileOrder) Load(id int64, v []any) error {
	r := NewReader(v)
	*o = fileOrder{ID: id, Number: r.Text(0, "number")}
	return r.Err()
}

type fileLine struct {
	ID      int64
	Order   int64
	Product string
}

func (*fileLine) Table() string     { return "line" }
func (*fileLine) Source() string    { return "feed" }
func (*fileLine) Columns() []string { return []string{"order", "product"} }
func (l *fileLine) GetID() int64    { return l.ID }
func (l *fileLine) SetID(id int64)  { l.ID = id }
func (l *fileLine) Values() []any   { return []any{l.Order, l.Product} }
func (l *fileLine) Load(id int64, v []any) error {
	r := NewReader(v)
	*l = fileLine{ID: id, Order: r.Integer(0, "order"), Product: r.Text(1, "product")}
	return r.Err()
}

const (
	ordersXML     = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Tea\"/><line product=\"Coffee\"/></lines></order>\n    <order number=\"A-2\"><lines><line product=\"Milk\"/></lines></order>\n  </orders>\n</shop>\n"
	ordersOptions = `{"at":"shop/orders/order","fields":{"number":"@number"},"nested":{"lines":{"at":"lines/line","fields":{"product":"@product"},"parent":"order"}}}`
	ordersFields  = `[{"name":"number","kind":"text"},{"name":"lines","kind":"records","entity":"Line","fields":[{"name":"product","kind":"text"}],"base":{},"table":"line","key":"id"}]`
)

func TestAFileSourceHoldsNestedRecordsInTheirOwnTable(t *testing.T) {
	file := filepath.Join(t.TempDir(), "orders.xml")
	if err := os.WriteFile(file, []byte(ordersXML), 0o644); err != nil {
		t.Fatal(err)
	}
	db, err := Connect()
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	source := FileSource{Name: "feed", Format: "xml", Path: file, Options: ordersOptions, Table: "order", Key: "id", Fields: ordersFields}
	if err := db.AddFile(source, libraryReadTable, libraryWriteTable); err != nil {
		t.Fatal(err)
	}
	orders, err := All[fileOrder](db)
	if err != nil {
		t.Fatal(err)
	}
	if want := []fileOrder{{1, "A-1"}, {2, "A-2"}}; !reflect.DeepEqual(orders, want) {
		t.Fatalf("orders %+v, want %+v", orders, want)
	}
	lines, err := All[fileLine](db)
	if err != nil {
		t.Fatal(err)
	}
	if want := []fileLine{{1, 1, "Tea"}, {2, 1, "Coffee"}, {3, 2, "Milk"}}; !reflect.DeepEqual(lines, want) {
		t.Fatalf("lines %+v, want %+v", lines, want)
	}
	if err := Save(db, &fileLine{Order: 2, Product: "Juice"}); err != nil {
		t.Fatal(err)
	}
	if err := Delete(db, &lines[0]); err != nil {
		t.Fatal(err)
	}
	want := "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Coffee\"/></lines></order>\n    <order number=\"A-2\"><lines><line product=\"Milk\"/><line product=\"Juice\"/></lines></order>\n  </orders>\n</shop>\n"
	if text, err := os.ReadFile(file); err != nil || string(text) != want {
		t.Fatalf("the file is %q, %v", text, err)
	}
}
