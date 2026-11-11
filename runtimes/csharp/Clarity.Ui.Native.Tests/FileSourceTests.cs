// A data source that is a file in a format (docs/FORMATS-PLAN.md): a CSV
// read into a table its entity finds, saves and deletes in, each change
// written back to the file with its own column order and the columns no
// entity maps kept. The same case as runtimes/rust/tests/file_source.rs.

using Clarity.Formats;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Ui.Native.Tests;

sealed record Login : IRecord<Login>
{
    public static string SourceName => "logins";
    public static string TableName => "login";
    public static IReadOnlyList<string> ColumnNames { get; } = ["Site", "visits", "paid"];
    public long Id { get; set; }
    public string Site { get; set; } = "";
    public long Visits { get; set; }
    public bool Paid { get; set; }
    public object?[] ToValues() => [Site, Visits, Paid];
    public static Login FromValues(long id, object?[] values) =>
        new() { Id = id, Site = Sql.GetText("site", values[0]), Visits = Sql.GetInteger("visits", values[1]), Paid = Sql.GetBoolean("paid", values[2]) };
}

sealed record FeedOrder : IRecord<FeedOrder>
{
    public static string SourceName => "feed";
    public static string TableName => "order";
    public static IReadOnlyList<string> ColumnNames { get; } = ["number"];
    public long Id { get; set; }
    public string Number { get; set; } = "";
    public object?[] ToValues() => [Number];
    public static FeedOrder FromValues(long id, object?[] values) => new() { Id = id, Number = Sql.GetText("number", values[0]) };
}

sealed record FeedLine : IRecord<FeedLine>
{
    public static string SourceName => "feed";
    public static string TableName => "line";
    public static IReadOnlyList<string> ColumnNames { get; } = ["order", "product"];
    public long Id { get; set; }
    public long Order { get; set; }
    public string Product { get; set; } = "";
    public object?[] ToValues() => [Order, Product];
    public static FeedLine FromValues(long id, object?[] values) => new() { Id = id, Order = Sql.GetInteger("order", values[0]), Product = Sql.GetText("product", values[1]) };
}

public sealed class FileSourceTests
{
    const string Fields = """[{"name":"Site","kind":"text"},{"name":"visits","kind":"integer"},{"name":"paid","kind":"bool"}]""";

    static FileSource Source(string path) => new("logins", "csv", path, """{"separator":";"}""", "login", "id", Fields, Readonly: false);

    [Fact]
    public void A_csv_file_is_a_data_source_written_back_after_each_change()
    {
        Sqlite.Use();
        var folder = Directory.CreateTempSubdirectory("clarity-file-source-");
        try
        {
            var file = Path.Combine(folder.FullName, "logins.csv");
            File.WriteAllText(file, "Site;Note;visits;paid\r\nexample.com;keep me;3;true\r\nshop.nl;;1;false\r\n");
            using (var db = Db.Connect())
            {
                db.AddFile(Source(file));

                var all = db.All<Login>();
                Assert.Equal(new List<Login> { new Login { Id = 1, Site = "example.com", Visits = 3, Paid = true }, new Login { Id = 2, Site = "shop.nl", Visits = 1, Paid = false } }, all);

                all[0].Visits = 4;
                db.Save(all[0]);
                var added = new Login { Site = "a;b", Visits = 0, Paid = true };
                db.Save(added);
                db.Delete(all[1]);
                Assert.Equal("Site;Note;visits;paid\r\nexample.com;keep me;4;true\r\n\"a;b\";;0;true\r\n", File.ReadAllText(file));
                Assert.False(File.Exists(file + ".new"), "the new file replaced the old one");
            }

            // A column the entity maps that the file lacks stops the program as it opens.
            File.WriteAllText(file, "Site;paid\r\n");
            using (var db = Db.Connect())
            {
                var error = Assert.Throws<StorageException>(() => db.AddFile(Source(file))).Message;
                Assert.Contains("data source logins: the CSV has no column 'visits'", error);
            }

            // A file not there yet starts empty, and the first save makes it and its folder.
            var fresh = Path.Combine(folder.FullName, "fresh", "logins.csv");
            using (var db = Db.Connect())
            {
                db.AddFile(Source(fresh));
                Assert.Empty(db.All<Login>());
                db.Save(new Login { Site = "new.nl", Visits = 0, Paid = true });
                Assert.Equal("Site;visits;paid\r\nnew.nl;0;true\r\n", File.ReadAllText(fresh));
                // A readonly source needs its file.
                var error = Assert.Throws<StorageException>(() => db.AddFile(Source(Path.Combine(folder.FullName, "missing.csv")) with { Readonly = true })).Message;
                Assert.Contains("data source logins: cannot read", error);
            }
        }
        finally
        {
            folder.Delete(recursive: true);
        }
    }

    const string OrdersXml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Tea\"/><line product=\"Coffee\"/></lines></order>\n    <order number=\"A-2\"><lines><line product=\"Milk\"/></lines></order>\n  </orders>\n</shop>\n";
    const string OrdersOptions = """{"at":"shop/orders/order","fields":{"number":"@number"},"nested":{"lines":{"at":"lines/line","fields":{"product":"@product"},"parent":"order"}}}""";
    const string OrdersFields = """[{"name":"number","kind":"text"},{"name":"lines","kind":"records","entity":"Line","fields":[{"name":"product","kind":"text"}],"base":{},"table":"line","key":"id"}]""";

    [Fact]
    public void A_file_source_holds_the_records_inside_its_records_in_their_own_table()
    {
        Sqlite.Use();
        var folder = Directory.CreateTempSubdirectory("clarity-file-source-nested-");
        try
        {
            var file = Path.Combine(folder.FullName, "orders.xml");
            File.WriteAllText(file, OrdersXml);
            using var db = Db.Connect();
            db.AddFile(new FileSource("feed", "xml", file, OrdersOptions, "order", "id", OrdersFields, Readonly: false));

            Assert.Equal(new List<FeedOrder> { new() { Id = 1, Number = "A-1" }, new() { Id = 2, Number = "A-2" } }, db.All<FeedOrder>());
            var lines = db.All<FeedLine>();
            Assert.Equal(new List<FeedLine> { new() { Id = 1, Order = 1, Product = "Tea" }, new() { Id = 2, Order = 1, Product = "Coffee" }, new() { Id = 3, Order = 2, Product = "Milk" } }, lines);

            db.Save(new FeedLine { Order = 2, Product = "Juice" });
            db.Delete(lines[0]);
            Assert.Equal(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Coffee\"/></lines></order>\n    <order number=\"A-2\"><lines><line product=\"Milk\"/><line product=\"Juice\"/></lines></order>\n  </orders>\n</shop>\n",
                File.ReadAllText(file));
        }
        finally
        {
            folder.Delete(recursive: true);
        }
    }
}
