// The files of a file store, which code reaches through `files f in store`,
// `read text of f` and `write t to store "path"`, and of any other folder
// (`files f in folder category.folder`), by its path after "folder:", with
// the extensions of the files it holds and whether its hidden ones count. A file found can become a record's picture: in the
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

    /// <summary>The store of a folder that is no store: "folder:" and its path.</summary>
    public const string FolderStore = "folder:";

    /// <summary>
    /// Whether a file's name (Notes.MD, .env) is one a folder takes: by its extensions
    /// ("*.md", ignoring case; none or "*.*": any file), a hidden one (its name starting
    /// with a dot) only with <paramref name="hidden"/> (conformance/files.json).
    /// </summary>
    public static bool Takes(IReadOnlyList<string> patterns, bool hidden, string name)
    {
        if (name.StartsWith('.') && !hidden)
            return false;
        if (patterns.Count == 0 || patterns.Contains("*.*"))
            return true;
        var extension = System.IO.Path.GetExtension(name).TrimStart('.').ToLowerInvariant();
        return patterns.Any(p => p.StartsWith("*.", StringComparison.Ordinal) && extension.Length > 0 && extension == p[2..].ToLowerInvariant());
    }

    /// <summary>
    /// Whether a blob or file field that says its extensions takes the file <paramref name="name"/>
    /// (a name or a path): its last part matched as a folder matches, a hidden one only with
    /// <paramref name="hidden"/>; no file (an empty name) is always taken. One of another type is
    /// refused (RefusedException) in Clarity's own words, fileTypeRefused (conformance/files.json, fieldFiles).
    /// </summary>
    public static void CheckFileType(string field, IReadOnlyList<string> patterns, bool hidden, string? name)
    {
        var whole = name ?? "";
        var last = whole[(whole.LastIndexOfAny(['/', '\\']) + 1)..];
        if (last.Length == 0 || Takes(patterns, hidden, last))
            return;
        var holds = last.StartsWith('.') && !hidden ? "no hidden" : patterns.Count == 0 ? "any" : string.Join(" ", patterns);
        throw new RefusedException(ClarityException.Own("fileTypeRefused", "{field} takes {extensions} files, not {name}", ("field", field), ("extensions", holds), ("name", last)));
    }

    /// <summary>The files of a folder ("folder:" and its path) its extensions take, in its folders too, sorted by path; hidden ones only with <paramref name="hidden"/>.</summary>
    public List<FileEntry> FilesIn(string folder, IReadOnlyList<string> patterns, bool hidden)
    {
        var root = StoreFolder(folder);
        var found = new List<FileEntry>();
        Walk(folder, root, "", found, hidden);
        found.RemoveAll(f => !Takes(patterns, hidden, f.Name));
        found.Sort((a, b) => string.CompareOrdinal(a.Path, b.Path));
        return found;
    }

    /// <summary>Writes the text as the file <paramref name="path"/> of a folder, when it takes the file.</summary>
    public void WriteTextIn(string folder, IReadOnlyList<string> patterns, bool hidden, string path, string text) =>
        WriteBytesIn(folder, patterns, hidden, path, new UTF8Encoding(false).GetBytes(text));

    /// <summary>Writes the bytes as the file <paramref name="path"/> of a folder, when it takes the file.</summary>
    public void WriteBytesIn(string folder, IReadOnlyList<string> patterns, bool hidden, string path, byte[] bytes)
    {
        var name = path[(path.LastIndexOf('/') + 1)..];
        if (!Takes(patterns, hidden, name))
        {
            var holds = name.StartsWith('.') && !hidden ? "no hidden" : string.Join(" ", patterns);
            throw new InvalidValueException($"the folder holds {holds} files, not '{path}'");
        }
        WriteBytes(folder, path, bytes);
    }

    static void Walk(string store, string folder, string prefix, List<FileEntry> found, bool hidden = false)
    {
        try
        {
            foreach (var dir in Directory.EnumerateDirectories(folder))
            {
                var name = System.IO.Path.GetFileName(dir);
                if (!name.StartsWith('.') || hidden)
                    Walk(store, dir, $"{prefix}{name}/", found, hidden);
            }
            foreach (var file in Directory.EnumerateFiles(folder))
            {
                var name = System.IO.Path.GetFileName(file);
                if (name.StartsWith('.') && !hidden)
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

    /// <summary>
    /// A file of a store as the file of a file field kept in <paramref name="fieldStore"/>
    /// (null: in the database), under its name: the file itself when it is in that store, else a copy.
    /// </summary>
    public RecordFile RecordFileFrom(FileEntry file, string? fieldStore) =>
        new(file.Name, fieldStore == file.Store ? Picture.InStore(file.Path) : Picture.New(ReadFile(file)));

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
