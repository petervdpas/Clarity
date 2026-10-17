// How values are shown, read and filled into text in a locale, against the
// cases every renderer shares (conformance/formats.json); the web renderer
// runs the same cases in app/src/lib/format.test.ts.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class FormatCaseTests
{
    static JsonElement Cases(string part)
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "formats.json");
        return JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty(part);
    }

    static Formats FormatsOf(JsonElement c) =>
        new(new CultureInfo(c.GetProperty("locale").GetString()!), c.GetProperty("currency").GetString());

    static string? Optional(JsonElement e, string name) =>
        e.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;

    static int? Scale(JsonElement e) => e.TryGetProperty("scale", out var v) && v.ValueKind == JsonValueKind.Number ? v.GetInt32() : null;

    static FormatKind Kind(string kind) => kind switch
    {
        "integer" => FormatKind.Integer,
        "decimal" => FormatKind.Decimal,
        "money" => FormatKind.Money,
        "date" => FormatKind.Date,
        "dateTime" => FormatKind.DateTime,
        _ => throw new ArgumentException($"unknown kind {kind}"),
    };

    static FieldFormat Field(JsonElement f) => new(Kind(f.GetProperty("kind").GetString()!), Scale(f), Optional(f, "format"));

    /// <summary>A stored value as the runtime holds it: a long, a decimal, a date or a date and time.</summary>
    static object? Stored(FormatKind kind, JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.Number => value.GetInt64(),
        _ => kind switch
        {
            FormatKind.Date => DateOnly.ParseExact(value.GetString()!, "yyyy-MM-dd", CultureInfo.InvariantCulture),
            FormatKind.DateTime => DateTime.ParseExact(value.GetString()!, "yyyy-MM-dd'T'HH:mm:ss", CultureInfo.InvariantCulture),
            _ => decimal.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        },
    };

    [Fact]
    public void EveryValueIsShownAsTheCasesSay()
    {
        var cases = Cases("shows").EnumerateArray().ToList();
        Assert.True(cases.Count > 50);
        var wrong = new List<string>();
        foreach (var c in cases)
        {
            var field = Field(c.GetProperty("field"));
            var shown = FormatsOf(c).Show(field, Stored(field.Kind, c.GetProperty("value")));
            var expect = c.GetProperty("shows").GetString();
            if (shown != expect)
                wrong.Add($"{c.GetProperty("locale")} {c.GetProperty("what")}: [{shown}] not [{expect}]");
        }
        Assert.True(wrong.Count == 0, string.Join("\n", wrong));
    }

    [Fact]
    public void TypedTextIsReadAsTheCasesSay()
    {
        var wrong = new List<string>();
        foreach (var c in Cases("reads").EnumerateArray())
        {
            var field = Field(c.GetProperty("field"));
            var read = FormatsOf(c).Read(field, c.GetProperty("typed").GetString()!);
            var expect = Stored(field.Kind == FormatKind.Integer ? FormatKind.Integer : FormatKind.Decimal, c.GetProperty("value"));
            if (!Equals(read, expect))
                wrong.Add($"{c.GetProperty("locale")} {c.GetProperty("what")}: [{read}] not [{expect}]");
        }
        Assert.True(wrong.Count == 0, string.Join("\n", wrong));
    }

    static Slot SlotOf(JsonElement s)
    {
        var kind = s.GetProperty("kind").GetString();
        var value = s.GetProperty("value");
        var format = Optional(s, "format");
        return kind switch
        {
            "text" => Slot.OfText(value.GetString()!),
            "integer" => Slot.OfInteger(value.GetInt64(), format),
            "decimal" => Slot.OfDecimal(decimal.Parse(value.GetString()!, CultureInfo.InvariantCulture), Scale(s), format),
            "money" => Slot.OfMoney(decimal.Parse(value.GetString()!, CultureInfo.InvariantCulture), format),
            "date" => Slot.OfDate((DateOnly?)Stored(FormatKind.Date, value), format),
            "dateTime" => Slot.OfDateTime((DateTime?)Stored(FormatKind.DateTime, value), format),
            "bool" => Slot.OfBoolean(value.GetBoolean()),
            "enum" => new Slot(SlotKind.Enum, value.GetString(), EnumName: s.GetProperty("enum").GetString()),
            _ => throw new ArgumentException($"unknown slot kind {kind}"),
        };
    }

    [Fact]
    public void TextIsFilledAsTheCasesSay()
    {
        var wrong = new List<string>();
        foreach (var c in Cases("fills").EnumerateArray())
        {
            var t = c.GetProperty("text");
            var text = new Text(t.GetProperty("text").GetString()!);
            foreach (var v in t.GetProperty("values").EnumerateObject())
                text.With(v.Name, SlotOf(v.Value));
            var labels = c.TryGetProperty("labels", out var l) ? l : default;
            string Label(string enumName, string value) =>
                labels.ValueKind == JsonValueKind.Object ? labels.GetProperty(enumName).GetProperty(value).GetString()! : value;
            var shown = FormatsOf(c).Fill(text, Label);
            var expect = c.GetProperty("shows").GetString();
            if (shown != expect)
                wrong.Add($"{c.GetProperty("locale")} {c.GetProperty("what")}: [{shown}] not [{expect}]");
        }
        Assert.True(wrong.Count == 0, string.Join("\n", wrong));
    }

    [Fact]
    public void WhatAnInputShowsReadsBackAsTheSameValue()
    {
        var f = new Formats(new CultureInfo("nl-NL"), "EUR");
        var money = new FieldFormat(FormatKind.Money);
        Assert.Equal("1.250,50", f.EditText(money, 1250.5m));
        Assert.Equal(1250.5m, f.Read(money, f.EditText(money, 1250.5m)));
        var percent = new FieldFormat(FormatKind.Decimal, 4, "percent");
        Assert.Equal(0.1234m, f.Read(percent, f.EditText(percent, 0.1234m)));
    }

    [Fact]
    public void MoneyWithoutACurrencyKeepsTwoDecimalsAndNoSign()
    {
        Assert.Equal("3.00", new Formats(new CultureInfo("en-US"), null).Show(new FieldFormat(FormatKind.Money), 3m));
    }

    [Fact]
    public void NoValueShowsAsNothing()
    {
        var f = new Formats(new CultureInfo("en-US"), "EUR");
        Assert.Equal("", f.Show(new FieldFormat(FormatKind.Date), null));
        Assert.Null(f.Read(new FieldFormat(FormatKind.Decimal, 2), "  "));
    }
}
