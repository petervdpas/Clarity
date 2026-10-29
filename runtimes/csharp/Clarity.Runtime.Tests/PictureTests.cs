// Pictures in records (X2), against the cases every runtime shares
// (conformance/pictures.json), and kept in SQLite or in a file store. The
// wire cases are the web renderer's; Avalonia shows pictures without JSON.

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class Album : IRecord<Album>
{
    public static string TableName => "album";
    public static IReadOnlyList<string> ColumnNames { get; } = ["title", "cover", "photo"];
    public static IReadOnlyList<PictureColumn> Pictures { get; } = [new("cover", null), new("photo", "photos")];

    public long Id { get; set; }
    public string Title { get; set; } = "";
    public Picture? Cover { get; set; }
    public Picture? Photo { get; set; }

    public object?[] ToValues() => [Title, Sql.Picture(Cover), Sql.Picture(Photo)];

    public static Album FromValues(long id, object?[] values) => new()
    {
        Id = id,
        Title = Sql.GetText("title", values[0]),
        Cover = Sql.GetPicture("cover", values[1]),
        Photo = Sql.GetPicture("photo", values[2]),
    };
}

public class PictureTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "pictures.json"))).RootElement;

    static readonly byte[] Png = [0x89, (byte)'P', (byte)'N', (byte)'G', 0x0D, 0x0A, 0x1A, 0x0A];
    static readonly byte[] Jpg = [0xFF, 0xD8, 0xFF, 0xE0];

    [Fact]
    public void APictureIsToldByItsFirstBytes()
    {
        foreach (var kind in Cases().GetProperty("kinds").EnumerateArray())
        {
            var found = Pictures.KindOf(Convert.FromHexString(kind.GetProperty("bytes").GetString()!));
            Assert.NotNull(found);
            Assert.Equal(kind.GetProperty("extension").GetString(), found.Extension);
            Assert.Equal(kind.GetProperty("type").GetString(), found.MediaType);
        }
        foreach (var other in Cases().GetProperty("notPictures").EnumerateArray())
            Assert.Null(Pictures.KindOf(Convert.FromHexString(other.GetString()!)));
    }

    [Fact]
    public void AFileIsNamedByItsBytes()
    {
        foreach (var name in Cases().GetProperty("fileNames").EnumerateArray())
            Assert.Equal(name.GetProperty("name").GetString(), Pictures.FileName(Convert.FromHexString(name.GetProperty("bytes").GetString()!)));
        Assert.Throws<InvalidValueException>(() => Pictures.FileName("%PDF"u8.ToArray()));
    }

    [Fact]
    public void ReadingRecordsHoldsWhetherThereIsAPicture()
    {
        var presence = Cases().GetProperty("presence");
        foreach (var (word, dialect) in new[] { ("sqlite", Dialect.Sqlite), ("postgres", Dialect.Postgres), ("mysql", Dialect.Mysql), ("mssql", Dialect.Mssql) })
            Assert.Equal(presence.GetProperty(word).GetString(), Pictures.Presence(dialect, "cover"));
    }

    static string Scratch(string name)
    {
        var dir = Path.Combine(Path.GetTempPath(), $"clarity-pictures-cs-{name}-{Environment.ProcessId}");
        if (Directory.Exists(dir))
            Directory.Delete(dir, true);
        return dir;
    }

    static Db Albums(string store)
    {
        var db = Sqlite.Memory();
        db.Migrate([["CREATE TABLE album (id INTEGER PRIMARY KEY, title TEXT NOT NULL, cover BLOB, photo TEXT)"]]);
        db.AddStore("photos", store);
        return db;
    }

    [Fact]
    public void APictureIsKeptInTheDatabaseAndFetchedOnItsOwn()
    {
        var store = Scratch("db");
        using var cleanup = new Cleanup(store);
        using var db = Albums(store);
        var album = new Album { Title = "Blue Train", Cover = Picture.New(Png) };
        db.Save(album);
        var read = db.Get<Album>(album.Id);
        Assert.Same(Picture.Stored, read.Cover);
        Assert.Null(read.Photo);
        Assert.Equal(Png, db.PictureOf<Album>(album.Id, "cover"));

        // Saving the record again keeps the picture it has.
        read.Title = "Blue Train (mono)";
        db.Save(read);
        Assert.Equal(Png, db.PictureOf<Album>(album.Id, "cover"));

        // A new picture replaces it; none empties it.
        read.Cover = Picture.New(Jpg);
        db.Save(read);
        Assert.Equal(Jpg, db.PictureOf<Album>(album.Id, "cover"));
        read.Cover = null;
        db.Save(read);
        Assert.Null(db.PictureOf<Album>(album.Id, "cover"));
        Assert.Null(db.Get<Album>(album.Id).Cover);

        // Only a picture is kept.
        read.Cover = Picture.New("%PDF-1.7"u8.ToArray());
        Assert.Throws<InvalidValueException>(() => db.Save(read));
    }

    [Fact]
    public void APictureInAFileStoreIsAFileNamedByItsBytes()
    {
        var store = Scratch("store");
        using (var db = Albums(store))
        {
            var album = new Album { Title = "Kind of Blue", Photo = Picture.New(Jpg) };
            db.Save(album);
            Assert.Equal(Jpg, File.ReadAllBytes(Path.Combine(store, Pictures.FileName(Jpg))));
            Assert.Same(Picture.Stored, db.Get<Album>(album.Id).Photo);
            Assert.Equal(Jpg, db.PictureOf<Album>(album.Id, "photo"));

            // A path that leaves the store is refused.
            db.Batch("", $"UPDATE album SET photo = '../secret.png' WHERE id = {album.Id}");
            Assert.Throws<InvalidValueException>(() => db.PictureOf<Album>(album.Id, "photo"));
        }
        Directory.Delete(store, true);
    }

    /// <summary>
    /// Another record's picture, copied in code (album.cover = other.cover): one in hand is
    /// copied as it is, a stored one is read first, and one of a file store stays linked
    /// only in the same store.
    /// </summary>
    [Fact]
    public void APictureIsCopiedFromAnotherRecord()
    {
        var store = Scratch("copied");
        using var cleanup = new Cleanup(store);
        using var db = Albums(store);
        var first = new Album { Title = "Blue Train", Cover = Picture.New(Png), Photo = Picture.New(Jpg) };
        db.Save(first);
        var read = db.Get<Album>(first.Id);
        // A stored picture: its bytes, from the database or from the store.
        Assert.Equal(Picture.New(Png), db.CopiedPicture<Album>(read.Cover, read.Id, "cover", null, null));
        Assert.Equal(Picture.New(Jpg), db.CopiedPicture<Album>(read.Photo, read.Id, "photo", "photos", null));
        // One in hand, and none, as they are.
        Assert.Equal(Picture.New(Png), db.CopiedPicture<Album>(first.Cover, first.Id, "cover", null, null));
        Assert.Null(db.CopiedPicture<Album>(null, first.Id, "cover", null, null));
        // A file of the store: linked in the same store, else its bytes.
        var linked = Picture.InStore(Pictures.FileName(Jpg));
        Assert.Equal(linked, db.CopiedPicture<Album>(linked, first.Id, "photo", "photos", "photos"));
        Assert.Equal(Picture.New(Jpg), db.CopiedPicture<Album>(linked, first.Id, "photo", "photos", null));

        // Saved into the other field, the copy is kept there.
        var second = new Album { Title = "Copy", Photo = db.CopiedPicture<Album>(read.Cover, read.Id, "cover", null, "photos") };
        db.Save(second);
        Assert.Equal(Png, db.PictureOf<Album>(second.Id, "photo"));
    }

    [Fact]
    public void AStoreComesFromItsPlaceOrTheEnvironment()
    {
        using var db = Sqlite.Memory();
        Assert.Throws<StorageException>(() => db.AddStore("photos", "env:CLARITY_NO_SUCH_STORE"));
        var store = Scratch("env");
        db.AddStore("photos", store);
        Assert.True(Directory.Exists(store));
        Directory.Delete(store, true);
    }

    [Fact]
    public void APictureShowsAsADataUrl() =>
        Assert.Equal("data:image/png;base64,iVBORw0KGgo=", Pictures.DataUrl(Png));

    [Fact]
    public void SameBytesSamePicture() =>
        Assert.Equal(Picture.New([1, 2]), Picture.New([1, 2]));

}

/// Removes a scratch folder when a test is done with it.
sealed class Cleanup(string folder) : IDisposable
{
    public void Dispose()
    {
        if (Directory.Exists(folder))
            Directory.Delete(folder, true);
    }
}
