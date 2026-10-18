// MySQL through go-sql-driver/mysql: decimals are DECIMAL, dates DATE and
// DATETIME, bools BOOLEAN. The location is a URL
// (mysql://user:password@host:port/database) or the driver's own form.
// MySQL cannot undo changes to tables, so a migration step that fails
// halfway keeps what ran before the failure, and says so.

package clarity

import (
	"database/sql"
	"errors"
	"net/url"
	"strings"

	"github.com/go-sql-driver/mysql"
)

func init() { servers[Mysql] = openMysql }

// mysqlConfig is the driver's settings for a location.
func mysqlConfig(location string) (*mysql.Config, error) {
	rest, ok := strings.CutPrefix(location, "mysql://")
	if !ok {
		return mysql.ParseDSN(location)
	}
	u, err := url.Parse("mysql://" + rest)
	if err != nil {
		return nil, err
	}
	c := mysql.NewConfig()
	c.Net = "tcp"
	c.Addr = u.Host
	if u.Port() == "" {
		c.Addr = u.Hostname() + ":3306"
	}
	c.User = u.User.Username()
	c.Passwd, _ = u.User.Password()
	c.DBName = strings.TrimPrefix(u.Path, "/")
	return c, nil
}

func openMysql(location string) (driver, error) {
	c, err := mysqlConfig(location)
	if err != nil {
		return nil, &StorageError{What: "cannot connect to MySQL: " + err.Error(), Err: err}
	}
	c.ParseTime = true
	connector, err := mysql.NewConnector(c)
	if err != nil {
		return nil, &StorageError{What: "cannot connect to MySQL: " + err.Error(), Err: err}
	}
	d, err := connect(Mysql, sql.OpenDB(connector))
	if err != nil {
		return nil, err
	}
	// A row is referred to (1451, 1217), or refers to nothing (1452, 1216).
	isReference := func(err error) bool {
		var e *mysql.MySQLError
		if !errors.As(err, &e) {
			return false
		}
		switch e.Number {
		case 1216, 1217, 1451, 1452:
			return true
		}
		return false
	}
	d.isReference, d.value = isReference, asTime
	return d, nil
}
