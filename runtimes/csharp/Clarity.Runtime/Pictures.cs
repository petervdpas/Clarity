// Pictures in records: an image field holds no picture (null), the one the
// record has stored, or a new one loaded but not yet saved. Reading records
// never carries the pictures themselves, so a list of thousands stays
// light; the screen fetches each picture on its own. A picture is kept in
// the database, or as a file in a file store named by the hash of its
// bytes, the column holding that name. The other runtimes do the same
// (conformance/pictures.json).

using System.Security.Cryptography;

namespace Clarity.Runtime;

/// <summary>The picture of an image field that has one.</summary>
public sealed class Picture : IEquatable<Picture>
{
    /// <summary>The record has one stored; it is fetched on its own.</summary>
    public static readonly Picture Stored = new(null);

    /// <summary>The bytes of one loaded but not yet saved; null for <see cref="Stored"/>.</summary>
    public byte[]? Bytes { get; }

    Picture(byte[]? bytes) => Bytes = bytes;

    /// <summary>One loaded but not yet saved.</summary>
    public static Picture New(byte[] bytes) => new(bytes);

    public bool Equals(Picture? other) =>
        other is not null && (ReferenceEquals(this, other) || (Bytes is not null && other.Bytes is not null && Bytes.AsSpan().SequenceEqual(other.Bytes)));

    public override bool Equals(object? obj) => Equals(obj as Picture);

    public override int GetHashCode() => Bytes is null ? 0 : Bytes.Length;
}

/// <summary>An image field's column, and the file store it keeps its pictures in; without one they are in the database.</summary>
public sealed record PictureColumn(string Column, string? Store);

/// <summary>A kind of picture every stack shows.</summary>
public sealed record PictureKind(string Extension, string MediaType);

public static class Pictures
{
    /// <summary>The kind of picture the bytes are, told by their first bytes.</summary>
    public static PictureKind? KindOf(ReadOnlySpan<byte> bytes) => bytes switch
    {
        [0x89, (byte)'P', (byte)'N', (byte)'G', ..] => new("png", "image/png"),
        [0xFF, 0xD8, 0xFF, ..] => new("jpg", "image/jpeg"),
        [(byte)'G', (byte)'I', (byte)'F', (byte)'8', ..] => new("gif", "image/gif"),
        [(byte)'B', (byte)'M', ..] => new("bmp", "image/bmp"),
        [(byte)'R', (byte)'I', (byte)'F', (byte)'F', _, _, _, _, (byte)'W', (byte)'E', (byte)'B', (byte)'P', ..] => new("webp", "image/webp"),
        _ => null,
    };

    /// <summary>The kind of the bytes, or why they cannot be kept as a picture.</summary>
    public static PictureKind Checked(ReadOnlySpan<byte> bytes) =>
        KindOf(bytes) ?? throw new InvalidValueException("this file is not a picture; use a png, jpg, gif, bmp or webp file");

    /// <summary>
    /// The name of a picture's file in a file store: the first 32 hex digits
    /// of the SHA-256 of its bytes, and the extension of its kind.
    /// </summary>
    public static string FileName(byte[] bytes)
    {
        var kind = Checked(bytes);
        return $"{Convert.ToHexStringLower(SHA256.HashData(bytes))[..32]}.{kind.Extension}";
    }

    /// <summary>A picture as a page shows it.</summary>
    public static string DataUrl(byte[] bytes) => $"data:{Checked(bytes).MediaType};base64,{Convert.ToBase64String(bytes)}";

    /// <summary>What the select list reads of a picture kept in the database: whether there is one, not the picture.</summary>
    public static string Presence(Dialect d, string column) => $"CASE WHEN {d.Quote(column)} IS NULL THEN 0 ELSE 1 END";

    /// <summary>The file a column's path names in the store at <paramref name="root"/>; a path that would leave the store is refused.</summary>
    internal static string InStore(string root, string path)
    {
        var parts = path.Split('/', '\\');
        if (path.Length == 0 || Path.IsPathRooted(path) || parts.Any(p => p is "" or "." or ".."))
            throw new InvalidValueException($"'{path}' is not a picture of its file store");
        return Path.Combine([root, .. parts]);
    }
}
