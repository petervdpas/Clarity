// A number box shows its value in the person's format and reads typed text
// the same way (conformance/formats.json): every reading case of the shared
// file, typed into a real number box; a value without a format keeps its
// text; what code sets shows and is no change.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class NumberPartTests
{
    static readonly JsonElement Formats = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "formats.json"))).RootElement;

    static FormatKind Kind(string kind) => kind switch
    {
        "integer" => FormatKind.Integer,
        "decimal" => FormatKind.Decimal,
        "money" => FormatKind.Money,
        "duration" => FormatKind.Duration,
        "date" => FormatKind.Date,
        "dateTime" => FormatKind.DateTime,
        _ => throw new ArgumentException(kind),
    };

    static FieldFormat Field(JsonElement f) => new(
        Kind(f.GetProperty("kind").GetString()!),
        f.TryGetProperty("scale", out var s) && s.ValueKind == JsonValueKind.Number ? s.GetInt32() : null,
        f.TryGetProperty("format", out var n) && n.ValueKind == JsonValueKind.String ? n.GetString() : null);

    static (Screen Screen, NumberPart Part, TextFactsHost Host) Number(FieldFormat format, object? value, string locale = "en-US", string? currency = null)
    {
        var host = new TextFactsHost { Formats = new(CultureInfo.GetCultureInfo(locale), currency) };
        host.Formatted["amount"] = format;
        host.Values["amount"] = value;
        var (screen, _, parts) = Made.Window("""<Number at="1,1" span="4" name="amount" bind="amount" onchange="changed" />""", host);
        return (screen, (NumberPart)parts[0], host);
    }

    [Fact]
    public void Typed_text_is_read_in_the_persons_format_as_every_stack_reads_it()
    {
        var wrong = new List<string>();
        var cases = Formats.GetProperty("reads").EnumerateArray().Where(c => Field(c.GetProperty("field")).Kind is not (FormatKind.Date or FormatKind.DateTime)).ToList();
        Assert.True(cases.Count > 5);
        foreach (var c in cases)
        {
            var format = Field(c.GetProperty("field"));
            var start = format.Kind is FormatKind.Integer or FormatKind.Duration ? (object)987654L : 987654m;
            var (screen, part, host) = Number(format, start, c.GetProperty("locale").GetString()!, c.GetProperty("currency").GetString());
            TextWindow.Type(screen, part.Editor, c.GetProperty("typed").GetString()!);
            screen.Focus(null);
            var expected = c.GetProperty("value");
            var value = host.Values["amount"];
            var ok = expected.ValueKind == JsonValueKind.Null
                ? part.Bad is not null && Equals(value, start)
                : part.Bad is null && NumberRange.Number(value) == decimal.Parse(expected.ValueKind == JsonValueKind.Number ? expected.GetRawText() : expected.GetString()!, CultureInfo.InvariantCulture);
            if (!ok)
                wrong.Add($"{c.GetProperty("what")} {c.GetProperty("typed")}: {value} (bad {part.Bad})");
        }
        Assert.Empty(wrong);
    }

    [Fact]
    public void The_value_shows_in_the_persons_format_without_a_currency_sign()
    {
        var (_, part, _) = Number(new FieldFormat(FormatKind.Money), 1250.5m, "nl-NL", "EUR");
        Assert.Equal("1.250,50", part.Editor.Text);
        var (_, whole, _) = Number(new FieldFormat(FormatKind.Integer), 1234567L);
        Assert.Equal("1,234,567", whole.Editor.Text);
    }

    [Fact]
    public void A_whole_number_field_stores_a_long_and_a_cleared_one_is_zero()
    {
        var (screen, part, host) = Number(new FieldFormat(FormatKind.Integer), 5L);
        TextWindow.Type(screen, part.Editor, "1,200");
        screen.Focus(null);
        Assert.Equal(1200L, host.Values["amount"]);
        TextWindow.Type(screen, part.Editor, "");
        screen.Focus(null);
        Assert.Equal(0L, host.Values["amount"]);
        Assert.Equal(["change amount", "change amount"], host.Events);
    }

    [Fact]
    public void The_same_number_typed_otherwise_is_no_change_and_shows_as_the_value()
    {
        var (screen, part, host) = Number(new FieldFormat(FormatKind.Decimal, 2), 2.5m);
        Assert.Equal("2.50", part.Editor.Text);
        TextWindow.Type(screen, part.Editor, "2.5");
        screen.Focus(null);
        Assert.Empty(host.Events);
        Assert.Equal("2.50", part.Editor.Text);
    }

    [Fact]
    public void What_code_sets_shows_unmarks_and_is_no_change()
    {
        var (screen, part, host) = Number(new FieldFormat(FormatKind.Decimal, 1), 1.5m);
        TextWindow.Type(screen, part.Editor, "lots");
        screen.Focus(null);
        Assert.Equal("lots", part.Bad);
        host.Values["amount"] = 4m;
        host.Change();
        Assert.Null(part.Bad);
        Assert.Equal("4.0", part.Editor.Text);
        Assert.Empty(host.Events);
    }

    [Fact]
    public void Without_a_format_it_keeps_the_text_and_unbound_in_its_state()
    {
        var host = new TextFactsHost();
        var (screen, _, parts) = Made.Window("""<Number at="1,1" span="3" name="count" onchange="changed" />""", host);
        var part = (NumberPart)parts[0];
        Assert.Null(part.Format);
        TextWindow.Type(screen, part.Editor, "12");
        screen.Focus(null);
        Assert.Equal("12", host.States["count"].Text);
        Assert.Equal(["change count"], host.Events);
    }

    [Fact]
    public void A_field_of_a_number_type_reads_by_its_type()
    {
        var host = new TextFactsHost { Values = { ["count"] = 3L } };
        var (screen, _, parts) = Made.Window("""<Number at="1,1" span="3" name="count" bind="count" />""", host);
        var part = (NumberPart)parts[0];
        Assert.Equal(FormatKind.Integer, part.Format?.Kind);
        TextWindow.Type(screen, part.Editor, "4");
        screen.Focus(null);
        Assert.Equal(4L, host.Values["count"]);
    }
}
