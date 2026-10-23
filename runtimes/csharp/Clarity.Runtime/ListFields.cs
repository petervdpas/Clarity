// Lists of values as fields (U1c): `list<text>`, `list<Genre>`, kept in
// one column as compact JSON, text as it is, decimals as text and enums by
// name (conformance/list-fields.json).

using System.Globalization;
using System.Text.Encodings.Web;
using System.Text.Json;

namespace Clarity.Runtime;

public static class ListFields
{
    static readonly JsonWriterOptions Writing = new() { Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping };

    /// <summary>The column text of a list.</summary>
    public static string Kept<T>(IEnumerable<T>? items)
    {
        using var buffer = new MemoryStream();
        using (var json = new Utf8JsonWriter(buffer, Writing))
        {
            json.WriteStartArray();
            foreach (var item in items ?? [])
                Write(json, item);
            json.WriteEndArray();
        }
        return System.Text.Encoding.UTF8.GetString(buffer.ToArray());
    }

    static void Write<T>(Utf8JsonWriter json, T item)
    {
        switch (item)
        {
            case null: json.WriteNullValue(); break;
            case string s: json.WriteStringValue(s); break;
            case long n: json.WriteNumberValue(n); break;
            case decimal d: json.WriteStringValue(d.ToString(CultureInfo.InvariantCulture)); break;
            case DateOnly d: json.WriteStringValue(d.ToString(Sql.DateFormat, CultureInfo.InvariantCulture)); break;
            case Enum e: json.WriteStringValue(e.ToString()); break;
            default: throw new ArgumentException($"a list field cannot hold {typeof(T).Name}");
        }
    }

    /// <summary>The items of a list column; none when it is null or empty.</summary>
    public static List<T> Read<T>(string column, object? value)
    {
        var text = (value as string)?.Trim();
        if (string.IsNullOrEmpty(text))
            return [];
        try
        {
            using var json = JsonDocument.Parse(text);
            return [.. json.RootElement.EnumerateArray().Select(Item<T>)];
        }
        catch (Exception e) when (e is JsonException or InvalidOperationException or FormatException or ArgumentException)
        {
            throw Sql.Invalid(column, value);
        }
    }

    static T Item<T>(JsonElement e)
    {
        var type = Nullable.GetUnderlyingType(typeof(T)) ?? typeof(T);
        if (e.ValueKind == JsonValueKind.Null)
            return default!;
        object item = type switch
        {
            _ when type == typeof(string) => e.GetString()!,
            _ when type == typeof(long) => e.GetInt64(),
            _ when type == typeof(decimal) => decimal.Parse(e.GetString()!, NumberStyles.Number, CultureInfo.InvariantCulture),
            _ when type == typeof(DateOnly) => DateOnly.ParseExact(e.GetString()!, Sql.DateFormat, CultureInfo.InvariantCulture),
            _ when type.IsEnum => Enum.Parse(type, e.GetString()!, false),
            _ => throw new ArgumentException($"a list field cannot hold {type.Name}"),
        };
        return (T)item;
    }
}
