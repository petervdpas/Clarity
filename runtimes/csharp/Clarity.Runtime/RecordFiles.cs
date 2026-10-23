// Files in records (U1c): a file field holds no file (null), the one the
// record has stored, a new one chosen but not yet saved, or a file of its
// file store; always with its name. Like a picture, reading records never
// carries the file itself; the screen fetches it on its own to open it or
// save a copy. In the database the bytes are in the field's column and the
// name in the column beside it; in a file store the file keeps its name in
// a folder named by its contents (conformance/record-files.json).

using System.Diagnostics;
using System.Security.Cryptography;

namespace Clarity.Runtime;

/// <summary>The file of a file field that has one, and its name.</summary>
public sealed record RecordFile(string Name, Picture Content);

public static class RecordFiles
{
    /// <summary>The name a chosen file is kept under: the last part of what was chosen, trimmed; "file" when nothing is left.</summary>
    public static string CleanName(string? chosen)
    {
        var text = chosen ?? "";
        var last = text[(text.LastIndexOfAny(['/', '\\']) + 1)..].Trim();
        return last.Length == 0 ? "file" : last;
    }

    /// <summary>The first 32 hex digits of the SHA-256 of the bytes: the folder a file is kept in.</summary>
    static string Contents(byte[] bytes) => Convert.ToHexStringLower(SHA256.HashData(bytes))[..32];

    /// <summary>The path of a file in a file store: a folder named by its contents, and in it the file under its name.</summary>
    public static string StorePath(byte[] bytes, string? name) => $"{Contents(bytes)}/{CleanName(name)}";

    /// <summary>The name a path in a store shows: its last part.</summary>
    public static string NameOf(string path) => path[(path.LastIndexOfAny(['/', '\\']) + 1)..];

    /// <summary>The extension of a file's name, in lower case without its dot; "" without one.</summary>
    public static string ExtensionOf(string name) => Path.GetExtension(name).TrimStart('.').ToLowerInvariant();

    /// <summary>The name of a record's file; "" when it has none.</summary>
    public static string NameOf(RecordFile? file) => file?.Name ?? "";

    /// <summary>The extension of a record's file; "" when it has none.</summary>
    public static string ExtensionOf(RecordFile? file) => ExtensionOf(NameOf(file));

    /// <summary>The folder a copy is saved in on system <paramref name="os"/>, with <paramref name="env"/> giving the environment; null when it names no home.</summary>
    public static string? DownloadsIn(string os, Func<string, string?> env) =>
        env(os == "windows" ? "USERPROFILE" : "HOME") is { Length: > 0 } home ? $"{home}/Downloads" : null;

    /// <summary>The name a copy gets where <paramref name="taken"/> names are in use: the name itself, else " (2)", " (3)" and on before its extension.</summary>
    public static string FreeName(string name, Func<string, bool> taken)
    {
        if (!taken(name))
            return name;
        // A leading dot starts a name, not an extension.
        var dot = name.LastIndexOf('.');
        var (stem, extension) = dot > 0 ? (name[..dot], name[dot..]) : (name, "");
        for (var n = 2; ; n++)
            if ($"{stem} ({n}){extension}" is var candidate && !taken(candidate))
                return candidate;
    }

    static string Os() => OperatingSystem.IsWindows() ? "windows" : OperatingSystem.IsMacOS() ? "macos" : "linux";

    /// <summary>Saves a copy of a file in the person's Downloads folder, under a name free there; the file it wrote.</summary>
    public static string SaveCopy(string name, byte[] bytes)
    {
        var folder = DownloadsIn(Os(), Environment.GetEnvironmentVariable) ?? throw new InvalidValueException("this system names no Downloads folder");
        try
        {
            Directory.CreateDirectory(folder);
            var file = Path.Combine(folder, FreeName(CleanName(name), n => File.Exists(Path.Combine(folder, n))));
            File.WriteAllBytes(file, bytes);
            return file;
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new StorageException($"cannot save a copy of {name}: {e.Message}", e);
        }
    }

    /// <summary>Opens a file in the program the system uses for its kind, from a copy in a folder of its own under the system's temporary folder.</summary>
    public static void Open(string name, byte[] bytes)
    {
        var folder = Path.Combine(Path.GetTempPath(), $"clarity-open-{Contents(bytes)}");
        var file = Path.Combine(folder, CleanName(name));
        try
        {
            Directory.CreateDirectory(folder);
            File.WriteAllBytes(file, bytes);
            Process.Start(new ProcessStartInfo(file) { UseShellExecute = true });
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or global::System.ComponentModel.Win32Exception)
        {
            throw new StorageException($"cannot open {file}: {e.Message}", e);
        }
    }
}
