// The tags input against the cases every renderer shares
// (conformance/list-fields.json), run through the real TagsInput: items
// typed into its box, picked from an enum's values, removed by their chip.

using System.Collections;
using System.Globalization;
using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Interactivity;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public enum Genre { Jazz, Rock, Folk }

public class TagsCaseTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "list-fields.json"))).RootElement;

    /// <summary>The case's items as the list a record holds.</summary>
    static IList Items(string item, JsonElement items) => item switch
    {
        "integer" => items.EnumerateArray().Select(i => i.GetInt64()).ToList(),
        "decimal" => items.EnumerateArray().Select(i => decimal.Parse(i.GetString()!, CultureInfo.InvariantCulture)).ToList(),
        "enum" => items.EnumerateArray().Select(i => Enum.Parse<Genre>(i.GetString()!)).ToList(),
        _ => items.EnumerateArray().Select(i => i.GetString()!).ToList(),
    };

    static TagItem Kind(string item) => item switch
    {
        "integer" => TagItem.Integer,
        "decimal" => TagItem.Decimal,
        "date" => TagItem.Date,
        "enum" => TagItem.Choice,
        _ => TagItem.Text,
    };

    [AvaloniaFact]
    public void A_typed_item_is_added_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("added").EnumerateArray())
        {
            var item = c.GetProperty("item").GetString()!;
            var before = Items(item, c.GetProperty("items"));
            var input = new TagsInput { Item = Kind(item), Value = before };
            var changed = 0;
            input.ValueChanged += (_, _) => changed++;
            var typed = c.GetProperty("typed").GetString()!;
            input.Box.Text = typed;
            input.Take();
            var refused = c.TryGetProperty("refused", out var r) && r.GetBoolean();
            var gives = refused ? before : Items(item, c.GetProperty("gives"));
            Assert.Equal(gives.Cast<object>(), input.Value!.Cast<object>());
            Assert.Equal(refused, input.Box.Classes.Contains("error"));
            Assert.Equal(refused ? typed : "", input.Box.Text);
            Assert.Equal(gives.Count == before.Count ? 0 : 1, changed);
            Assert.Same(before.GetType(), input.Value.GetType());
        }
    }

    [AvaloniaFact]
    public void A_list_is_shown_as_its_items()
    {
        foreach (var c in File.GetProperty("shown").EnumerateArray())
        {
            var item = c.GetProperty("item").GetString()!;
            var labels = c.TryGetProperty("labels", out var l) ? l.EnumerateObject().ToDictionary(p => p.Name, p => p.Value.GetString()) : [];
            var converter = new ShownConverter(v => v is Genre g ? labels.GetValueOrDefault(g.ToString()) : null);
            Assert.Equal(c.GetProperty("text").GetString(), converter.Convert(Items(item, c.GetProperty("items")), typeof(string), null, CultureInfo.InvariantCulture));
        }
    }

    [AvaloniaFact]
    public void An_enum_offers_the_values_the_list_has_not()
    {
        var input = new TagsInput
        {
            Item = TagItem.Choice,
            Choices = new[] { new Choice(Genre.Jazz, "Jazz music"), new Choice(Genre.Rock, "Rock"), new Choice(Genre.Folk, "Folk") },
            Value = new List<Genre> { Genre.Rock },
        };
        Assert.Equal(["Rock"], input.Chips.Select(c => c.Label.Text));
        Assert.Equal(["Jazz music", "Folk"], input.Offered.Items.Cast<Choice>().Select(c => c.Label));
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        input.Offered.SelectedIndex = 1;
        Assert.Equal([Genre.Rock, Genre.Folk], input.Value!.Cast<Genre>());
        Assert.Equal(1, changed);
        Assert.Equal(["Jazz music"], input.Offered.Items.Cast<Choice>().Select(c => c.Label));
    }

    [AvaloniaFact]
    public void A_chip_removes_its_item()
    {
        var input = new TagsInput { Value = new List<string> { "jazz", "live" }, RemoveText = "Remove" };
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        input.Chips[0].Remove.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Equal(["live"], input.Value!.Cast<string>());
        Assert.Equal(1, changed);
        Assert.Equal(["live"], input.Chips.Select(c => c.Label.Text));
        input.IsReadOnly = true;
        Assert.False(input.Chips[0].Remove.IsVisible);
        input.Box.Text = "more";
        input.Take();
        Assert.Equal(["live"], input.Value!.Cast<string>());
    }
}
