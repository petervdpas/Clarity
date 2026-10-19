// Where a MySQL database is, written once in the project file and read the
// same by every stack: a URL becomes the driver's settings, the driver's own
// form stays as it is.

package mysql

import (
	"reflect"
	"testing"
)

func TestMysqlTakesAUrl(t *testing.T) {
	c, err := config("mysql://root:p%40ss@db.local:3307/sales")
	must(t, err)
	equal(t, []string{c.User, c.Passwd, c.Net, c.Addr, c.DBName}, []string{"root", "p@ss", "tcp", "db.local:3307", "sales"})
	c, err = config("mysql://shop@localhost/scratch")
	must(t, err)
	equal(t, []string{c.User, c.Passwd, c.Addr, c.DBName}, []string{"shop", "", "localhost:3306", "scratch"})
	// The driver's own form stays as it is.
	c, err = config("root:secret@tcp(localhost:3306)/scratch")
	must(t, err)
	equal(t, []string{c.User, c.Passwd, c.Addr, c.DBName}, []string{"root", "secret", "localhost:3306", "scratch"})
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
