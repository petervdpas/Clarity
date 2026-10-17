// MySQL through MySqlConnector: decimals are DECIMAL, dates DATE and
// DATETIME, bools BOOLEAN. MySQL cannot undo changes to tables, so a
// migration step that fails halfway keeps what ran before the failure, and
// says so.

#if CLARITY_MYSQL
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

    public override void Apply(int version, IReadOnlyList<string> statements)
    {
        for (var i = 0; i < statements.Count; i++)
        {
            try
            {
                Batch(statements[i]);
            }
            catch (Exception e) when (e is StorageException or ReferenceFailure)
            {
                var what = e.InnerException?.Message ?? e.Message;
                throw new StorageException(i == 0 ? what : $"{what}; MySQL cannot undo the {i} statement(s) of step {version} that ran before it");
            }
        }
        Batch(Store.Applied(version));
    }
}
#endif
