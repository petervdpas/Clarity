// A record of an entity, as generated code describes it, and how its values
// are written and read. Each driver stores a value its own way: SQLite
// keeps decimals and dates as text, so no precision is lost; the server
// databases use their own types. Enums are stored by name.

package clarity

import (
	"fmt"
	"strconv"
	"time"

	"github.com/shopspring/decimal"
)

// Record is a record of an entity. Generated record types implement it on
// their pointer.
type Record interface {
	// Table is the table the records are stored in.
	Table() string
	// Columns are the stored columns, without the key, in the order of Values.
	Columns() []string
	GetID() int64
	SetID(id int64)
	Values() []any
	// Load fills the record from values in Columns order.
	Load(id int64, values []any) error
}

// Sourced is a record of a data source other than the default one.
type Sourced interface{ Source() string }

// Keyed is a record whose key column is not id.
type Keyed interface{ Key() string }

// The data events a record can have. A save or delete of a record with any
// of them runs with them in one transaction: when one fails, nothing of it
// is kept.
type (
	BeforeSaver   interface{ BeforeSave(db *Db) error }
	AfterSaver    interface{ AfterSave(db *Db) error }
	BeforeDeleter interface{ BeforeDelete(db *Db) error }
	AfterDeleter  interface{ AfterDelete(db *Db) error }
)

func sourceOf(r Record) string {
	if s, ok := r.(Sourced); ok {
		return s.Source()
	}
	return ""
}

func keyOf(r Record) string {
	if k, ok := r.(Keyed); ok {
		return k.Key()
	}
	return "id"
}

func hasEvents(r Record) bool {
	switch r.(type) {
	case BeforeSaver, AfterSaver, BeforeDeleter, AfterDeleter:
		return true
	}
	return false
}

// Reader reads the values of a row into a record's fields and keeps the
// first value that does not fit, as bufio.Scanner keeps its error.
type Reader struct {
	values []any
	err    error
}

func NewReader(values []any) *Reader { return &Reader{values: values} }

// Err is the first value that did not fit, or nil.
func (r *Reader) Err() error { return r.err }

func (r *Reader) fail(column string, value any) {
	if r.err == nil {
		r.err = &InvalidError{fmt.Sprintf("%s cannot be %v", column, value)}
	}
}

func (r *Reader) value(i int) any {
	if i < len(r.values) {
		return r.values[i]
	}
	return nil
}

func (r *Reader) Text(i int, column string) string {
	switch v := r.value(i).(type) {
	case string:
		return v
	case []byte:
		return string(v)
	case nil:
		return ""
	// Databases written by other programs may hold numbers in text columns.
	case int64:
		return strconv.FormatInt(v, 10)
	case float64:
		return strconv.FormatFloat(v, 'f', -1, 64)
	case decimal.Decimal:
		return v.String()
	default:
		r.fail(column, v)
		return ""
	}
}

func (r *Reader) Integer(i int, column string) int64 {
	n, ok := integer(r.value(i))
	if !ok {
		r.fail(column, r.value(i))
	}
	return n
}

func integer(value any) (int64, bool) {
	switch v := value.(type) {
	case int64:
		return v, true
	case int32:
		return int64(v), true
	case int:
		return int64(v), true
	case nil:
		return 0, true
	case bool:
		if v {
			return 1, true
		}
		return 0, true
	case []byte:
		n, err := strconv.ParseInt(string(v), 10, 64)
		return n, err == nil
	case string:
		// Empty text is no number: it reads as the default.
		if v == "" {
			return 0, true
		}
		n, err := strconv.ParseInt(v, 10, 64)
		return n, err == nil
	case float64:
		return int64(v), v == float64(int64(v))
	case decimal.Decimal:
		return v.IntPart(), v.Equal(decimal.NewFromInt(v.IntPart()))
	default:
		return 0, false
	}
}

func (r *Reader) Decimal(i int, column string) decimal.Decimal {
	switch v := r.value(i).(type) {
	case decimal.Decimal:
		return v
	case string:
		d, err := decimal.NewFromString(v)
		if err != nil {
			r.fail(column, v)
		}
		return d
	case []byte:
		d, err := decimal.NewFromString(string(v))
		if err != nil {
			r.fail(column, v)
		}
		return d
	case int64:
		return decimal.NewFromInt(v)
	case float64:
		return decimal.NewFromFloat(v)
	case nil:
		return decimal.Zero
	default:
		r.fail(column, v)
		return decimal.Zero
	}
}

func (r *Reader) Bool(i int, column string) bool {
	if b, ok := r.value(i).(bool); ok {
		return b
	}
	return r.Integer(i, column) != 0
}

// Date reads a date; nil when there is none. A date can be written with a
// time of day after it.
func (r *Reader) Date(i int, column string) *Date {
	switch v := r.value(i).(type) {
	case nil:
		return nil
	case time.Time:
		d := DateOf(v)
		return &d
	case string, []byte:
		text := fmt.Sprint(r.Text(i, column))
		if len(text) > 10 {
			text = text[:10]
		}
		d, err := ParseDate(text)
		if err != nil {
			r.fail(column, v)
			return nil
		}
		return &d
	default:
		r.fail(column, v)
		return nil
	}
}

func (r *Reader) DateTime(i int, column string) *DateTime {
	switch v := r.value(i).(type) {
	case nil:
		return nil
	case time.Time:
		d := DateTimeOf(v)
		return &d
	case string, []byte:
		d, err := ParseDateTime(r.Text(i, column))
		if err != nil {
			r.fail(column, v)
			return nil
		}
		return &d
	default:
		r.fail(column, v)
		return nil
	}
}

// Reference reads the id a reference holds; nil when it is not set.
func (r *Reader) Reference(i int, column string) *int64 {
	if r.value(i) == nil {
		return nil
	}
	n := r.Integer(i, column)
	return &n
}

// Name reads an enum value by its name: one of names, or the first when the
// value is empty or missing.
func (r *Reader) Name(i int, column string, names ...string) string {
	text := r.Text(i, column)
	if text == "" && len(names) > 0 {
		return names[0]
	}
	for _, n := range names {
		if n == text {
			return text
		}
	}
	r.fail(column, text)
	return ""
}

// Show is a value as text, the way "text" + value shows it in Clarity.
func Show(value any) string {
	switch v := value.(type) {
	case string:
		return v
	case int64:
		return strconv.FormatInt(v, 10)
	case decimal.Decimal:
		return v.String()
	case bool:
		if v {
			return "yes"
		}
		return "no"
	case *Date:
		if v == nil {
			return ""
		}
		return v.String()
	case *DateTime:
		if v == nil {
			return ""
		}
		return fmt.Sprintf("%s %02d:%02d", v.Date, v.Hour, v.Minute)
	case fmt.Stringer:
		return v.String()
	default:
		return fmt.Sprint(v)
	}
}

// RefTo is a reference to the record with id, as a reference field holds it.
func RefTo(id int64) *int64 { return &id }

// OneOfDecimals is Clarity's in for decimals: whether value equals one of options.
func OneOfDecimals(value decimal.Decimal, options ...decimal.Decimal) bool {
	for _, o := range options {
		if value.Equal(o) {
			return true
		}
	}
	return false
}
