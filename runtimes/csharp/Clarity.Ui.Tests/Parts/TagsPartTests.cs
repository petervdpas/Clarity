// The tags part against the cases every renderer shares
// (conformance/list-fields.json): items typed into its box, picked from an
// enum's values, removed by their chip. Every change gives the record a new
// list of its own type.

using System.Collections;
using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public enum TagsPartGenre { Jazz, Rock, Folk }

public class TagsPartTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "list-fields.json"))).RootElement;

    const string Markup = """<Tags at="1,1" span="8" name="genres" bind="album.genres" onchange="genresChanged" />""";

    /// <summary>The case's items as the list a record holds.</summary>
    static IList Items(string item, JsonElement items) => item switch
    {
        "integer" => items.EnumerateArray().Select(i => i.GetInt64()).ToList(),
        "decimal" => items.EnumerateArray().Select(i => decimal.Parse(i.GetString()!, CultureInfo.InvariantCulture)).ToList(),
        "enum" => items.EnumerateArray().Select(i => Enum.Parse<TagsPartGenre>(i.GetString()!)).ToList(),
        _ => items.EnumerateArray().Select(i => i.GetString()!).ToList(),
    };

    static (Screen Screen, TagsPart Tags, TestHost Host) Make(IList items, string markup = Markup)
    {
        var host = new TestHost();
        host.Values["album.genres"] = items;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<TagsPart>(parts[0]), host);
    }

    static void Click(Screen screen, Box box)
    {
        screen.PointerPressed(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left);
        screen.PointerReleased(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left);
    }

    [Fact]
    public void A_list_of_days_adds_a_day_picked_from_the_calendar()
    {
        var (screen, tags, host) = Make(new List<DateOnly> { new(2026, 10, 1) });
        screen.Focus(tags.Editor);
        Assert.Empty(screen.Overlays);
        // Down in the box opens it; the button inside the box does too.
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        var calendar = Assert.IsType<DayCalendar>(Assert.Single(screen.Overlays).Content);
        // The box keeps the focus: its keys work the calendar.
        Assert.Same(tags.Editor, screen.Focused);
        Click(screen, calendar.CellOf(calendar.Day)!.Value);
        Assert.Empty(screen.Overlays);
        Assert.Equal([new DateOnly(2026, 10, 1), calendar.Day], ((List<DateOnly>)host.Values["album.genres"]!).ToArray());
        // Escape closes it and adds nothing.
        screen.Focus(null);
        screen.Focus(tags.Editor);
        Click(screen, tags.CalendarButton!.Bounds);
        Assert.Single(screen.Overlays);
        Assert.Same(tags.Editor, screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(screen.Overlays);
        Assert.Equal(2, ((List<DateOnly>)host.Values["album.genres"]!).Count);
    }

    [Fact]
    public void A_typed_item_is_added_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("added").EnumerateArray())
        {
            var item = c.GetProperty("item").GetString()!;
            var before = Items(item, c.GetProperty("items"));
            var (_, tags, host) = Make(before);
            var typed = c.GetProperty("typed").GetString()!;
            tags.Editor!.Text = typed;
            tags.Take();
            var refused = c.TryGetProperty("refused", out var r) && r.GetBoolean();
            var gives = refused ? before : Items(item, c.GetProperty("gives"));
            var now = (IList)host.Values["album.genres"]!;
            Assert.Equal(gives.Cast<object>(), now.Cast<object>());
            Assert.Same(before.GetType(), now.GetType());
            Assert.Equal(refused, tags.Bad);
            Assert.Equal(refused ? typed : "", tags.Editor.Text);
            Assert.Equal(gives.Count == before.Count ? 0 : 2, host.Log.Count);
        }
    }

    [Fact]
    public void Its_chips_show_the_items_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("shown").EnumerateArray())
        {
            var item = c.GetProperty("item").GetString()!;
            var host = new TestHost();
            host.Values["album.genres"] = Items(item, c.GetProperty("items"));
            if (c.TryGetProperty("labels", out var labels))
                host.Offered["album.genres"] = [.. labels.EnumerateObject().Select(p => new Choice(Enum.Parse<TagsPartGenre>(p.Name), p.Value.GetString()!))];
            var (_, _, parts) = Made.Window(Markup, host);
            var tags = (TagsPart)parts[0];
            Assert.Equal(c.GetProperty("text").GetString(), string.Join(", ", tags.Chips.Select(chip => chip.Label)));
        }
    }

    [Fact]
    public void An_enum_offers_the_values_the_list_has_not()
    {
        var host = new TestHost();
        host.Values["album.genres"] = new List<TagsPartGenre> { TagsPartGenre.Rock };
        host.Offered["album.genres"] = [new(TagsPartGenre.Jazz, "Jazz music"), new(TagsPartGenre.Rock, "Rock"), new(TagsPartGenre.Folk, "Folk")];
        var (screen, _, parts) = Made.Window(Markup, host);
        var tags = (TagsPart)parts[0];
        Assert.Null(tags.Editor);
        Assert.Equal(["Rock"], tags.Chips.Select(c => c.Label));
        Assert.Equal(["Jazz music", "Folk"], tags.Offered.Select(c => c.Label));
        Click(screen, tags.Adder.Bounds);
        var list = Assert.Single(screen.Overlays);
        Click(screen, new Box(list.Place.X, list.Place.Y + 1 + DropdownList.Line, list.Place.Width, DropdownList.Line));
        Assert.Empty(screen.Overlays);
        Assert.Equal([TagsPartGenre.Rock, TagsPartGenre.Folk], ((IList)host.Values["album.genres"]!).Cast<TagsPartGenre>());
        Assert.Equal(["write genres album.genres System.Collections.Generic.List`1[Clarity.Ui.Tests.TagsPartGenre]", "run genres change genresChanged"], host.Log);
        Assert.Equal(["Jazz music"], tags.Offered.Select(c => c.Label));
    }

    [Fact]
    public void A_chip_removes_its_item_and_a_read_only_one_changes_nothing()
    {
        var (screen, tags, host) = Make(new List<string> { "jazz", "live" });
        Assert.Equal("Remove", tags.Chips[0].Remove!.Name);
        Click(screen, tags.Chips[0].Remove!.Bounds);
        Assert.Equal(["live"], ((IList)host.Values["album.genres"]!).Cast<string>());
        Assert.Equal(["live"], tags.Chips.Select(c => c.Label));
        var (_, readOnly, host2) = Make(new List<string> { "live" }, """<Tags at="1,1" span="8" name="genres" bind="album.genres" readonly="true" />""");
        Assert.Null(readOnly.Chips[0].Remove);
        Assert.Null(readOnly.Editor);
        readOnly.Take();
        Assert.Empty(host2.Log);
    }

    [Fact]
    public void Enter_adds_what_is_typed_and_it_draws()
    {
        var (screen, tags, host) = Make(new List<string> { "jazz" });
        screen.Focus(tags.Editor);
        screen.TextTyped("live");
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal(["jazz", "live"], ((IList)host.Values["album.genres"]!).Cast<string>());
        Assert.Equal("", tags.Editor!.Text);
        using var shot = Shot.Of(screen, 800, 400);
        Assert.True(tags.Chips[1].Bounds.X > tags.Chips[0].Bounds.Right);
        Assert.True(tags.Editor.Bounds.X > tags.Chips[1].Bounds.Right);
    }

    [Fact]
    public void The_place_to_add_is_as_high_as_an_input_box_and_the_chips_sit_in_its_middle()
    {
        var (_, tags, _) = Make(new List<string> { "jazz" });
        var box = tags.Editor!.Bounds;
        Assert.Equal(GridLayout.RowHeight, box.Height);
        Assert.Equal(tags.Bounds.Y, box.Y);
        var chip = tags.Chips[0].Bounds;
        Assert.Equal(box.Y + box.Height / 2, chip.Y + chip.Height / 2, 3);
        var (_, picked, _) = Make(new List<TagsPartGenre> { TagsPartGenre.Jazz });
        Assert.Equal(GridLayout.RowHeight, picked.Adder.Bounds.Height);
    }
}
