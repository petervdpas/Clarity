// SQL Server through go-mssqldb: decimals are DECIMAL, dates DATE and
// DATETIME2, bools BIT. The location is an ADO.NET connection string or a
// sqlserver:// URL, as the driver reads them. A migration step runs in one
// transaction.

package clarity

import (
	"database/sql"
	"errors"

	"github.com/golang-sql/civil"
	mssql "github.com/microsoft/go-mssqldb"
)

// mssqlForeignKey is a statement that conflicts with a reference between tables.
const mssqlForeignKey = 547

func init() { servers[Mssql] = openMssql }

// mssqlValue is a date as DATE and a date and time as DATETIME2, not as a
// moment with a time zone.
func mssqlValue(value any) any {
	switch v := value.(type) {
	case Date:
		return civil.Date{Year: v.Year, Month: v.Month, Day: v.Day}
	case DateTime:
		return civil.DateTime{
			Date: civil.Date{Year: v.Year, Month: v.Month, Day: v.Day},
			Time: civil.Time{Hour: v.Hour, Minute: v.Minute, Second: v.Second},
		}
	}
	return value
}

func openMssql(location string) (driver, error) {
	connector, err := mssql.NewConnector(location)
	if err != nil {
		return nil, &StorageError{What: "cannot connect to SQL Server: " + err.Error(), Err: err}
	}
	d, err := connect(Mssql, sql.OpenDB(connector))
	if err != nil {
		return nil, err
	}
	isReference := func(err error) bool {
		var e mssql.Error
		return errors.As(err, &e) && e.Number == mssqlForeignKey
	}
	d.isReference, d.value, d.returning, d.undoes = isReference, mssqlValue, true, true
	return d, nil
}
