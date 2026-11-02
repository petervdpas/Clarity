// The color part against the cases every renderer shares
// (conformance/colors.json): colors typed into its box, picked from the
// palette its swatch opens. Only a color that differs is a change; text
// that is no color stays as typed and is marked, and the value stays.

using System.Text.Json;

namespace Clarity.Ui.Tests;

public class ColorPartTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "colors.json"))).RootElement;

    const string Markup = """<Color at="1,1" span="4" name="tint" bind="tag.color" onchange="tintChanged" />""";

    static (Screen Screen, ColorPart Color, TestHost Host) Make(string value = "#123456", string markup = Markup)
    {
        var host = new TestHost();
        host.Values["tag.color"] = value;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<ColorPart>(parts[0]), host);
    }

    static void Click(Screen screen, double x, double y)
    {
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    [Fact]
    public void A_typed_color_is_read_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("read").EnumerateArray())
        {
            var typed = c.GetProperty("typed").GetString()!;
            var color = c.GetProperty("color").ValueKind == JsonValueKind.Null ? null : c.GetProperty("color").GetString();
            var (_, part, host) = Make();
            part.Editor.Text = typed;
            part.Take();
            Assert.Equal(color ?? "#123456", host.Values["tag.color"]);
            Assert.Equal(color is null, part.Bad);
            Assert.Equal(color is null ? [] : new[] { $"write tint tag.color {color}", "run tint change tintChanged" }, host.Log);
            if (color is null)
                Assert.Equal(typed, part.Editor.Text);
        }
    }

    [Fact]
    public void The_palette_is_the_one_every_renderer_offers_and_opens_below_the_swatch()
    {
        var rows = File.GetProperty("palette").EnumerateArray().Select(r => r.EnumerateArray().Select(c => c.GetString()!).ToList()).ToList();
        var (screen, part, _) = Make();
        Click(screen, part.Swatch.Bounds.X + 4, part.Swatch.Bounds.Y + 4);
        var open = Assert.Single(screen.Overlays);
        var palette = Assert.IsType<PaletteGrid>(open.Content);
        Assert.Equal(rows.SelectMany(r => r), palette.Colors);
        Assert.Equal(60, palette.Colors.Count);
        Assert.True(open.Place.Y >= part.Bounds.Bottom);
        Click(screen, part.Swatch.Bounds.X + 4, part.Swatch.Bounds.Y + 4);
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void A_picked_color_is_a_change_only_when_it_differs()
    {
        var (screen, part, host) = Make("#3b82f6");
        Click(screen, part.Swatch.Bounds.X + 4, part.Swatch.Bounds.Y + 4);
        var palette = (PaletteGrid)screen.Overlays[0].Content;
        Assert.Equal("#3b82f6", palette.Chosen);
        var blue = palette.Place("#3b82f6");
        Click(screen, blue.X + blue.Width / 2, blue.Y + blue.Height / 2);
        Assert.Empty(screen.Overlays);
        Assert.Empty(host.Log);
        Click(screen, part.Swatch.Bounds.X + 4, part.Swatch.Bounds.Y + 4);
        var red = ((PaletteGrid)screen.Overlays[0].Content).Place("#ef4444");
        Click(screen, red.X + red.Width / 2, red.Y + red.Height / 2);
        Assert.Equal("#ef4444", host.Values["tag.color"]);
        Assert.Equal(["write tint tag.color #ef4444", "run tint change tintChanged"], host.Log);
        Assert.Equal("#ef4444", part.Editor.Text);
    }

    [Fact]
    public void What_code_sets_is_no_change_and_a_read_only_one_opens_nothing()
    {
        var (screen, part, host) = Make();
        host.Values["tag.color"] = "#000000";
        host.Change();
        screen.Draw(new SKCanvas(new SKBitmap(10, 10)));
        Assert.Equal("#000000", part.Editor.Text);
        Assert.Empty(host.Log);
        var (screen2, readOnly, host2) = Make(markup: """<Color at="1,1" span="4" name="tint" bind="tag.color" readonly="true" />""");
        Click(screen2, readOnly.Swatch.Bounds.X + 4, readOnly.Swatch.Bounds.Y + 4);
        Assert.Empty(screen2.Overlays);
        readOnly.Editor.Text = "#ffffff";
        readOnly.Take();
        Assert.Empty(host2.Log);
    }

    [Fact]
    public void Enter_takes_the_typed_color_and_Escape_closes_the_palette()
    {
        var (screen, part, host) = Make();
        screen.Focus(part.Editor);
        part.Editor.SelectAll();
        screen.TextTyped("f0a");
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal("#ff00aa", host.Values["tag.color"]);
        Click(screen, part.Swatch.Bounds.X + 4, part.Swatch.Bounds.Y + 4);
        Assert.Single(screen.Overlays);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void The_swatch_shows_the_color_picked()
    {
        var (screen, part, _) = Make("#ff0000");
        using var shot = Shot.Of(screen, 800, 400);
        var middle = shot.PixelAt((int)(part.Swatch.Bounds.X + part.Swatch.Bounds.Width / 2), (int)(part.Swatch.Bounds.Y + part.Swatch.Bounds.Height / 2));
        Assert.Equal((255, 0, 0), (middle.Red, middle.Green, middle.Blue));
    }
}
