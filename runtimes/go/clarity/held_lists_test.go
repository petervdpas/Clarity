package clarity

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"strings"
	"testing"
)

// A stored record's list (order.lines) loads with the record, one level deep:
// Find and All call LoadListsOf for every record found, which reads each list
// once for all of them (FindByOwners); Get loads one record's the same way.

// heldLineReads counts the reads of the lines: once per list for a whole find.
var heldLineReads int

type heldOrder struct {
	ID     int64
	Number string
	Lines  []heldLine
}

func (*heldOrder) Table() string     { return "held_order" }
func (*heldOrder) Columns() []string { return []string{"number"} }
func (o *heldOrder) GetID() int64    { return o.ID }
func (o *heldOrder) SetID(id int64)  { o.ID = id }
func (o *heldOrder) Values() []any   { return []any{o.Number} }
func (o *heldOrder) Load(id int64, v []any) error {
	r := NewReader(v)
	*o = heldOrder{ID: id, Number: r.Text(0, "number")}
	return r.Err()
}

// As generated code writes them.
func (o *heldOrder) LoadLists(db *Db) error {
	all := []heldOrder{*o}
	if err := o.LoadListsOf(db, all); err != nil {
		return err
	}
	*o = all[0]
	return nil
}
func (*heldOrder) LoadListsOf(db *Db, records []heldOrder) error {
	owners := make([]int64, len(records))
	for i := range records {
		owners[i] = records[i].ID
	}
	heldLineReads += (len(owners) + OwnersPerRead - 1) / OwnersPerRead
	lines, err := FindByOwners[heldLine](db, "held_order", owners)
	if err != nil {
		return err
	}
	byOrder := ByOwner(lines, func(l *heldLine) *int64 { return l.Order })
	for i := range records {
		records[i].Lines = byOrder[records[i].ID]
	}
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
	_, err = applySteps(db, "", [][]string{{
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

// Every case of conformance/held-lists.json: a find reads each list once per
// ownersPerRead orders and gives each order its own lines in order.
func TestAFindReadsEachListOnceForAllTheRecordsItFinds(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/held-lists.json")
	must(t, err)
	var file struct {
		OwnersPerRead int `json:"ownersPerRead"`
		Cases         []struct {
			Name   string              `json:"name"`
			Orders int                 `json:"orders"`
			Lines  [][2]any            `json:"lines"`
			Expect map[string][]string `json:"expect"`
			Reads  int                 `json:"reads"`
		} `json:"cases"`
	}
	must(t, json.Unmarshal(text, &file))
	equal(t, file.OwnersPerRead, OwnersPerRead)
	for _, c := range file.Cases {
		db, err := Connect(Source{"", Sqlite, ":memory:"})
		must(t, err)
		_, err = applySteps(db, "", [][]string{{
			"CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL)",
			"CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL)",
		}})
		must(t, err)
		for n := 1; n <= c.Orders; n++ {
			must(t, Save(db, &heldOrder{Number: fmt.Sprintf("ORD-%05d", n)}))
		}
		for _, l := range c.Lines {
			order := int64(l[0].(float64))
			must(t, Save(db, &heldLine{Order: &order, Product: l[1].(string)}))
		}
		heldLineReads = 0
		found, err := Find[heldOrder](db, Query{}.Order("number", false))
		must(t, err)
		if heldLineReads != c.Reads {
			t.Errorf("%s: %d reads, want %d", c.Name, heldLineReads, c.Reads)
		}
		byID := map[string]heldOrder{}
		for _, o := range found {
			byID[strconv.FormatInt(o.ID, 10)] = o
		}
		for id, want := range c.Expect {
			got := []string{}
			for _, l := range byID[id].Lines {
				got = append(got, l.Product)
			}
			if strings.Join(got, " ") != strings.Join(want, " ") {
				t.Errorf("%s: order %s holds %v, want %v", c.Name, id, got, want)
			}
			one, err := Get[heldOrder](db, byID[id].ID)
			must(t, err)
			equal(t, len(one.Lines), len(want))
		}
		db.Close()
	}
}
