// A program checks its database's migration steps at start and does not migrate
// (docs/ledger/migrations.md): behind or ahead, it stops with one English line
// (conformance/migrate-check.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class MigrateCheckTests
{
    [Fact]
    public void ADatabaseIsCheckedAsTheCasesSay()
    {
        Sqlite.Use();
        using var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "migrate-check.json")));
        foreach (var c in file.RootElement.GetProperty("cases").EnumerateArray())
        {
            var source = c.GetProperty("source").GetString()!;
            using var db = Db.Connect((source, Dialect.Sqlite, ":memory:"));
            var had = c.GetProperty("had").GetInt32();
            db.ApplySteps(source, [.. Enumerable.Range(0, had).Select(n => new[] { $"CREATE TABLE t{n} (id INTEGER PRIMARY KEY)" })]);
            var line = c.GetProperty("line");
            var steps = c.GetProperty("steps").GetInt32();
            if (line.ValueKind == JsonValueKind.Null)
                db.CheckSteps(source, steps);
            else
                Assert.Equal(line.GetString(), Assert.Throws<StepsException>(() => db.CheckSteps(source, steps)).Message);
        }
    }
}
