// Clarity's JSON: values as Clarity writes them everywhere (the window
// wire, the data formats of code through Clarity.Formats, calls to web
// APIs), and records decoded from it.

using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Clarity.Runtime;

/// <summary>Clarity's names on the wire: a member's name with its first letter small, so CreditLimit
/// (or CreditLimit_, where the name was taken) is <c>creditLimit</c>, the name the project wrote.</summary>
public sealed class ClarityNaming : JsonNamingPolicy
{
    public static readonly ClarityNaming Instance = new();

    public override string ConvertName(string name)
    {
        var bare = name.TrimEnd('_');
        return bare.Length == 0 ? bare : char.ToLowerInvariant(bare[0]) + bare[1..];
    }
}

/// <summary>Values as Clarity's JSON, and records decoded from it.</summary>
public static class JsonValues
{
    /// <summary>Values as Clarity writes them: names as the project wrote them, decimals as exact text, enums by name.</summary>
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = ClarityNaming.Instance,
        Converters = { new DecimalAsText(), new JsonStringEnumConverter() },
        Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    /// <summary>The JSON of a value, as Clarity writes values everywhere.</summary>
    public static string Of<T>(T value) => JsonSerializer.Serialize(value, Options);

    /// <summary>Records (or a value) decoded from Clarity's JSON, such as what Clarity.Formats fitted; throws why not.</summary>
    public static T Decode<T>(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<T>(json, Options) ?? throw new ValueException("what was read does not fit");
        }
        catch (JsonException e)
        {
            throw new ValueException($"what was read does not fit: {e.Message}");
        }
    }

    /// <summary>A decimal as exact text, as Clarity's values travel; read from text or a number.</summary>
    sealed class DecimalAsText : JsonConverter<decimal>
    {
        public override decimal Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options) =>
            reader.TokenType == JsonTokenType.String
                ? decimal.Parse(reader.GetString()!, NumberStyles.Number, CultureInfo.InvariantCulture)
                : reader.GetDecimal();

        public override void Write(Utf8JsonWriter writer, decimal value, JsonSerializerOptions options) =>
            writer.WriteStringValue(value.ToString(CultureInfo.InvariantCulture));
    }
}
