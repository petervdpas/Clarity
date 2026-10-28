// The files of a file store (X2), against the cases every runtime shares
// (conformance/files.json): listed, read and written as text, and a file as
// a record's picture.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class FileTests : IDisposable
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "files.json"))).RootElement;

    readonly string root = Path.Combine(Path.GetTempPath(), $"clarity-files-cs-{Guid.NewGuid():N}");
    readonly Db db = Sqlite.Memory();

    public FileTests()
    {
        foreach (var file in Cases().GetProperty("tree").EnumerateArray())
        {
            var path = Path.Combine(root, file.GetProperty("path").GetString()!);
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            File.WriteAllText(path, file.GetProperty("text").GetString());
            var at = DateTime.ParseExact(file.GetProperty("modified").GetString()!, "yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture);
            File.SetLastWriteTimeUtc(path, DateTime.SpecifyKind(at, DateTimeKind.Utc));
        }
        db.AddStore("photos", root);
    }

    public void Dispose()
    {
        db.Dispose();
        Directory.Delete(root, true);
    }

    [Fact]
    public void AStoreListsItsFilesAsEveryRuntimeDoes()
    {
        var listed = db.Files("photos").Select(f => new { f.Path, f.Name, f.Extension, f.Size, Modified = f.Modified?.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture) });
        var expected = Cases().GetProperty("listing").EnumerateArray().Select(f => new
        {
            Path = f.GetProperty("path").GetString()!,
            Name = f.GetProperty("name").GetString()!,
            Extension = f.GetProperty("extension").GetString()!,
            Size = f.GetProperty("size").GetInt64(),
            Modified = f.GetProperty("modified").GetString(),
        });
        Assert.Equal(expected, listed);
        Assert.Throws<StorageException>(() => db.Files("films"));
    }

    [Fact]
    public void BytesAreReadAndWrittenAsTheyAre()
    {
        foreach (var c in Cases().GetProperty("bytes").EnumerateArray())
        {
            var path = c.GetProperty("path").GetString()!;
            var bytes = Convert.FromBase64String(c.GetProperty("base64").GetString()!);
            db.WriteBytes("photos", path, bytes);
            var file = db.Files("photos").Single(f => f.Path == path);
            Assert.Equal(c.GetProperty("size").GetInt64(), file.Size);
            Assert.Equal(bytes, db.ReadBytes(file));
        }
    }

    [Fact]
    public void TextIsReadAndWrittenInsideTheStore()
    {
        var notes = db.Files("photos").Single(f => f.Name == "notes.txt");
        Assert.Equal("Hello\nworld\n", db.ReadText(notes));
        db.WriteText("photos", "exports/titles.txt", "Blue Train\n");
        Assert.Equal("Blue Train\n", File.ReadAllText(Path.Combine(root, "exports", "titles.txt")));
        foreach (var path in Cases().GetProperty("refused").EnumerateArray())
            Assert.Throws<InvalidValueException>(() => db.WriteText("photos", path.GetString()!, "x"));
    }

    [Fact]
    public void AFileBecomesAPictureLinkedInItsOwnStore()
    {
        byte[] png = [0x89, (byte)'P', (byte)'N', (byte)'G', 0x0D, 0x0A, 0x1A, 0x0A];
        Directory.CreateDirectory(Path.Combine(root, "covers"));
        File.WriteAllBytes(Path.Combine(root, "covers", "cover.png"), png);
        var cover = db.Files("photos").Single(f => f.Name == "cover.png");
        Assert.Equal(Picture.InStore("covers/cover.png"), db.PictureFrom(cover, "photos"));
        Assert.Equal("covers/cover.png", Sql.Picture(Picture.InStore("covers/cover.png")));
        Assert.Equal(Picture.New(png), db.PictureFrom(cover, null));
        var notes = db.Files("photos").Single(f => f.Name == "notes.txt");
        Assert.Throws<InvalidValueException>(() => db.PictureFrom(notes, null));
        Assert.Equal(png, db.StoreFile("photos", "covers/cover.png"));
        Assert.Throws<InvalidValueException>(() => db.StoreFile("photos", "../x.png"));
    }
}
