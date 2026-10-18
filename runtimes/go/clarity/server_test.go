// The generated migration steps (conformance/migrations.json) applied by the
// Go runtime, with records stored between the steps: the same scenario as
// the Rust and C# runtimes' server tests. It runs on SQLite always, and on
// a server database when its variable says where one is to spare:
//
//	CLARITY_TEST_POSTGRES="host=localhost port=5432 user=postgres password=... dbname=scratch"
//	CLARITY_TEST_MYSQL="mysql://root:...@localhost:3306/scratch"
//	CLARITY_TEST_MSSQL="server=tcp:localhost,1433;database=scratch;user=sa;password=...;TrustServerCertificate=true"

package clarity

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/shopspring/decimal"
)

// A customer as the first step stores it.
type before struct {
	ID     int64
	Name   string
	Email  string
	Kind   string
	Limit  decimal.Decimal
	Active bool
	Since  *Date
	Notes  string
}

func (*before) Table() string { return "customer" }
func (*before) Columns() []string {
	return []string{"name", "email", "kind", "limit", "active", "since", "notes"}
}
func (b *before) GetID() int64   { return b.ID }
func (b *before) SetID(id int64) { b.ID = id }
func (b *before) Values() []any {
	return []any{b.Name, b.Email, b.Kind, b.Limit, b.Active, b.Since, b.Notes}
}
func (b *before) Load(id int64, v []any) error {
	r := NewReader(v)
	*b = before{id, r.Text(0, "name"), r.Text(1, "email"), r.Text(2, "kind"), r.Decimal(3, "limit"), r.Bool(4, "active"), r.Date(5, "since"), r.Text(6, "notes")}
	return r.Err()
}

// An order as the first step stores it.
type order struct {
	ID       int64
	Number   string
	Customer *int64
	Total    int64
	Placed   *DateTime
	Paid     bool
	Code     string
}

func (*order) Table() string { return "order" }
func (*order) Columns() []string {
	return []string{"number", "customer", "total", "placed", "paid", "code"}
}
func (o *order) GetID() int64   { return o.ID }
func (o *order) SetID(id int64) { o.ID = id }
func (o *order) Values() []any {
	return []any{o.Number, o.Customer, o.Total, o.Placed, o.Paid, o.Code}
}
func (o *order) Load(id int64, v []any) error {
	r := NewReader(v)
	*o = order{id, r.Text(0, "number"), r.Reference(1, "customer"), r.Integer(2, "total"), r.DateTime(3, "placed"), r.Bool(4, "paid"), r.Text(5, "code")}
	return r.Err()
}

// A customer after the second step.
type customer struct {
	ID          int64
	Name        string
	Email       string
	Kind        string
	CreditLimit decimal.Decimal
	Vip         bool
	Remarks     string
	Phone       string
}

func (*customer) Table() string { return "customer" }
func (*customer) Columns() []string {
	return []string{"name", "email", "kind", "credit_limit", "vip", "remarks", "phone"}
}
func (c *customer) GetID() int64   { return c.ID }
func (c *customer) SetID(id int64) { c.ID = id }
func (c *customer) Values() []any {
	return []any{c.Name, c.Email, c.Kind, c.CreditLimit, c.Vip, c.Remarks, c.Phone}
}
func (c *customer) Load(id int64, v []any) error {
	r := NewReader(v)
	*c = customer{id, r.Text(0, "name"), r.Text(1, "email"), r.Text(2, "kind"), r.Decimal(3, "credit_limit"), r.Bool(4, "vip"), r.Text(5, "remarks"), r.Text(6, "phone")}
	return r.Err()
}

// An order after the second step: another table, and a money total.
type purchase struct {
	ID       int64
	Number   string
	Customer *int64
	Total    decimal.Decimal
	Placed   *DateTime
	Paid     int64
	Code     int64
}

func (*purchase) Table() string { return "purchase" }
func (*purchase) Columns() []string {
	return []string{"number", "customer", "total", "placed", "paid", "code"}
}
func (p *purchase) GetID() int64   { return p.ID }
func (p *purchase) SetID(id int64) { p.ID = id }
func (p *purchase) Values() []any {
	return []any{p.Number, p.Customer, p.Total, p.Placed, p.Paid, p.Code}
}
func (p *purchase) Load(id int64, v []any) error {
	r := NewReader(v)
	*p = purchase{id, r.Text(0, "number"), r.Reference(1, "customer"), r.Decimal(2, "total"), r.DateTime(3, "placed"), r.Integer(4, "paid"), r.Integer(5, "code")}
	return r.Err()
}

func steps(t *testing.T, database string) [][]string {
	text, err := os.ReadFile("../../../conformance/migrations.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases struct {
		Steps map[string][][]string `json:"steps"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	return cases.Steps[database]
}

func names[T any](found []T, name func(T) string) []string {
	out := []string{}
	for _, f := range found {
		out = append(out, name(f))
	}
	return out
}

func must(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}

func equal(t *testing.T, got, want any) {
	t.Helper()
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %#v, want %#v", got, want)
	}
}

func ref(id int64) *int64 { return &id }

func migratesAndKeepsRecords(t *testing.T, dialect Dialect, at, database string) {
	db, err := Connect(Source{"main", dialect, at})
	must(t, err)
	defer db.Close()
	for _, table := range []string{"purchase", "order", "customer", "clarity_schema"} {
		must(t, db.Batch("main", "DROP TABLE IF EXISTS "+dialect.Quote(table)))
	}
	all := steps(t, database)

	done, err := db.Migrate(all[:1])
	must(t, err)
	equal(t, done, 1)
	since := Date{2026, 1, 15}
	ada := &before{Name: "Ada Lovelace", Email: "ada@example.com", Kind: "Business", Limit: decimal.RequireFromString("1250.50"), Active: true, Since: &since, Notes: "first's"}
	must(t, Save(db, ada))
	grace := &before{Name: "Grace Hopper", Kind: "Private", Limit: decimal.RequireFromString("99.5")}
	must(t, Save(db, grace))
	got, err := Get[before](db, ada.ID)
	must(t, err)
	if got.Name != ada.Name || !got.Limit.Equal(ada.Limit) || *got.Since != since || got.Notes != ada.Notes || !got.Active {
		t.Fatalf("got %#v", got)
	}
	// Saving what is stored already is no missing record.
	must(t, Save(db, grace))

	placed := DateTime{Date{2026, 10, 17}, 9, 30, 15}
	o := &order{Number: "ORD-1", Customer: ref(ada.ID), Total: 99, Placed: &placed, Paid: true, Code: "42"}
	must(t, Save(db, o))
	blank := &order{Number: "ORD-0"}
	must(t, Save(db, blank))
	must(t, Save(db, o))
	// Through a reference: the customer's name, in a condition and an order.
	byNumber := func(o order) string { return o.Number }
	through, err := Find[order](db, Query{}.Filter(Through("customer", "customer", "id", Contains("name", "LOVE"))))
	must(t, err)
	equal(t, names(through, byNumber), []string{"ORD-1"})
	grace2 := &order{Number: "ORD-G", Customer: ref(grace.ID)}
	must(t, Save(db, grace2))
	through, err = Find[order](db, Query{}.Filter(Not(IsNone("customer"))).OrderThrough([]Hop{{Column: "customer", Table: "customer", Key: "id"}}, "name", true, false))
	must(t, err)
	equal(t, names(through, byNumber), []string{"ORD-G", "ORD-1"})
	must(t, Delete(db, grace2))

	var inUse *InUseError
	if err := Delete(db, ada); !errors.As(err, &inUse) || inUse.Table != "customer" || inUse.ID != ada.ID {
		t.Fatalf("deleting a customer with orders: %v", err)
	}

	// Queries: contains ignores case and wildcards, decimals compare as numbers.
	byName := func(c before) string { return c.Name }
	found, err := Find[before](db, Query{}.Filter(Contains("name", "LOVE")))
	must(t, err)
	equal(t, names(found, byName), []string{"Ada Lovelace"})
	found, err = Find[before](db, Query{}.Filter(Contains("name", "%")))
	must(t, err)
	equal(t, len(found), 0)
	found, err = Find[before](db, Query{}.Filter(And(Number("limit", Gt, int64(900)), IsTrue("active"))))
	must(t, err)
	equal(t, names(found, byName), []string{"Ada Lovelace"})
	found, err = Find[before](db, Query{}.OrderNumber("limit", false))
	must(t, err)
	equal(t, names(found, byName), []string{"Grace Hopper", "Ada Lovelace"})
	found, err = Find[before](db, Query{}.Filter(Compare("since", Eq, &since)))
	must(t, err)
	equal(t, len(found), 1)
	found, err = Find[before](db, Query{}.Filter(OneOf("kind", []any{"Private"})))
	must(t, err)
	equal(t, names(found, byName), []string{"Grace Hopper"})
	found, err = Find[before](db, Query{}.Filter(IsNone("since")))
	must(t, err)
	equal(t, names(found, byName), []string{"Grace Hopper"})

	done, err = db.Migrate(all)
	must(t, err)
	equal(t, done, 1)
	done, err = db.Migrate(all)
	must(t, err)
	equal(t, done, 0)
	version, err := db.SchemaVersion()
	must(t, err)
	equal(t, version, 2)

	c, err := Get[customer](db, ada.ID)
	must(t, err)
	if !c.CreditLimit.Equal(decimal.RequireFromString("1250.50")) || !c.Vip || c.Remarks != "first's" || c.Phone != "" {
		t.Fatalf("customer after the second step: %#v", c)
	}
	p, err := Get[purchase](db, o.ID)
	must(t, err)
	if !p.Total.Equal(decimal.NewFromInt(99)) || *p.Customer != ada.ID || *p.Placed != placed || p.Paid != 1 || p.Code != 42 {
		t.Fatalf("purchase after the second step: %#v", p)
	}
	// Empty text is no number: it becomes the default.
	b, err := Get[purchase](db, blank.ID)
	must(t, err)
	equal(t, b.Code, int64(0))

	// Unique email: empty many times, a filled one once.
	linus := &customer{Name: "Linus"}
	must(t, Save(db, linus))
	if err := Save(db, &customer{Name: "Copy", Email: "ada@example.com"}); err == nil {
		t.Fatal("a second customer with the same e-mail was saved")
	}
	// The reference is kept through the renamed table.
	adaNow, err := Get[customer](db, ada.ID)
	must(t, err)
	if err := Delete(db, &adaNow); !errors.As(err, &inUse) {
		t.Fatalf("deleting a customer with purchases: %v", err)
	}
	var invalid *InvalidError
	if err := Save(db, &purchase{Number: "ORD-2", Customer: ref(999)}); !errors.As(err, &invalid) {
		t.Fatalf("a purchase of a customer that is not there: %v", err)
	}
	must(t, Delete(db, &p))
	must(t, Delete(db, &adaNow))
	everyone, err := All[customer](db)
	must(t, err)
	equal(t, len(everyone), 2)
}

func TestTheServerScenarioOnSqlite(t *testing.T) {
	migratesAndKeepsRecords(t, Sqlite, filepath.Join(t.TempDir(), "scenario.db"), "sqlite")
}

func onServer(t *testing.T, variable string, dialect Dialect, database string) {
	at := os.Getenv(variable)
	if at == "" {
		t.Skip(variable + " is not set")
	}
	migratesAndKeepsRecords(t, dialect, at, database)
}

func TestTheServerScenarioOnPostgres(t *testing.T) {
	onServer(t, "CLARITY_TEST_POSTGRES", Postgres, "postgres")
}

func TestTheServerScenarioOnMysql(t *testing.T) {
	onServer(t, "CLARITY_TEST_MYSQL", Mysql, "mysql")
}

func TestTheServerScenarioOnMssql(t *testing.T) {
	onServer(t, "CLARITY_TEST_MSSQL", Mssql, "mssql")
}

// Every server driver is in the runtime; an application keeps the ones it uses.
func TestEveryServerHasADriver(t *testing.T) {
	for _, d := range []Dialect{Postgres, Mysql, Mssql} {
		if _, ok := servers[d]; !ok {
			t.Errorf("no driver for %s", d.Label())
		}
	}
}
