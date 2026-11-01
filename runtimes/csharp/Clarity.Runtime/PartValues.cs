// How a part's values cross as JSON: read, where numbers may come as their
// text, as an engine or a custom part gives a decimal; and written, as every
// stack writes them.

using System.Globalization;
using System.Text.Json;

namespace Clarity.Runtime;

public static class PartValues
{
    public static readonly JsonSerializerOptions Json = new() { NumberHandling = System.Text.Json.Serialization.JsonNumberHandling.AllowReadingFromString };

    /// <summary>A told value at <paramref name="at"/> as the handler's type; its empty value when missing.</summary>
    public static T Read<T>(JsonElement[] values, int at) =>
        at < values.Length ? values[at].Deserialize<T>(Json)! : default!;

    /// <summary>A value as a tell gives it: a whole number or a bool as JSON's own, a decimal,
    /// a date or a moment as text in its plain form.</summary>
    public static string Write(object? value) => value switch
    {
        null => "null",
        decimal d => JsonSerializer.Serialize(d.ToString(CultureInfo.InvariantCulture)),
        DateOnly d => JsonSerializer.Serialize(d.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture)),
        DateTime t => JsonSerializer.Serialize(t.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture)),
        object?[] list => "[" + string.Join(",", list.Select(Write)) + "]",
        Enum e => JsonSerializer.Serialize(e.ToString()),
        _ => JsonSerializer.Serialize(value),
    };
}
