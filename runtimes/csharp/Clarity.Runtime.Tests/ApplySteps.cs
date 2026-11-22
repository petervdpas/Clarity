// A test's database set up as clarity-migrate leaves it (docs/ledger/migrations.md):
// the program does not migrate, it checks (Db.CheckSteps).

using Clarity.Runtime;

namespace Clarity.Runtime.Tests;

static class TestSteps
{
    /// <summary>
    /// Runs the steps of <paramref name="source"/> its database has not had yet and
    /// counts each in clarity_schema, with references unchecked on SQLite while
    /// they run (a step may build a table again that others refer to); gives how
    /// many ran.
    /// </summary>
    public static int ApplySteps(this Db db, string source, IReadOnlyList<string[]> steps)
    {
        var dialect = db.DialectOf(source);
        db.Batch(source, dialect.VersionTable());
        var done = (int)Sql.GetInteger("version", db.Rows(source, "SELECT MAX(version) FROM clarity_schema", []).FirstOrDefault()?[0]);
        var sqlite = dialect == Dialect.Sqlite;
        if (sqlite)
            db.Batch(source, "PRAGMA foreign_keys = OFF");
        try
        {
            for (var i = done; i < steps.Count; i++)
            {
                foreach (var statement in steps[i])
                    db.Batch(source, statement);
                db.Batch(source, $"INSERT INTO clarity_schema (version) VALUES ({i + 1})");
            }
        }
        finally
        {
            if (sqlite)
                db.Batch(source, "PRAGMA foreign_keys = ON");
        }
        return Math.Max(0, steps.Count - done);
    }

    public static int ApplySteps(this Db db, IReadOnlyList<string[]> steps) => db.ApplySteps("", steps);
}
