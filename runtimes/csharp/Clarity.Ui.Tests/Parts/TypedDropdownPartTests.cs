// The typed and the search dropdown against the cases every renderer shares
// (conformance/dropdown-options.json): text typed into their box, an offered
// row picked from the list below it. A typed dropdown takes any text; a
// search dropdown a record picked, and empty text is none. Only a value
// that differs is a change.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class TypedDropdownPartTests
{
    internal static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "dropdown-options.json"))).RootElement;

    const string Markup = """<Dropdown at="1,1" span="4" name="city" bind="person.city" typed="true" onchange="cityChanged" />""";

    static (Screen Screen, TypedDropdownPart Part, TestHost Host) Make(IEnumerable<string> values, string? value = null)
    {
        var host = new TestHost();
        host.Values["person.city"] = value;
        host.Offered["person.city"] = [.. values.Select(v => new Choice(v, v))];
        var (screen, _, parts) = VariantMade.Window(Markup, host);
        return (screen, Assert.IsType<TypedDropdownPart>(parts[0]), host);
    }

    [Fact]
    public void A_typed_dropdown_offers_the_values_its_field_has()
    {
        foreach (var c in File.GetProperty("suggestions").EnumerateArray())
        {
            var values = c.GetProperty("values").EnumerateArray().Select(v => v.GetString()!).ToList();
            var gives = c.GetProperty("gives").EnumerateArray().Select(v => v.GetString()!).ToList();
            var (screen, part, _) = Make(values);
            screen.Focus(part.Editor);
            screen.TextTyped(c.GetProperty("typed").GetString()!);
            Assert.Equal(gives, part.Offered.Select(o => o.Label));
            Assert.Equal(gives.Count > 0, part.List is not null);
        }
    }

    [Fact]
    public void Typed_text_is_the_value_and_a_change_only_when_it_differs()
    {
        var (screen, part, host) = Make(["Utrecht", "Gent"], "Gent");
        Assert.Equal("Gent", part.Editor.Text);
        screen.Focus(part.Editor);
        part.Editor.SelectAll();
        screen.TextTyped("Rome");
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal("Rome", host.Values["person.city"]);
        Assert.Equal(["write city person.city Rome", "run city change cityChanged"], host.Log);
        part.Editor.SelectAll();
        screen.TextTyped("utr");
        var list = Assert.Single(screen.Overlays);
        screen.PointerMoved(list.Place.X + 10, list.Place.Y + 1 + DropdownList.Line / 2);
        screen.PointerPressed(list.Place.X + 10, list.Place.Y + 1 + DropdownList.Line / 2, PointerButton.Left);
        screen.PointerReleased(list.Place.X + 10, list.Place.Y + 1 + DropdownList.Line / 2, PointerButton.Left);
        Assert.Equal("Utrecht", host.Values["person.city"]);
        Assert.Equal("Utrecht", part.Editor.Text);
        Assert.Empty(screen.Overlays);
        Assert.Equal(4, host.Log.Count);
        Assert.Same(part.Editor, screen.Focused);
        part.Take();
        Assert.Equal(4, host.Log.Count);
    }

    [Fact]
    public void Down_and_Enter_pick_and_Escape_puts_the_text_back()
    {
        var (screen, part, host) = Make(["Utrecht", "amsterdam", "Gent"], "Gent");
        screen.Focus(part.Editor);
        Assert.Single(screen.Overlays);
        part.Editor.SelectAll();
        screen.TextTyped("e");
        Assert.Equal(["amsterdam", "Gent", "Utrecht"], part.Offered.Select(o => o.Label));
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(screen.Overlays);
        Assert.Equal("Gent", part.Editor.Text);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal("Gent", host.Values["person.city"]);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_read_only_one_offers_nothing_and_it_draws()
    {
        var host = new TestHost();
        host.Values["person.city"] = "Gent";
        host.Offered["person.city"] = [new("Utrecht", "Utrecht")];
        var (screen, _, parts) = VariantMade.Window("""<Dropdown at="1,1" span="4" name="city" bind="person.city" typed="true" readonly="true" />""", host);
        var part = (TypedDropdownPart)parts[0];
        screen.Focus(part.Editor);
        Assert.Empty(screen.Overlays);
        part.Editor.Text = "Rome";
        part.Take();
        Assert.Empty(host.Log);
        using var shot = Shot.Of(screen, 800, 400);
    }
}

public class SearchDropdownPartTests
{
    const string Markup = """<Dropdown at="1,1" span="4" name="owner" bind="task.owner" search="true" onchange="ownerChanged" />""";

    static List<Choice> Records() =>
    [
        new(null, ""),
        .. TypedDropdownPartTests.File.GetProperty("search").GetProperty("options").EnumerateArray().Select(o =>
            new Choice(o.GetProperty("id").GetInt64(), o.GetProperty("text").GetString()!, [.. o.GetProperty("fields").EnumerateObject().Select(f => f.Value.GetString()!)])),
    ];

    static (Screen Screen, SearchDropdownPart Part, TestHost Host) Make(object? value = null)
    {
        var host = new TestHost();
        host.Values["task.owner"] = value;
        host.Offered["task.owner"] = Records();
        host.Fields["task.owner"] = new FieldFacts("Owner", Refers: "Person");
        var (screen, _, parts) = VariantMade.Window(Markup, host);
        return (screen, Assert.IsType<SearchDropdownPart>(parts[0]), host);
    }

    [Fact]
    public void A_search_dropdown_offers_the_records_that_hold_every_typed_word()
    {
        foreach (var c in TypedDropdownPartTests.File.GetProperty("search").GetProperty("cases").EnumerateArray())
        {
            var gives = c.GetProperty("gives").EnumerateArray().Select(v => v.GetInt64()).ToList();
            var (screen, part, _) = Make();
            screen.Focus(part.Editor);
            screen.TextTyped(c.GetProperty("typed").GetString()!);
            Assert.Equal(gives, part.Offered.Select(o => (long)o.Value!));
        }
    }

    [Fact]
    public void A_records_fields_show_as_columns_in_the_list()
    {
        var (screen, part, _) = Make();
        Shot.Of(screen, 800, 400);
        screen.Focus(part.Editor);
        var list = Assert.IsType<DropdownList>(Assert.Single(screen.Overlays).Content);
        Shot.Of(screen, 800, 400);
        var first = list.CellBoxes(0);
        Assert.Equal(2, first.Count);
        // Each field starts where its column starts, in every row.
        Assert.True(first[1].X > first[0].Right);
        Assert.Equal(first[1].X, list.CellBoxes(1)[1].X);
    }

    [Fact]
    public void A_picked_record_is_the_value_and_empty_text_is_none()
    {
        var (screen, part, host) = Make(1L);
        Assert.Equal("Jansen", part.Editor.Text);
        screen.Focus(part.Editor);
        part.Editor.SelectAll();
        screen.TextTyped("vries");
        var row = Assert.Single(part.Offered);
        Assert.Equal("de Vries, Utrecht", SearchDropdownPart.Line(row));
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal(3L, host.Values["task.owner"]);
        Assert.Equal("de Vries", part.Editor.Text);
        part.Editor.SelectAll();
        screen.TextTyped("Jan");
        part.Take();
        Assert.Equal(3L, host.Values["task.owner"]);
        Assert.Equal("de Vries", part.Editor.Text);
        part.Editor.Text = "";
        part.Take();
        Assert.Null(host.Values["task.owner"]);
        Assert.Equal(["write owner task.owner 3", "run owner change ownerChanged", "write owner task.owner ", "run owner change ownerChanged"], host.Log);
    }

    [Fact]
    public void Only_a_reference_searches()
    {
        var host = new TestHost();
        host.Offered["task.kind"] = [new("a", "A")];
        var (_, _, parts) = VariantMade.Window("""<Dropdown at="1,1" span="4" bind="task.kind" search="true" />""", host);
        Assert.IsType<DropdownPart>(parts[0]);
    }
}

public class VariantsInWindowsTests
{
    [Fact]
    public void A_window_makes_the_variant_that_fits()
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window("""
            <Dropdown at="1,1" span="4" bind="city" typed="true" />
            <Image at="1,2" span="4" bind="person.photo" />
            <Dropdown at="1,3" span="4" bind="city" />
            """, host);
        Assert.IsType<TypedDropdownPart>(parts[0]);
        Assert.IsType<PictureFieldPart>(parts[1]);
        Assert.IsType<DropdownPart>(parts[2]);
    }
}
