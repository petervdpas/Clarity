// Text the screen shows (a message, a refusal, a caption, an error): the
// text as written in Clarity with a typed value per slot, so the screen
// fills each slot in the person's format.

using System.Globalization;
using System.Text;

namespace Clarity.Runtime;

public enum SlotKind
{
    Text,
    Integer,
    Decimal,
    Money,
    Date,
    DateTime,
    Bool,
    Enum,
}

/// <summary>
/// The value of one slot, with what the screen needs to format it. Values
/// are kept as the Rust runtime sends them: numbers and dates as invariant
/// text, so no stack loses precision.
/// </summary>
public sealed record Slot(SlotKind Kind, object? Value, int? Scale = null, string? Format = null, string? EnumName = null)
{
    public static Slot OfText(string value) => new(SlotKind.Text, value);

    public static Slot OfInteger(long value, string? format) => new(SlotKind.Integer, value, Format: format);

    public static Slot OfDecimal(decimal value, int? scale, string? format) =>
        new(SlotKind.Decimal, value.ToString(CultureInfo.InvariantCulture), scale, format);

    public static Slot OfMoney(decimal value, string? format) =>
        new(SlotKind.Money, value.ToString(CultureInfo.InvariantCulture), Format: format);

    public static Slot OfDate(DateOnly? value, string? format) =>
        new(SlotKind.Date, value?.ToString(Sql.DateFormat, CultureInfo.InvariantCulture), Format: format);

    public static Slot OfDateTime(DateTime? value, string? format) =>
        new(SlotKind.DateTime, value?.ToString(Sql.DateTimeFormat, CultureInfo.InvariantCulture), Format: format);

    public static Slot OfBoolean(bool value) => new(SlotKind.Bool, value);

    /// <summary>An enum value, by its name; the screen shows its label.</summary>
    public static Slot OfName<E>(E value, string enumName) where E : struct, Enum =>
        new(SlotKind.Enum, value.ToString(), EnumName: enumName);

    /// <summary>The value as plain text, for places that are not a screen.</summary>
    internal string Plain() => Value switch
    {
        null => "",
        bool b => b ? "yes" : "no",
        long n => n.ToString(CultureInfo.InvariantCulture),
        var other => other.ToString() ?? "",
    };
}

public sealed record Text(string Template)
{
    /// <summary>The value of each slot, by its path as written: <c>customer.name</c>.</summary>
    public SortedDictionary<string, Slot> Values { get; } = new(StringComparer.Ordinal);

    /// <summary>Text that is shown as it is: a brace in it is no slot.</summary>
    public static Text Plain(string text) => new(text.Replace("{", "{{").Replace("}", "}}"));

    public Text With(string path, Slot value)
    {
        Values[path] = value;
        return this;
    }

    /// <summary>
    /// The text with its slots filled plainly, without a person's format: for
    /// logs, and for code that reads a caption back.
    /// </summary>
    public string Show()
    {
        var out_ = new StringBuilder();
        var rest = Template.AsSpan();
        while (rest.IndexOfAny('{', '}') is var at && at >= 0)
        {
            out_.Append(rest[..at]);
            var brace = rest[at];
            var after = rest[(at + 1)..];
            // `{{` and `}}` are braces.
            if (after.Length > 0 && after[0] == brace)
            {
                out_.Append(brace);
                rest = after[1..];
                continue;
            }
            if (brace == '}')
            {
                out_.Append('}');
                rest = after;
                continue;
            }
            var end = after.IndexOf('}');
            if (end < 0)
                end = after.Length;
            var path = after[..end].ToString();
            out_.Append(Values.TryGetValue(path, out var slot) ? slot.Plain() : "");
            rest = end < after.Length ? after[(end + 1)..] : [];
        }
        out_.Append(rest);
        return out_.ToString();
    }
}
