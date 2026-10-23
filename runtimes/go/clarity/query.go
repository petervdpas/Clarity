// Queries: which records of an entity to read, and in which order.
// Generated code builds a Query from Clarity's find; this file turns it into
// SQL with parameters, in the dialect of the database the records are in.

package clarity

import (
	"fmt"
	"strings"
	"unicode/utf8"
)

// Op is a comparison between a column and a value.
type Op int

const (
	Eq Op = iota
	NotEq
	Lt
	LtEq
	Gt
	GtEq
)

func (o Op) sql() string {
	return [...]string{"=", "!=", "<", "<=", ">", ">="}[o]
}

// Cond is a condition on the columns of one table.
type Cond interface {
	sql(d Dialect, params *[]any) string
}

type compareCond struct {
	column  string
	op      Op
	value   any
	numeric bool
}

type containsCond struct{ column, text string }

type holdsCond struct {
	column string
	item   any
}

type matchesCond struct {
	table, key string
	columns    []string
	words      []string
}

type oneOfCond struct {
	column  string
	values  []any
	numeric bool
}

type isTrueCond struct{ column string }

type isNoneCond struct{ column string }

// Hop is a step through a reference: the column that holds it, and the
// table and key column of the records it refers to.
type Hop struct{ Column, Table, Key string }

type throughCond struct {
	hop   Hop
	inner Cond
}

type andCond struct{ left, right Cond }

type orCond struct{ left, right Cond }

type notCond struct{ inner Cond }

// Compare is column op value.
func Compare(column string, op Op, value any) Cond {
	return compareCond{column, op, value, false}
}

// Number compares a decimal or money column as a number.
func Number(column string, op Op, value any) Cond {
	return compareCond{column, op, value, true}
}

// Contains is the column's text containing text, ignoring case.
func Contains(column, text string) Cond { return containsCond{column, text} }

// Holds is a list field's JSON list holding the item, exactly
// (a.tags contains "live").
func Holds(column string, item any) Cond { return holdsCond{column, item} }

// Matches is full-text search (a matches x): the record of table holds
// every word of text, each as the start of a word, in the columns marked
// search, through the table's full-text index.
func Matches(table, key string, columns []string, text string) Cond {
	return matchesCond{table, key, columns, Words(text)}
}

// Words are the words of a search: runs of ASCII letters and digits and of
// any characters beyond ASCII. Everything else parts them.
func Words(text string) []string {
	return strings.FieldsFunc(text, func(r rune) bool {
		return r < utf8.RuneSelf && !(r >= '0' && r <= '9' || r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z')
	})
}

// mysqlShortest is the shortest word MySQL's full-text index keeps
// (innodb_ft_min_token_size).
const mysqlShortest = 3

func (c matchesCond) sql(d Dialect, params *[]any) string {
	if len(c.words) == 0 {
		return "1 = 1"
	}
	quoted := make([]string, len(c.columns))
	for i, column := range c.columns {
		quoted[i] = d.Quote(column)
	}
	var parts []string
	switch d {
	case Sqlite:
		query := make([]string, len(c.words))
		for i, w := range c.words {
			query[i] = `"` + w + `"*`
		}
		index := d.Quote(c.table + "__search")
		return d.Quote(c.key) + " IN (SELECT rowid FROM " + index + " WHERE " + index + " MATCH " + param(d, params, strings.Join(query, " AND "), false) + ")"
	case Postgres:
		query := make([]string, len(c.words))
		document := make([]string, len(quoted))
		for i, w := range c.words {
			query[i] = w + ":*"
		}
		for i, column := range quoted {
			document[i] = "coalesce(" + column + ", '')"
		}
		return "to_tsvector('simple', " + strings.Join(document, " || ' ' || ") + ") @@ to_tsquery('simple', " + param(d, params, strings.Join(query, " & "), false) + ")"
	case Mysql:
		// The index leaves out words shorter than 3 characters.
		columns := strings.Join(quoted, ", ")
		var long []string
		for _, w := range c.words {
			if utf8.RuneCountInString(w) >= mysqlShortest {
				long = append(long, "+"+w+"*")
			}
		}
		if len(long) > 0 {
			parts = append(parts, "MATCH ("+columns+") AGAINST ("+param(d, params, strings.Join(long, " "), false)+" IN BOOLEAN MODE)")
		}
		for _, w := range c.words {
			if utf8.RuneCountInString(w) < mysqlShortest {
				parts = append(parts, "CONCAT_WS(' ', "+columns+") REGEXP "+param(d, params, "(^|[^[:alnum:]])"+w, false))
			}
		}
	default:
		document := make([]string, len(quoted))
		for i, column := range quoted {
			document[i] = "ISNULL(" + column + ", N'')"
		}
		text := "(N' ' + " + strings.Join(document, " + N' ' + ") + ")"
		for _, w := range c.words {
			parts = append(parts, text+" LIKE "+param(d, params, "%[^0-9A-Za-z]"+w+"%", false))
		}
	}
	if len(parts) > 1 {
		return "(" + strings.Join(parts, " AND ") + ")"
	}
	return parts[0]
}

// OneOf is the column equal to one of the values.
func OneOf(column string, values []any) Cond { return oneOfCond{column, values, false} }

// OneOfNumbers is OneOf for a decimal or money column.
func OneOfNumbers(column string, values []any) Cond { return oneOfCond{column, values, true} }

// IsTrue is a bool column that is true.
func IsTrue(column string) Cond { return isTrueCond{column} }

// IsNone is a column with no value: an empty date or reference (none).
func IsNone(column string) Cond { return isNoneCond{column} }

// Through is cond on the record that column refers to, in table by key
// (a.artist.name contains x).
func Through(column, table, key string, cond Cond) Cond {
	return throughCond{Hop{column, table, key}, cond}
}

func And(left, right Cond) Cond { return andCond{left, right} }

func Or(left, right Cond) Cond { return orCond{left, right} }

func Not(inner Cond) Cond { return notCond{inner} }

func param(d Dialect, params *[]any, value any, numeric bool) string {
	*params = append(*params, value)
	mark := d.Mark(len(*params))
	if numeric {
		return d.number(mark)
	}
	return mark
}

// columnSQL is a column as a comparison reads it.
func columnSQL(d Dialect, column string, numeric bool) string {
	if numeric {
		return d.number(d.Quote(column))
	}
	return d.Quote(column)
}

func (c compareCond) sql(d Dialect, params *[]any) string {
	return columnSQL(d, c.column, c.numeric) + " " + c.op.sql() + " " + param(d, params, c.value, c.numeric)
}

func (c containsCond) sql(d Dialect, params *[]any) string {
	pattern := "%" + d.escapeLike(c.text) + "%"
	return d.contains(c.column, param(d, params, pattern, false))
}

func (c holdsCond) sql(d Dialect, params *[]any) string {
	return d.holds(c.column, param(d, params, d.item(c.item), false))
}

func (c oneOfCond) sql(d Dialect, params *[]any) string {
	marks := make([]string, len(c.values))
	for i, v := range c.values {
		marks[i] = param(d, params, v, c.numeric)
	}
	return columnSQL(d, c.column, c.numeric) + " IN (" + strings.Join(marks, ", ") + ")"
}

func (c isTrueCond) sql(d Dialect, _ *[]any) string { return d.isTrue(c.column) }

func (c isNoneCond) sql(d Dialect, _ *[]any) string { return d.Quote(c.column) + " IS NULL" }

// The inner condition names the columns of the referred table, which a
// subquery's own FROM resolves first, even for a table that refers to itself.
func (c throughCond) sql(d Dialect, params *[]any) string {
	return d.Quote(c.hop.Column) + " IN (SELECT " + d.Quote(c.hop.Key) + " FROM " + d.Quote(c.hop.Table) + " WHERE " + c.inner.sql(d, params) + ")"
}

func (c andCond) sql(d Dialect, params *[]any) string {
	left := c.left.sql(d, params)
	return "(" + left + " AND " + c.right.sql(d, params) + ")"
}

func (c orCond) sql(d Dialect, params *[]any) string {
	left := c.left.sql(d, params)
	return "(" + left + " OR " + c.right.sql(d, params) + ")"
}

func (c notCond) sql(d Dialect, params *[]any) string { return "NOT (" + c.inner.sql(d, params) + ")" }

// orderKey is a column to sort by, perhaps of a record the references in
// through lead to.
type orderKey struct {
	through             []Hop
	column              string
	descending, numeric bool
}

// sql is the key as SQL. Through references, each step is a subquery on its
// own alias (t1, t2, ...) that finds the referred record by the column of
// the step before.
func (o orderKey) sql(d Dialect, table string) string {
	direction := ""
	if o.descending {
		direction = " DESC"
	}
	if len(o.through) == 0 {
		return columnSQL(d, o.column, o.numeric) + direction
	}
	var step func(outer string, at int) string
	step = func(outer string, at int) string {
		if at == len(o.through) {
			column := outer + "." + d.Quote(o.column)
			if o.numeric {
				return d.number(column)
			}
			return column
		}
		hop := o.through[at]
		alias := d.Quote(fmt.Sprintf("t%d", at+1))
		return "(SELECT " + step(alias, at+1) + " FROM " + d.Quote(hop.Table) + " " + alias +
			" WHERE " + alias + "." + d.Quote(hop.Key) + " = " + outer + "." + d.Quote(hop.Column) + ")"
	}
	return step(d.Quote(table), 0) + direction
}

// Query is the records to read: a condition and an order. Without an order,
// records come in the order they were stored.
type Query struct {
	filter Cond
	order  []orderKey
}

func (q Query) Filter(cond Cond) Query {
	q.filter = cond
	return q
}

func (q Query) Order(column string, descending bool) Query {
	return q.OrderThrough(nil, column, descending, false)
}

// OrderNumber is Order for a decimal or money column.
func (q Query) OrderNumber(column string, descending bool) Query {
	return q.OrderThrough(nil, column, descending, true)
}

// OrderThrough is Order by a column of the record the references in through
// lead to (order by a.artist.name); numeric for a decimal or money column.
func (q Query) OrderThrough(through []Hop, column string, descending, numeric bool) Query {
	q.order = append(append([]orderKey(nil), q.order...), orderKey{through, column, descending, numeric})
	return q
}

// SQL is the SELECT statement for table in d, reading columns, and its
// parameters. Records that sort the same come in the order of key.
func (q Query) SQL(d Dialect, table, key, columns string) (string, []any) {
	var params []any
	sql := "SELECT " + columns + " FROM " + d.Quote(table)
	if q.filter != nil {
		sql += " WHERE " + q.filter.sql(d, &params)
	}
	keys := make([]string, 0, len(q.order)+1)
	for _, o := range q.order {
		keys = append(keys, o.sql(d, table))
	}
	keys = append(keys, d.Quote(key))
	return sql + " ORDER BY " + strings.Join(keys, ", "), params
}

// TextContains is Clarity's contains outside a query: text containing part,
// ignoring case.
func TextContains(text, part string) bool {
	return strings.Contains(strings.ToLower(text), strings.ToLower(part))
}
