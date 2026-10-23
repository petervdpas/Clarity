// The dropdown's options against the cases every renderer shares
// (conformance/dropdown-options.json), run through the real TypedInput and
// LookupInput: text typed into their box, an offered row picked.

using System.Text.Json;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class DropdownOptionTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "dropdown-options.json"))).RootElement;

    static Choice[] Records() =>
    [
        .. File.GetProperty("search").GetProperty("options").EnumerateArray().Select(o =>
            new Choice(o.GetProperty("id").GetInt64(), o.GetProperty("text").GetString()!, [.. o.GetProperty("fields").EnumerateObject().Select(f => f.Value.GetString()!)]))
    ];

    [AvaloniaFact]
    public void A_typed_dropdown_offers_the_values_its_field_has()
    {
        foreach (var c in File.GetProperty("suggestions").EnumerateArray())
        {
            var values = c.GetProperty("values").EnumerateArray().Select(v => v.GetString()!).ToList();
            var gives = c.GetProperty("gives").EnumerateArray().Select(v => v.GetString()!).ToList();
            var typed = c.GetProperty("typed").GetString()!;
            Assert.Equal(gives, DropdownOptions.Suggestions(values, typed));
            var input = new TypedInput { Values = values };
            input.Box.Text = typed;
            Assert.Equal(gives, input.Offered.Items.Cast<Offered>().Select(o => o.Cells[0]));
        }
    }

    [AvaloniaFact]
    public void A_search_dropdown_offers_the_records_that_hold_every_typed_word()
    {
        foreach (var c in File.GetProperty("search").GetProperty("cases").EnumerateArray())
        {
            var gives = c.GetProperty("gives").EnumerateArray().Select(v => v.GetInt64()).ToList();
            var typed = c.GetProperty("typed").GetString()!;
            Assert.Equal(gives, DropdownOptions.Searched(Records(), typed).Select(r => (long)r.Value!));
            var input = new LookupInput { Choices = Records() };
            input.Box.Text = typed;
            Assert.Equal(gives, input.Offered.Items.Cast<Offered>().Select(o => (long)o.Key!));
        }
    }

    [AvaloniaFact]
    public void Typed_text_is_the_value_and_a_change_only_when_it_differs()
    {
        var input = new TypedInput { Value = "Gent", Values = ["Utrecht", "Gent"] };
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        input.Box.Text = "Rome";
        input.Take();
        Assert.Equal("Rome", input.Value);
        Assert.Equal(1, changed);
        input.Box.Text = "utr";
        input.Pick(input.Offered.Items.Cast<Offered>().Single());
        Assert.Equal("Utrecht", input.Value);
        Assert.Equal("Utrecht", input.Box.Text);
        Assert.Equal(2, changed);
        input.Take();
        Assert.Equal(2, changed);
    }

    [AvaloniaFact]
    public void A_picked_record_is_the_value_and_empty_text_is_none()
    {
        var input = new LookupInput { Choices = Records(), Value = 1L };
        Assert.Equal("Jansen", input.Box.Text);
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        input.Box.Text = "vries";
        var row = input.Offered.Items.Cast<Offered>().Single();
        Assert.Equal(["de Vries", "de Vries", "Utrecht"], row.Cells);
        input.Pick(row);
        Assert.Equal(3L, input.Value);
        Assert.Equal("de Vries", input.Box.Text);
        input.Box.Text = "Jan";
        input.Take();
        Assert.Equal(3L, input.Value);
        Assert.Equal("de Vries", input.Box.Text);
        input.Box.Text = "";
        input.Take();
        Assert.Null(input.Value);
        Assert.Equal(2, changed);
    }
}
