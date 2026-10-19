// SQL Server through Microsoft.Data.SqlClient: decimals are DECIMAL, dates
// DATE and DATETIME2, bools BIT. A migration step runs in one transaction.

using System.Data.Common;
using Microsoft.Data.SqlClient;

namespace Clarity.Runtime;

internal sealed class MssqlDriver(SqlConnection conn) : ServerDriver(conn)
{
    /// <summary>A statement conflicts with a reference between tables.</summary>
    const int ForeignKey = 547;

    public override Dialect Dialect => Dialect.Mssql;

    internal static MssqlDriver Open(string location)
    {
        var conn = new SqlConnection(location);
        Store.Connect(conn, Dialect.Mssql);
        return new MssqlDriver(conn);
    }

    protected override string? ParameterName(int n) => Dialect.Mark(n);

    protected override bool IsReference(DbException e) => e is SqlException { Number: ForeignKey };
}
