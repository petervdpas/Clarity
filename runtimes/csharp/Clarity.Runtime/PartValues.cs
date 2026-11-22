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

    /// <summary>A value as a tell, a part or a report's data gives it: as Clarity writes
    /// values everywhere (conformance/values-json.json), the same as a part's value on the wire.</summary>
    public static string Write(object? value) => WireJson.Value(value)?.ToJsonString(JsonValues.Options) ?? "null";
}
