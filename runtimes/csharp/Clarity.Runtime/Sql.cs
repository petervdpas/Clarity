// The values records are stored as, and the conversions between field
// values and them, used by the records Clarity generates. Each database
// driver stores a value its own way: SQLite keeps decimals and dates as
// text, so no precision is lost, and the server databases use their own
// number and date types. Enums are stored by name.

using System.Globalization;

namespace Clarity.Runtime;

public static class Sql
{
    internal const string DateFormat = "yyyy-MM-dd";
    internal const string DateTimeFormat = "yyyy-MM-ddTHH:mm:ss";

    static readonly string[] DateTimeFormats = [DateTimeFormat, "yyyy-MM-dd HH:mm:ss", "yyyy-MM-ddTHH:mm", "yyyy-MM-dd HH:mm"];

    /// <summary>An enum value, stored by its name.</summary>
    public static string Name<E>(E value) where E : struct, Enum => value.ToString();

    /// <summary>A picture the record has stored already: its column is left as it is when the record is saved.</summary>
    public static readonly object Kept = new();

    /// <summary>A picture as it is saved: none, the one stored already (kept), or a new one.</summary>
    public static object? Picture(Picture? value) => value is null ? null : value.Bytes ?? Kept;

    /// <summary>
    /// Whether a record has a picture: what the select list reads of one kept in
    /// the database (1 or 0), or the path of its file in a file store.
    /// </summary>
    public static Picture? GetPicture(string column, object? value)
    {
        var there = value switch
        {
            null => false,
            long n => n != 0,
            int n => n != 0,
            bool b => b,
            string path => path.Length > 0,
            byte[] bytes => bytes.Length > 0,
            _ => throw new StorageException($"{column}: {value} is no picture"),
        };
        return there ? Runtime.Picture.Stored : null;
    }

    static InvalidValueException Invalid(string column, object? value) => new($"{column} cannot be {value ?? "null"}");

    public static string GetText(string column, object? value) => value switch
    {
        string s => s,
        null => "",
        // Databases written by other programs may hold numbers in text columns.
        long n => n.ToString(CultureInfo.InvariantCulture),
        double r => r.ToString(CultureInfo.InvariantCulture),
        decimal d => d.ToString(CultureInfo.InvariantCulture),
        _ => throw Invalid(column, value),
    };

    public static long GetInteger(string column, object? value) => value switch
    {
        long n => n,
        int n => n,
        null => 0,
        bool b => b ? 1 : 0,
        decimal d when d == decimal.Truncate(d) => (long)d,
        _ => throw Invalid(column, value),
    };

    public static decimal GetDecimal(string column, object? value) => value switch
    {
        decimal d => d,
        string s when decimal.TryParse(s, NumberStyles.Number, CultureInfo.InvariantCulture, out var d) => d,
        long n => n,
        double r => (decimal)r,
        null => 0m,
        _ => throw Invalid(column, value),
    };

    public static bool GetBoolean(string column, object? value) => value is bool b ? b : GetInteger(column, value) != 0;

    public static DateOnly? GetDate(string column, object? value) => value switch
    {
        null => null,
        DateOnly d => d,
        DateTime d => DateOnly.FromDateTime(d),
        // A date can be written with a time of day after it.
        string s when DateOnly.TryParseExact(s.Length > 10 ? s[..10] : s, DateFormat, CultureInfo.InvariantCulture,
            DateTimeStyles.None, out var d) => d,
        _ => throw Invalid(column, value),
    };

    public static DateTime? GetDateTime(string column, object? value) => value switch
    {
        null => null,
        DateTime d => d,
        DateOnly d => d.ToDateTime(TimeOnly.MinValue),
        string s when DateTime.TryParseExact(s, DateTimeFormats, CultureInfo.InvariantCulture, DateTimeStyles.None, out var d) => d,
        _ => throw Invalid(column, value),
    };

    public static long? GetReference(string column, object? value) => value switch
    {
        null => null,
        long n => n,
        int n => n,
        _ => throw Invalid(column, value),
    };

    /// <summary>An enum value by its name; an empty or missing value gives the default.</summary>
    public static E GetName<E>(string column, object? value) where E : struct, Enum => value switch
    {
        null or "" => default,
        string s when Enum.TryParse<E>(s, false, out var e) && Enum.IsDefined(e) => e,
        _ => throw Invalid(column, value),
    };
}

/// <summary>How a value appears in text, for messages and <c>"text" + value</c>.</summary>
public static class Show
{
    public static string Of(string value) => value;

    public static string Of(long value) => value.ToString(CultureInfo.InvariantCulture);

    public static string Of(decimal value) => value.ToString(CultureInfo.InvariantCulture);

    public static string Of(bool value) => value ? "yes" : "no";

    public static string Of(DateOnly? value) => value?.ToString(Sql.DateFormat, CultureInfo.InvariantCulture) ?? "";

    public static string Of(DateTime? value) => value?.ToString("yyyy-MM-dd HH:mm", CultureInfo.InvariantCulture) ?? "";

    public static string Of<E>(E value) where E : struct, Enum => value.ToString();
}
