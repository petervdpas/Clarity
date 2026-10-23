// The SQL of queries per database, against the cases every stack's runtime
// shares (conformance/queries.json).

package clarity

import (
	"encoding/json"
	"fmt"
	"os"
	"reflect"
	"testing"

	"github.com/shopspring/decimal"
)

type queryCase struct {
	Name    string         `json:"name"`
	Table   string         `json:"table"`
	Key     string         `json:"key"`
	Columns []string       `json:"columns"`
	Query   map[string]any `json:"query"`
	Expect  map[string]struct {
		SQL    string `json:"sql"`
		Params []any  `json:"params"`
	} `json:"expect"`
}

var dialects = map[string]Dialect{"sqlite": Sqlite, "postgres": Postgres, "mysql": Mysql, "mssql": Mssql}

func one(value any) (string, any) {
	for k, v := range value.(map[string]any) {
		return k, v
	}
	panic("empty")
}

func caseValue(t *testing.T, value any) any {
	kind, inner := one(value)
	switch kind {
	case "text":
		return inner.(string)
	case "integer":
		return int64(inner.(float64))
	case "decimal":
		return decimal.RequireFromString(inner.(string))
	case "bool":
		return inner.(bool)
	case "date":
		d, err := ParseDate(inner.(string))
		if err != nil {
			t.Fatal(err)
		}
		return d
	case "null":
		return nil
	}
	t.Fatalf("unknown value %s", kind)
	return nil
}

func caseValues(t *testing.T, value any) []any {
	var out []any
	for _, v := range value.([]any) {
		out = append(out, caseValue(t, v))
	}
	return out
}

func caseOp(t *testing.T, value any) Op {
	ops := map[string]Op{"Eq": Eq, "NotEq": NotEq, "Lt": Lt, "LtEq": LtEq, "Gt": Gt, "GtEq": GtEq}
	op, ok := ops[value.(string)]
	if !ok {
		t.Fatalf("unknown op %v", value)
	}
	return op
}

func caseCond(t *testing.T, value any) Cond {
	kind, inner := one(value)
	field := func(name string) any { return inner.(map[string]any)[name] }
	pair := func() (Cond, Cond) {
		both := inner.([]any)
		return caseCond(t, both[0]), caseCond(t, both[1])
	}
	switch kind {
	case "compare":
		return Compare(field("column").(string), caseOp(t, field("op")), caseValue(t, field("value")))
	case "number":
		return Number(field("column").(string), caseOp(t, field("op")), caseValue(t, field("value")))
	case "contains":
		return Contains(field("column").(string), field("text").(string))
	case "holds":
		return Holds(field("column").(string), caseValue(t, field("value")))
	case "matches":
		var columns []string
		for _, c := range field("columns").([]any) {
			columns = append(columns, c.(string))
		}
		return Matches(field("table").(string), field("key").(string), columns, field("text").(string))
	case "oneOf":
		return OneOf(field("column").(string), caseValues(t, field("values")))
	case "oneOfNumbers":
		return OneOfNumbers(field("column").(string), caseValues(t, field("values")))
	case "isTrue":
		return IsTrue(inner.(string))
	case "isNone":
		return IsNone(inner.(string))
	case "and":
		l, r := pair()
		return And(l, r)
	case "or":
		l, r := pair()
		return Or(l, r)
	case "not":
		return Not(caseCond(t, inner))
	case "through":
		return Through(field("column").(string), field("table").(string), field("key").(string), caseCond(t, field("filter")))
	}
	t.Fatalf("unknown condition %s", kind)
	return nil
}

func caseQuery(t *testing.T, value map[string]any) Query {
	query := Query{}
	if filter, ok := value["filter"]; ok {
		query = query.Filter(caseCond(t, filter))
	}
	if order, ok := value["order"]; ok {
		for _, o := range order.([]any) {
			key := o.(map[string]any)
			descending, _ := key["descending"].(bool)
			numeric, _ := key["numeric"].(bool)
			through, hops := key["through"].([]any)
			switch {
			case hops:
				var path []Hop
				for _, h := range through {
					hop := h.(map[string]any)
					path = append(path, Hop{hop["column"].(string), hop["table"].(string), hop["key"].(string)})
				}
				query = query.OrderThrough(path, key["column"].(string), descending, numeric)
			case numeric:
				query = query.OrderNumber(key["column"].(string), descending)
			default:
				query = query.Order(key["column"].(string), descending)
			}
		}
	}
	return query
}

func TestEveryQueryCaseGivesItsSQLOnEveryDatabase(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/queries.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases struct {
		Cases []queryCase `json:"cases"`
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	if len(cases.Cases) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range cases.Cases {
		query := caseQuery(t, c.Query)
		for name, expect := range c.Expect {
			d := dialects[name]
			columns := d.Quote(c.Key)
			for _, column := range c.Columns {
				columns += ", " + d.Quote(column)
			}
			sql, params := query.SQL(d, c.Table, c.Key, columns)
			if sql != expect.SQL {
				t.Errorf("%s on %s:\n got %s\nwant %s", c.Name, name, sql, expect.SQL)
			}
			want := caseValues(t, expect.Params)
			if !sameParams(params, want) {
				t.Errorf("%s on %s: params %v, want %v", c.Name, name, params, want)
			}
		}
	}
}

func sameParams(got, want []any) bool {
	if len(got) != len(want) {
		return false
	}
	for i := range got {
		g, gok := got[i].(decimal.Decimal)
		w, wok := want[i].(decimal.Decimal)
		if gok && wok {
			if !g.Equal(w) {
				return false
			}
			continue
		}
		if !reflect.DeepEqual(got[i], want[i]) {
			fmt.Printf("%#v != %#v\n", got[i], want[i])
			return false
		}
	}
	return true
}
