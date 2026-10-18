// Where a server database is, written once in the project file and read the
// same by every stack. PostgreSQL (a URL or libpq's key=value words) and SQL
// Server (an ADO.NET string) go to their drivers as they are; a MySQL URL
// becomes the driver's settings.

package clarity

import "testing"

func TestMysqlTakesAUrl(t *testing.T) {
	c, err := mysqlConfig("mysql://root:p%40ss@db.local:3307/sales")
	must(t, err)
	equal(t, []string{c.User, c.Passwd, c.Net, c.Addr, c.DBName}, []string{"root", "p@ss", "tcp", "db.local:3307", "sales"})
	c, err = mysqlConfig("mysql://shop@localhost/scratch")
	must(t, err)
	equal(t, []string{c.User, c.Passwd, c.Addr, c.DBName}, []string{"shop", "", "localhost:3306", "scratch"})
	// The driver's own form stays as it is.
	c, err = mysqlConfig("root:secret@tcp(localhost:3306)/scratch")
	must(t, err)
	equal(t, []string{c.User, c.Passwd, c.Addr, c.DBName}, []string{"root", "secret", "localhost:3306", "scratch"})
}
