// Where a server database is, written once in the project file and read the
// same by every stack: the Rust runtime takes PostgreSQL as a URL or as
// libpq's key=value words, MySQL as a URL and SQL Server as an ADO.NET
// string; the C# runtime reads the same and hands ADO.NET its own form.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class LocationTests
{
    [Fact]
    public void PostgresTakesAUrlOrKeyValueWords()
    {
        Assert.Equal(
            "Host=db.local;Port=5433;Username=shop;Password=p@ss word;Database=sales;SSL Mode=Require",
            Locations.Postgres("postgres://shop:p%40ss%20word@db.local:5433/sales?sslmode=require"));
        Assert.Equal("Host=localhost;Username=postgres;Database=scratch", Locations.Postgres("postgresql://postgres@localhost/scratch"));
        Assert.Equal(
            "Host=localhost;Port=5432;Username=postgres;Password=it's;Database=scratch",
            Locations.Postgres("host=localhost port=5432 user=postgres password='it\\'s' dbname=scratch"));
        // ADO.NET's own form stays as it is.
        Assert.Equal("Host=x;Database=y", Locations.Postgres("Host=x;Database=y"));
    }

    [Fact]
    public void MysqlTakesAUrl()
    {
        Assert.Equal("Server=localhost;Port=3306;User ID=root;Password=secret;Database=scratch", Locations.Mysql("mysql://root:secret@localhost:3306/scratch"));
        Assert.Equal("Server=x;Database=y", Locations.Mysql("Server=x;Database=y"));
    }
}
