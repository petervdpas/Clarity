// A spin box against the cases every stack shares (conformance/ranges.json):
// its keys move it a step, or ten, within its range; a typed number is kept
// within it; only a value that differs is a change; the arrows spin it.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class SpinBoxPartTests
{
    static readonly JsonElement Ranges = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "ranges.json"))).RootElement;

    static string? Text(JsonElement c, string name) => c.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;

    static decimal Number(string text) => decimal.Parse(text, CultureInfo.InvariantCulture);

    static readonly Dictionary<string, Key> Keys = new()
    {
        ["up"] = Key.Up,
        ["down"] = Key.Down,
        ["pageUp"] = Key.PageUp,
        ["pageDown"] = Key.PageDown,
    };

    static (Screen Screen, NumberPart Part, TextFactsHost Host) Spin(JsonElement c, string? step, bool fieldRange = false)
    {
        var whole = c.GetProperty("whole").GetBoolean();
        var host = new TextFactsHost();
        host.Formatted["level"] = new FieldFormat(whole ? FormatKind.Integer : FormatKind.Decimal, whole ? null : 3);
        host.Values["level"] = Text(c, "value") is { } v ? NumberRange.Held(Number(v), whole) : null;
        string Attr(string name, string? value) => value is null ? "" : $" {name}=\"{value}\"";
        var markup = fieldRange
            ? $"""<Spinbox at="1,1" span="3" name="level" bind="level"{Attr("step", step)} onchange="changed" />"""
            : $"""<Spinbox at="1,1" span="3" name="level" bind="level"{Attr("min", Text(c, "min"))}{Attr("max", Text(c, "max"))}{Attr("step", step)} onchange="changed" />""";
        if (fieldRange)
            host.Ranges["level"] = (Text(c, "min"), Text(c, "max"));
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, (NumberPart)parts[0], host);
    }

    [Fact]
    public void A_key_moves_a_spin_box_as_on_every_renderer()
    {
        foreach (var c in Ranges.GetProperty("keys").EnumerateArray())
        {
            var whole = c.GetProperty("whole").GetBoolean();
            var key = c.GetProperty("key").GetString()!;
            var (screen, part, host) = Spin(c, Text(c, "step"));
            if (Keys.TryGetValue(key, out var pressed))
            {
                screen.Focus(part.Editor);
                screen.KeyPressed(new KeyInput(pressed, Modifiers.None));
            }
            else
                part.Spin(key == "home" ? RangeKey.Home : RangeKey.End);
            var after = Number(Text(c, "after")!);
            Assert.True(after == NumberRange.Number(host.Values["level"]), $"{key} from {Text(c, "value")}: {host.Values["level"]}");
            Assert.Equal(whole, host.Values["level"] is long or null);
        }
    }

    [Fact]
    public void A_typed_number_stays_within_the_range_as_on_every_renderer()
    {
        foreach (var c in Ranges.GetProperty("typed").EnumerateArray())
        {
            var (screen, part, host) = Spin(c, "1");
            host.Values["level"] = null;
            host.Change();
            TextWindow.Type(screen, part.Editor, Text(c, "value")!);
            screen.Focus(null);
            Assert.Equal(Number(Text(c, "after")!), NumberRange.Number(host.Values["level"]));
        }
    }

    [Fact]
    public void Without_its_own_range_it_takes_the_fields()
    {
        var c = Ranges.GetProperty("typed")[1];
        var (screen, part, host) = Spin(c, "1", fieldRange: true);
        TextWindow.Type(screen, part.Editor, "500");
        screen.Focus(null);
        Assert.Equal(99L, host.Values["level"]);
    }

    [Fact]
    public void Only_a_value_that_differs_is_a_change_and_the_arrows_spin()
    {
        var host = new TextFactsHost { Values = { ["level"] = 3L } };
        var (screen, _, parts) = Made.Window("""<Spinbox at="1,1" span="3" name="level" bind="level" min="1" max="3" onchange="changed" />""", host);
        var spin = (NumberPart)parts[0];
        spin.Spin(RangeKey.Up);
        Assert.Empty(host.Events);
        var down = spin.Down!.Bounds;
        screen.PointerPressed(down.X + 3, down.Y + 3, PointerButton.Left);
        screen.PointerReleased(down.X + 3, down.Y + 3, PointerButton.Left);
        Assert.Equal(2L, host.Values["level"]);
        Assert.Equal(["change level"], host.Events);
        Assert.Equal("2", spin.Editor.Text);
        // Left and right still move the caret.
        screen.Focus(spin.Editor);
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        Assert.Equal(0, spin.Editor.Caret);
        Assert.Equal(2L, host.Values["level"]);
    }

    [Fact]
    public void A_number_box_has_no_arrows_and_a_read_only_spin_box_does_not_move()
    {
        var host = new TextFactsHost { Values = { ["level"] = 3L } };
        var (_, _, parts) = Made.Window("""
            <Number at="1,1" span="3" name="count" bind="level" />
            <Spinbox at="5,1" span="3" name="level" bind="level" />
            """, host);
        Assert.Null(((NumberPart)parts[0]).Up);
        Assert.Null(((NumberPart)parts[0]).Range);
        var spin = (NumberPart)parts[1];
        host.State("level")!.IsReadOnly = true;
        spin.Spin(RangeKey.Up);
        Assert.Equal(3L, host.Values["level"]);
    }

    [Fact]
    public void An_unbound_spin_box_keeps_its_number_as_text()
    {
        var host = new TextFactsHost();
        var (_, _, parts) = Made.Window("""<Spinbox at="1,1" span="3" name="copies" min="1" max="9" />""", host);
        var spin = (NumberPart)parts[0];
        spin.Spin(RangeKey.Up);
        Assert.Equal("2", host.States["copies"].Text);
        spin.Spin(RangeKey.End);
        Assert.Equal("9", host.States["copies"].Text);
    }
}
