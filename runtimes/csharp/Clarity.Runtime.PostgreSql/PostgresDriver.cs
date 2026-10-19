// PostgreSQL through Npgsql: decimals are NUMERIC, dates DATE and
// TIMESTAMP, bools BOOLEAN. A migration step runs in one transaction, so a
// step that fails leaves the database as it was.

using System.Data.Common;
using Npgsql;

namespace Clarity.Runtime;

internal sealed class PostgresDriver(NpgsqlConnection conn) : ServerDriver(conn)
{
    /// <summary>foreign_key_violation.</summary>
    const string ForeignKey = "23503";

    public override Dialect Dialect => Dialect.Postgres;

    internal static PostgresDriver Open(string location)
    {
        var conn = new NpgsqlConnection(Locations.Postgres(location));
        Store.Connect(conn, Dialect.Postgres);
        return new PostgresDriver(conn);
    }

    protected override bool IsReference(DbException e) => e is PostgresException { SqlState: ForeignKey };
}
