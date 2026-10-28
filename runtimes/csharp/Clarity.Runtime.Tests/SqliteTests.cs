// SQLite is a package of its own (Clarity.Runtime.Sqlite), as each server
// database is: a program served to a browser carries no database driver,
// and the server or desktop program that keeps the data turns it on.

using System.Xml.Linq;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class SqliteTests
{
    /// <summary>The runtime's folder, the one with Clarity.slnx, above the tests' output.</summary>
    static string Runtime()
    {
        var at = new DirectoryInfo(AppContext.BaseDirectory);
        while (!File.Exists(Path.Combine(at.FullName, "Clarity.slnx")))
            at = at.Parent!;
        return at.FullName;
    }

    [Fact]
    public void The_core_carries_no_database_driver()
    {
        var project = XDocument.Load(Path.Combine(Runtime(), "Clarity.Runtime", "Clarity.Runtime.csproj"));
        var packages = project.Descendants("PackageReference").Select(p => (string)p.Attribute("Include")!).ToList();
        Assert.DoesNotContain("Microsoft.Data.Sqlite", packages);
        var used = typeof(Db).Assembly.GetReferencedAssemblies().Select(a => a.Name!).ToList();
        Assert.DoesNotContain(used, a => a.StartsWith("Microsoft.Data.Sqlite") || a.StartsWith("SQLitePCL"));
        Assert.Equal("Clarity.Runtime.Sqlite", typeof(Sqlite).Assembly.GetName().Name);
    }

    [Fact]
    public void A_database_opens_once_sqlite_is_turned_on()
    {
        Sqlite.Use();
        var dir = Path.Combine(Path.GetTempPath(), "clarity-sqlite-" + Guid.NewGuid().ToString("N"));
        try
        {
            using var db = Db.Connect(("", Dialect.Sqlite, Path.Combine(dir, "app.db")));
            Assert.True(File.Exists(Path.Combine(dir, "app.db")));
            using var memory = Sqlite.Memory();
            using var file = Sqlite.Open(Path.Combine(dir, "other.db"));
        }
        finally
        {
            if (Directory.Exists(dir))
                Directory.Delete(dir, true);
        }
    }
}
