package clarity

import "testing"

// A stored record's list (order.lines) loads with the record, one level deep:
// Find and Get call LoadLists, which reads the lines with FindAlone.

type heldOrder struct {
	ID     int64
	Number string
	Lines  []heldLine
}

func (*heldOrder) Table() string        { return "held_order" }
func (*heldOrder) Columns() []string    { return []string{"number"} }
func (o *heldOrder) GetID() int64       { return o.ID }
func (o *heldOrder) SetID(id int64)     { o.ID = id }
func (o *heldOrder) Values() []any      { return []any{o.Number} }
func (o *heldOrder) Load(id int64, v []any) error {
	r := NewReader(v)
	*o = heldOrder{ID: id, Number: r.Text(0, "number")}
	return r.Err()
}
func (o *heldOrder) LoadLists(db *Db) error {
	lines, err := FindAlone[heldLine](db, Query{}.Filter(Compare("held_order", Eq, o.ID)).Order("id", false))
	if err != nil {
		return err
	}
	o.Lines = lines
	return nil
}

type heldLine struct {
	ID      int64
	Order   *int64
	Product string
}

func (*heldLine) Table() string     { return "held_line" }
func (*heldLine) Columns() []string { return []string{"held_order", "product"} }
func (l *heldLine) GetID() int64    { return l.ID }
func (l *heldLine) SetID(id int64)  { l.ID = id }
func (l *heldLine) Values() []any   { return []any{l.Order, l.Product} }
func (l *heldLine) Load(id int64, v []any) error {
	r := NewReader(v)
	*l = heldLine{id, r.Reference(0, "held_order"), r.Text(1, "product")}
	return r.Err()
}

func TestAStoredRecordsListsLoadWithIt(t *testing.T) {
	db, err := Connect(Source{"", Sqlite, ":memory:"})
	must(t, err)
	defer db.Close()
	_, err = db.Migrate([][]string{{
		"CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL)",
		"CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL)",
	}})
	must(t, err)
	first, second := &heldOrder{Number: "ORD-1"}, &heldOrder{Number: "ORD-2"}
	must(t, Save(db, first))
	must(t, Save(db, second))
	for _, p := range []string{"pen", "ink"} {
		must(t, Save(db, &heldLine{Order: &first.ID, Product: p}))
	}
	found, err := Find[heldOrder](db, Query{}.Order("number", false))
	must(t, err)
	products := []string{}
	for _, l := range found[0].Lines {
		products = append(products, l.Product)
	}
	equal(t, products, []string{"pen", "ink"})
	equal(t, len(found[1].Lines), 0)
	got, err := Get[heldOrder](db, first.ID)
	must(t, err)
	equal(t, len(got.Lines), 2)
	alone, err := FindAlone[heldOrder](db, Query{})
	must(t, err)
	equal(t, len(alone[0].Lines), 0)
}
