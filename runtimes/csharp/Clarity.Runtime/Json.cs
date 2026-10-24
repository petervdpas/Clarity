// JSON in code (DESIGN 5.23): `json of value` and `read Order from json
// text`. A service's JSON is not Clarity's own, so what is read is fitted to
// the entity first: names match ignoring case, underscores and hyphens,
// numbers and strings of numbers stand for each other, what is left out
// keeps its default. The other runtimes fit alike (conformance/json.json);
// the record's own decoding then reads what was fitted.

using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace Clarity.Runtime;

/// <summary>What a field takes from JSON: text, integer, decimal (and money), bool, date, datetime, enum (one of
/// <paramref name="Values"/>, any case), values (a list whose items fit <paramref name="Item"/>) or other (never read).</summary>
public sealed record JsonKind(string Is, string[]? Values = null, JsonKind? Item = null);

/// <summary>A field of an entity, as JSON is fitted to it.</summary>
public sealed record JsonField(string Name, JsonKind Kind);

/// <summary>Values as Clarity's JSON, and records read from a service's JSON.</summary>
public static class JsonValues
{
    /// <summary>Values as Clarity writes them: names in snake case, decimals as exact text, enums by name.</summary>
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        Converters = { new DecimalAsText(), new JsonStringEnumConverter() },
        Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    static readonly JsonSerializerOptions Compact = new() { Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping };

    /// <summary>The JSON of a value, as Clarity writes values everywhere.</summary>
    public static string Of<T>(T value) => JsonSerializer.Serialize(value, Options);

    /// <summary>Reads <paramref name="text"/> as one record of <paramref name="entity"/>, starting at <paramref name="at"/>,
    /// fitted to <paramref name="fields"/> over <paramref name="template"/>, a new record.</summary>
    public static T Read<T>(string text, string? at, string entity, JsonField[] fields, T template)
    {
        var fitted = Fit(text, at, false, entity, fields, JsonSerializer.SerializeToNode(template, Options));
        return Decode<T>(fitted, entity);
    }

    /// <summary>Reads <paramref name="text"/> as a list of records of <paramref name="entity"/>; see <see cref="Read"/>.</summary>
    public static List<T> ReadList<T>(string text, string? at, string entity, JsonField[] fields, T template)
    {
        var fitted = Fit(text, at, true, entity, fields, JsonSerializer.SerializeToNode(template, Options));
        return Decode<List<T>>(fitted, entity);
    }

    static T Decode<T>(JsonNode? fitted, string entity)
    {
        try
        {
            return fitted.Deserialize<T>(Options) ?? throw new ValueException($"the JSON does not fit {entity}");
        }
        catch (JsonException e)
        {
            throw new ValueException($"the JSON does not fit {entity}: {e.Message}");
        }
    }

    /// <summary>What <see cref="Read"/> decodes: the JSON fitted to the entity; throws why it does not fit.</summary>
    public static JsonNode? Fit(string text, string? at, bool many, string entity, JsonField[] fields, JsonNode? template)
    {
        JsonNode? value;
        try
        {
            value = JsonNode.Parse(text);
        }
        catch (JsonException)
        {
            throw new ValueException("the text is no JSON");
        }
        if (!string.IsNullOrEmpty(at))
        {
            var walked = new List<string>();
            foreach (var step in at.Split('.'))
            {
                walked.Add(step);
                if (value is not JsonObject o || !o.TryGetPropertyValue(step, out var next))
                    throw new ValueException($"the JSON has no '{string.Join(".", walked)}'");
                value = next;
            }
        }
        var baseRecord = template as JsonObject ?? new JsonObject();
        if (many)
        {
            if (value is not JsonArray items)
                throw new ValueException($"the JSON is not a list of {entity}");
            var fitted = new JsonArray();
            for (var i = 0; i < items.Count; i++)
            {
                if (items[i] is not JsonObject item)
                    throw new ValueException($"item {i + 1} of the list is not one {entity}");
                try
                {
                    fitted.Add(Record(item, entity, fields, baseRecord));
                }
                catch (ValueException e)
                {
                    throw new ValueException($"item {i + 1}: {e.Message}");
                }
            }
            return fitted;
        }
        if (value is not JsonObject one)
            throw new ValueException($"the JSON is not one {entity}");
        return Record(one, entity, fields, baseRecord);
    }

    /// <summary>A name as names are matched: lower case, without underscores, hyphens and spaces.</summary>
    static string Plain(string name) => new(name.Where(c => c is not ('_' or '-' or ' ')).Select(char.ToLowerInvariant).ToArray());

    static JsonObject Record(JsonObject given, string entity, JsonField[] fields, JsonObject template)
    {
        var record = (JsonObject)template.DeepClone();
        foreach (var field in fields)
        {
            if (!given.TryGetPropertyValue(field.Name, out var value))
                value = given.FirstOrDefault(p => Plain(p.Key) == Plain(field.Name)).Value;
            if (value is null || field.Kind.Is == "other")
                continue;
            record[field.Name] = Value(value, field.Kind) ?? throw new ValueException($"field '{field.Name}' of {entity} is {Wanted(field.Kind)}, not {value.ToJsonString(Compact)}");
        }
        return record;
    }

    static string Wanted(JsonKind kind) => kind.Is switch
    {
        "text" => "text",
        "integer" => "an integer",
        "decimal" => "a decimal",
        "bool" => "true or false",
        "date" => "a date",
        "datetime" => "a date and time",
        "enum" => kind.Values switch
        {
            null or [] => "an enum value",
            [var one] => one,
            var values => $"{string.Join(", ", values[..^1])} or {values[^1]}",
        },
        "values" => "a list",
        _ => "something else",
    };

    static bool Digits(string text) => text.Length > 0 && text.All(char.IsAsciiDigit);

    static bool IsDecimal(string text)
    {
        var digits = text.StartsWith('-') ? text[1..] : text;
        var dot = digits.IndexOf('.');
        return dot < 0 ? Digits(digits) : Digits(digits[..dot]) && Digits(digits[(dot + 1)..]);
    }

    static bool IsDate(string text) => text.Length == 10 && text[4] == '-' && text[7] == '-' && Digits(text[..4]) && Digits(text[5..7]) && Digits(text[8..]);

    /// <summary>A date and time as Clarity writes it, YYYY-MM-DDTHH:MM:SS, from the same with or without seconds,
    /// a fraction of a second and a Z.</summary>
    static string? DateTimeText(string text)
    {
        if (text.EndsWith('Z'))
            text = text[..^1];
        var at = text.IndexOfAny(['T', ' ']);
        if (at < 0 || !IsDate(text[..at]))
            return null;
        var clock = text[(at + 1)..].Split('.')[0];
        var parts = clock.Split(':');
        bool Two(string p) => p.Length == 2 && Digits(p);
        return parts switch
        {
            [var h, var m] when Two(h) && Two(m) => $"{text[..at]}T{h}:{m}:00",
            [var h, var m, var s] when Two(h) && Two(m) && Two(s) => $"{text[..at]}T{h}:{m}:{s}",
            _ => null,
        };
    }

    static JsonNode? Value(JsonNode given, JsonKind kind)
    {
        var element = given.GetValueKind();
        string? Text() => element == JsonValueKind.String ? given.GetValue<string>() : null;
        switch (kind.Is)
        {
            case "text":
                return element switch
                {
                    JsonValueKind.String => JsonValue.Create(Text()),
                    JsonValueKind.Number or JsonValueKind.True or JsonValueKind.False => JsonValue.Create(given.ToJsonString()),
                    _ => null,
                };
            case "integer":
                if (element == JsonValueKind.Number)
                {
                    var raw = given.ToJsonString();
                    if (long.TryParse(raw, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var whole))
                        return JsonValue.Create(whole);
                    if (double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out var d) && d == Math.Floor(d) && Math.Abs(d) < 9e15)
                        return JsonValue.Create((long)d);
                    return null;
                }
                return Text() is { } t && long.TryParse(t.Trim(), NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n) ? JsonValue.Create(n) : null;
            case "decimal":
                if (element == JsonValueKind.Number)
                    return JsonValue.Create(given.ToJsonString());
                return Text() is { } dt && IsDecimal(dt.Trim()) ? JsonValue.Create(dt.Trim()) : null;
            case "bool":
                return element is JsonValueKind.True or JsonValueKind.False ? JsonValue.Create(element == JsonValueKind.True) : null;
            case "date":
                if (Text() is not { } date)
                    return null;
                if (IsDate(date))
                    return JsonValue.Create(date);
                return DateTimeText(date) is { } moment ? JsonValue.Create(moment[..10]) : null;
            case "datetime":
                return Text() is { } when && DateTimeText(when) is { } fitted ? JsonValue.Create(fitted) : null;
            case "enum":
                return Text() is { } word && kind.Values?.FirstOrDefault(v => string.Equals(v, word, StringComparison.OrdinalIgnoreCase)) is { } found
                    ? JsonValue.Create(found)
                    : null;
            case "values":
                if (given is not JsonArray items || kind.Item is null)
                    return null;
                var list = new JsonArray();
                foreach (var item in items)
                {
                    if (item is null || Value(item, kind.Item) is not { } fittedItem)
                        return null;
                    list.Add(fittedItem);
                }
                return list;
        }
        return null;
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
