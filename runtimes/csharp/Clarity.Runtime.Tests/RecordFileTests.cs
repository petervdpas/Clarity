// Files in records (U1c), against the cases every runtime shares
// (conformance/record-files.json), and kept in SQLite or in a file store.

using System.Text;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class Product : IRecord<Product>
{
    public static string TableName => "product";
    public static IReadOnlyList<string> ColumnNames { get; } = ["manual", "manual_name", "sheet", "sheet_name"];
    public static IReadOnlyList<PictureColumn> Pictures { get; } = [new("manual", null, "manual_name"), new("sheet", "documents", "sheet_name")];
    public long Id { get; set; }
    public RecordFile? Manual { get; set; }
    public RecordFile? Sheet { get; set; }
    public object?[] ToValues() => [Sql.FileContent(Manual), Sql.FileName(Manual), Sql.FileContent(Sheet), Sql.FileName(Sheet)];
    public static Product FromValues(long id, object?[] values) => new()
    {
        Id = id,
        Manual = Sql.GetFile("manual", values[0], values[1]),
        Sheet = Sql.GetFile("sheet", values[2], values[3]),
    };
}

public class RecordFileTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "record-files.json"))).RootElement;

    [Fact]
    public void AFileInAStoreIsKeptUnderItsNameInAFolderOfItsContents()
    {
        foreach (var c in Cases().GetProperty("storePaths").EnumerateArray())
            Assert.Equal(c.GetProperty("path").GetString(), RecordFiles.StorePath(Encoding.UTF8.GetBytes(c.GetProperty("text").GetString()!), c.GetProperty("chosen").GetString()));
        foreach (var c in Cases().GetProperty("names").EnumerateArray())
            Assert.Equal(c.GetProperty("name").GetString(), RecordFiles.NameOf(c.GetProperty("path").GetString()!));
    }

    [Fact]
    public void ACopyGoesToTheDownloadsFolderUnderAFreeName()
    {
        foreach (var c in Cases().GetProperty("downloads").EnumerateArray())
        {
            var env = c.GetProperty("env");
            var found = RecordFiles.DownloadsIn(c.GetProperty("os").GetString()!, n => env.TryGetProperty(n, out var v) ? v.GetString() : null);
            Assert.Equal(c.GetProperty("folder").ValueKind == JsonValueKind.Null ? null : c.GetProperty("folder").GetString(), found);
        }
        foreach (var c in Cases().GetProperty("copies").EnumerateArray())
        {
            var taken = c.GetProperty("taken").EnumerateArray().Select(t => t.GetString()).ToList();
            Assert.Equal(c.GetProperty("saved").GetString(), RecordFiles.FreeName(c.GetProperty("name").GetString()!, taken.Contains));
        }
    }

    [Fact]
    public void AFileIsKeptWithItsNameInTheDatabaseOrItsStore()
    {
        var store = Path.Combine(Path.GetTempPath(), $"clarity-record-files-{Environment.ProcessId}");
        Directory.CreateDirectory(store);
        using var db = Sqlite.Memory();
        db.Migrate([["CREATE TABLE product (id INTEGER PRIMARY KEY, manual BLOB, manual_name TEXT NOT NULL DEFAULT '', sheet TEXT, sheet_name TEXT NOT NULL DEFAULT '')"]]);
        db.AddStore("documents", store);
        var pdf = "%PDF-1.7 not a picture"u8.ToArray();
        var product = new Product { Manual = new("manual.pdf", Picture.New(pdf)), Sheet = new("sheet.pdf", Picture.New(pdf)) };
        db.Save(product);
        var read = db.Get<Product>(product.Id);
        Assert.Equal("manual.pdf", read.Manual?.Name);
        Assert.Equal("sheet.pdf", read.Sheet?.Name);
        Assert.Equal(pdf, db.PictureOf<Product>(product.Id, "manual"));
        Assert.Equal(pdf, File.ReadAllBytes(Path.Combine(store, RecordFiles.StorePath(pdf, "sheet.pdf"))));
        read.Manual = null;
        db.Save(read);
        Assert.Null(db.Get<Product>(product.Id).Manual);

        // Copied to another record (product.manual = other.sheet): with its name, the bytes read when stored.
        var again = db.Get<Product>(product.Id);
        var copied = db.CopiedFile<Product>(again.Sheet, again.Id, "sheet", "documents", null);
        Assert.Equal("sheet.pdf", copied?.Name);
        Assert.Equal(Picture.New(pdf), copied?.Content);
        Assert.Null(db.CopiedFile<Product>(again.Manual, again.Id, "manual", null, null));
        var other = new Product { Manual = copied };
        db.Save(other);
        Assert.Equal(pdf, db.PictureOf<Product>(other.Id, "manual"));
        Directory.Delete(store, recursive: true);
    }
}
