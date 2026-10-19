// Data events: a record whose entity has them runs them around its save and
// delete, in one transaction with them, as on the other stacks.

package clarity_test

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/shopspring/decimal"

	. "github.com/petervdpas/Clarity/runtimes/go/clarity"
	// The server databases, turned on as an application turns them on.
	_ "github.com/petervdpas/Clarity/runtimes/go/mysql"
	_ "github.com/petervdpas/Clarity/runtimes/go/postgresql"
	_ "github.com/petervdpas/Clarity/runtimes/go/sqlserver"
)

// logLine is what the events of an account wrote, in order.
type logLine struct {
	ID   int64
	Text string
}

func (*logLine) Table() string     { return "log" }
func (*logLine) Columns() []string { return []string{"text"} }
func (l *logLine) GetID() int64    { return l.ID }
func (l *logLine) SetID(id int64)  { l.ID = id }
func (l *logLine) Values() []any   { return []any{l.Text} }
func (l *logLine) Load(id int64, v []any) error {
	r := NewReader(v)
	*l = logLine{id, r.Text(0, "text")}
	return r.Err()
}

type account struct {
	ID      int64
	Name    string
	Balance int64
}

func (*account) Table() string     { return "account" }
func (*account) Columns() []string { return []string{"name", "balance"} }
func (a *account) GetID() int64    { return a.ID }
func (a *account) SetID(id int64)  { a.ID = id }
func (a *account) Values() []any   { return []any{a.Name, a.Balance} }
func (a *account) Load(id int64, v []any) error {
	r := NewReader(v)
	*a = account{id, r.Text(0, "name"), r.Integer(1, "balance")}
	return r.Err()
}

func (a *account) BeforeSave(db *Db) error {
	if strings.TrimSpace(a.Name) == "" {
		return Refuse(PlainText("An account needs a name"))
	}
	a.Name = strings.TrimSpace(a.Name)
	return nil
}

func (a *account) AfterSave(db *Db) error {
	if err := Save(db, &logLine{Text: fmt.Sprintf("saved %s with %d", a.Name, a.Balance)}); err != nil {
		return err
	}
	if a.Balance < 0 {
		return Refuse(PlainText("An account cannot be in debt"))
	}
	return nil
}

func (a *account) BeforeDelete(db *Db) error {
	if a.Balance != 0 {
		return Refuse(NewText("{name} still holds {balance}").With("name", SlotText(a.Name)).With("balance", SlotInteger(a.Balance, "")))
	}
	return nil
}

func (a *account) AfterDelete(db *Db) error {
	return Save(db, &logLine{Text: "deleted " + a.Name})
}

// eventTables are the tables of the event tests, per database.
var eventTables = map[Dialect][]string{
	Sqlite: {
		"CREATE TABLE account (id INTEGER PRIMARY KEY, name TEXT NOT NULL, balance INTEGER NOT NULL)",
		"CREATE TABLE log (id INTEGER PRIMARY KEY, text TEXT NOT NULL)",
	},
	Postgres: {
		"CREATE TABLE account (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL, balance BIGINT NOT NULL)",
		"CREATE TABLE log (id BIGSERIAL PRIMARY KEY, text TEXT NOT NULL)",
	},
	Mysql: {
		"CREATE TABLE account (id BIGINT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(200) NOT NULL, balance BIGINT NOT NULL)",
		"CREATE TABLE log (id BIGINT AUTO_INCREMENT PRIMARY KEY, text VARCHAR(200) NOT NULL)",
	},
	Mssql: {
		"CREATE TABLE account (id BIGINT IDENTITY PRIMARY KEY, name NVARCHAR(200) NOT NULL, balance BIGINT NOT NULL)",
		"CREATE TABLE log (id BIGINT IDENTITY PRIMARY KEY, text NVARCHAR(200) NOT NULL)",
	},
}

// eachDb runs test on SQLite in memory, and on every server database a
// CLARITY_TEST_* variable names (see server_test.go), with empty tables.
func eachDb(t *testing.T, test func(t *testing.T, db *Db)) {
	databases := []struct {
		dialect Dialect
		at      string
	}{
		{Sqlite, ":memory:"},
		{Postgres, os.Getenv("CLARITY_TEST_POSTGRES")},
		{Mysql, os.Getenv("CLARITY_TEST_MYSQL")},
		{Mssql, os.Getenv("CLARITY_TEST_MSSQL")},
	}
	for _, d := range databases {
		t.Run(d.dialect.Label(), func(t *testing.T) {
			if d.at == "" {
				t.Skip("no " + d.dialect.Label() + " to test on")
			}
			db, err := Connect(Source{"", d.dialect, d.at})
			must(t, err)
			t.Cleanup(func() { db.Close() })
			for _, table := range []string{"account", "log", "clarity_schema"} {
				must(t, db.Batch("", "DROP TABLE IF EXISTS "+d.dialect.Quote(table)))
			}
			_, err = db.Migrate([][]string{eventTables[d.dialect]})
			must(t, err)
			test(t, db)
		})
	}
}

func logged(t *testing.T, db *Db) []string {
	lines, err := All[logLine](db)
	must(t, err)
	return names(lines, func(l logLine) string { return l.Text })
}

func refusal(t *testing.T, err error) string {
	t.Helper()
	var refused *RefusedError
	if !errors.As(err, &refused) {
		t.Fatalf("not refused: %v", err)
	}
	return refused.Error()
}

func TestEventsRunAroundASave(t *testing.T) {
	eachDb(t, func(t *testing.T, db *Db) {
		ada := &account{Name: "  Ada ", Balance: 10}
		must(t, Save(db, ada))
		// Before save changed the record that was stored.
		equal(t, ada.Name, "Ada")
		stored, err := Get[account](db, ada.ID)
		must(t, err)
		equal(t, stored.Name, "Ada")
		equal(t, logged(t, db), []string{"saved Ada with 10"})

		equal(t, refusal(t, Save(db, &account{})), "An account needs a name")
		all, _ := All[account](db)
		equal(t, len(all), 1)
	})
}

func TestAFailingEventKeepsNothing(t *testing.T) {
	eachDb(t, func(t *testing.T, db *Db) {
		debtor := &account{Name: "Grace", Balance: -5}
		equal(t, refusal(t, Save(db, debtor)), "An account cannot be in debt")
		// The record, and what after save wrote, are gone; the record is new again.
		equal(t, debtor.ID, int64(0))
		all, _ := All[account](db)
		equal(t, len(all), 0)
		equal(t, logged(t, db), []string{})

		// The database is usable afterwards.
		debtor.Balance = 0
		must(t, Save(db, debtor))
		equal(t, logged(t, db), []string{"saved Grace with 0"})
	})
}

func TestEventsRunAroundADelete(t *testing.T) {
	eachDb(t, func(t *testing.T, db *Db) {
		ada := &account{Name: "Ada", Balance: 10}
		must(t, Save(db, ada))
		equal(t, refusal(t, Delete(db, ada)), "Ada still holds 10")
		ada.Balance = 0
		must(t, Save(db, ada))
		must(t, Delete(db, ada))
		all, _ := All[account](db)
		equal(t, len(all), 0)
		equal(t, logged(t, db), []string{"saved Ada with 10", "saved Ada with 0", "deleted Ada"})
	})
}

// Text goes to the screen as the Rust runtime sends it: plain text as a
// string, text with slots as the text and its typed values.
func TestTextTravelsAsTheOtherRuntimesSendIt(t *testing.T) {
	plain, _ := json.Marshal(PlainText("Brace { yourself }"))
	equal(t, string(plain), `"Brace { yourself }"`)
	since := Date{2026, 10, 17}
	saved := NewText("Saved {customer.name} on {when}: {total}, {paid}, {kind}").
		With("customer.name", SlotText("Ada")).
		With("when", SlotDate(&since, "long")).
		With("total", SlotDecimal(decimal.RequireFromString("1250.50"), 2, "")).
		With("paid", SlotBool(true)).
		With("kind", SlotEnum("Business", "CustomerKind"))
	wire, _ := json.Marshal(saved)
	equal(t, string(wire), `{"text":"Saved {customer.name} on {when}: {total}, {paid}, {kind}","values":{"customer.name":{"kind":"text","value":"Ada"},"kind":{"kind":"enum","value":"Business","enum":"CustomerKind"},"paid":{"kind":"bool","value":true},"total":{"kind":"decimal","value":"1250.5","scale":2},"when":{"kind":"date","value":"2026-10-17","format":"long"}}}`)
	equal(t, saved.Show(), "Saved Ada on 2026-10-17: 1250.5, yes, Business")
	var back Text
	must(t, json.Unmarshal(wire, &back))
	equal(t, back.Show(), saved.Show())
}
