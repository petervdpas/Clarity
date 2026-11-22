// MySQL through MySqlConnector: decimals are DECIMAL, dates DATE and
// DATETIME, bools BOOLEAN. MySQL cannot undo changes to tables, so a
// migration step that fails halfway keeps what ran before the failure, and
// says so.

using System.Data.Common;
using MySqlConnector;

namespace Clarity.Runtime;

internal sealed class MysqlDriver(MySqlConnection conn) : ServerDriver(conn)
{
    public override Dialect Dialect => Dialect.Mysql;

    internal static MysqlDriver Open(string location)
    {
        var conn = new MySqlConnection(Locations.Mysql(location));
        Store.Connect(conn, Dialect.Mysql);
        return new MysqlDriver(conn);
    }

    /// <summary>A row is referred to (1451, 1217), or refers to nothing (1452, 1216).</summary>
    protected override bool IsReference(DbException e) => e is MySqlException { Number: 1216 or 1217 or 1451 or 1452 };

    public override long Insert(string sql, IReadOnlyList<object?> parameters) => Run(() =>
    {
        using var command = (MySqlCommand)Command(sql, parameters);
        command.ExecuteNonQuery();
        return command.LastInsertedId;
    });
}
