// The files of a file store, which code reaches through `files f in store`,
// `read text of f` and `write t to store "path"`: only folders the project
// settings name. A file found can become a record's picture: in the
// field's own store by its path, elsewhere as a copy. The other runtimes do
// the same (conformance/files.json).

using System.Text;

namespace Clarity.Runtime;

/// <summary>A file of a file store: its path inside the store with '/' between folders, its extension in lower case without the dot.</summary>
public sealed record FileEntry(string Store, string Path, string Name, string Extension, long Size, DateTime? Modified);

public sealed partial class Db
{
    /// <summary>Every file of a store and its folders, but those whose name or folder starts with a dot, sorted by path.</summary>
    public List<FileEntry> Files(string store)
    {
        var root = StoreFolder(store);
        var found = new List<FileEntry>();
        Walk(store, root, "", found);
        found.Sort((a, b) => string.CompareOrdinal(a.Path, b.Path));
        return found;
    }

    static void Walk(string store, string folder, string prefix, List<FileEntry> found)
    {
        try
        {
            foreach (var dir in Directory.EnumerateDirectories(folder))
            {
                var name = System.IO.Path.GetFileName(dir);
                if (!name.StartsWith('.'))
                    Walk(store, dir, $"{prefix}{name}/", found);
            }
            foreach (var file in Directory.EnumerateFiles(folder))
            {
                var name = System.IO.Path.GetFileName(file);
                if (name.StartsWith('.'))
                    continue;
                var info = new FileInfo(file);
                var extension = System.IO.Path.GetExtension(name).TrimStart('.').ToLowerInvariant();
                var modified = info.LastWriteTimeUtc;
                found.Add(new FileEntry(store, prefix + name, name, extension, info.Length,
                    new DateTime(modified.Year, modified.Month, modified.Day, modified.Hour, modified.Minute, modified.Second, DateTimeKind.Unspecified)));
            }
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new StorageException($"cannot read {folder}: {e.Message}", e);
        }
    }

    /// <summary>The bytes of the file <paramref name="path"/> of a store: a picture linked from a file before its record is saved.</summary>
    public byte[] StoreFile(string store, string path) => ReadFile(new FileEntry(store, path, "", "", 0, null));

    /// <summary>The text a file holds.</summary>
    public string ReadText(FileEntry file)
    {
        var bytes = ReadFile(file);
        try
        {
            return new UTF8Encoding(false, true).GetString(bytes);
        }
        catch (DecoderFallbackException)
        {
            throw new InvalidValueException($"{file.Path} is not text");
        }
    }

    /// <summary>Writes the text as the file <paramref name="path"/> of a store, making its folders.</summary>
    public void WriteText(string store, string path, string text) => WriteBytes(store, path, new UTF8Encoding(false).GetBytes(text));

    /// <summary>The contents of a file as they are.</summary>
    public byte[] ReadBytes(FileEntry file) => ReadFile(file);

    /// <summary>Writes the bytes as they are as the file <paramref name="path"/> of a store, making its folders.</summary>
    public void WriteBytes(string store, string path, byte[] bytes)
    {
        var file = Pictures.InStore(StoreFolder(store), path);
        try
        {
            Directory.CreateDirectory(System.IO.Path.GetDirectoryName(file)!);
            File.WriteAllBytes(file, bytes);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new StorageException($"cannot write {file}: {e.Message}", e);
        }
    }

    /// <summary>
    /// A file as the picture of a field kept in <paramref name="fieldStore"/> (null: in the
    /// database): the file itself when it is in that store, else a copy.
    /// </summary>
    public Picture PictureFrom(FileEntry file, string? fieldStore)
    {
        var bytes = ReadFile(file);
        Pictures.Checked(bytes);
        return fieldStore == file.Store ? Picture.InStore(file.Path) : Picture.New(bytes);
    }

    byte[] ReadFile(FileEntry file)
    {
        var path = Pictures.InStore(StoreFolder(file.Store), file.Path);
        try
        {
            return File.ReadAllBytes(path);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new StorageException($"cannot read {path}: {e.Message}", e);
        }
    }
}
