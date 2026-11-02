// The slider against the cases every renderer shares (conformance/ranges.json):
// its keys move a step, ten or to an end; let go, it takes the nearest step.
// Only a value that differs is a change; a drag writes nothing until it is
// let go; unbound it keeps its value in its state's text.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class SliderPartTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "ranges.json"))).RootElement;

    static string? Text(JsonElement c, string name) => c.GetProperty(name).ValueKind == JsonValueKind.Null ? null : c.GetProperty(name).GetString();

    static decimal Number(string text) => decimal.Parse(text, CultureInfo.InvariantCulture);

    static readonly Dictionary<string, Key> Keys = new()
    {
        ["up"] = Key.Up,
        ["down"] = Key.Down,
        ["pageUp"] = Key.PageUp,
        ["pageDown"] = Key.PageDown,
        ["home"] = Key.Home,
        ["end"] = Key.End,
    };

    static (TestHost Host, Screen Screen, SliderPart Slider) Slider(JsonElement c, string? value)
    {
        var host = new TestHost();
        var whole = c.GetProperty("whole").GetBoolean();
        host.Values["v"] = value is null ? null : NumberRange.Held(Number(value), whole);
        var range = string.Concat(new[] { ("min", Text(c, "min")), ("max", Text(c, "max")), ("step", Text(c, "step")) }.Where(p => p.Item2 is not null).Select(p => $" {p.Item1}=\"{p.Item2}\""));
        var (screen, _, parts) = Made.Window($"""<Slider at="1,1" span="6" name="level" bind="v"{range} onchange="moved" />""", host);
        return (host, screen, (SliderPart)parts[0]);
    }

    [Fact]
    public void A_key_moves_it_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("keys").EnumerateArray().Where(c => Text(c, "min") is not null && Text(c, "max") is not null))
        {
            var (host, screen, slider) = Slider(c, Text(c, "value"));
            screen.Focus(slider);
            screen.KeyPressed(new KeyInput(Keys[c.GetProperty("key").GetString()!], Modifiers.None));
            var after = Number(Text(c, "after")!);
            Assert.Equal(after, slider.Value);
            Assert.Equal(c.GetProperty("whole").GetBoolean(), host.Values["v"] is long);
            var changed = Text(c, "value") is not { } before || Number(before) != after;
            Assert.Equal(changed ? 1 : 0, host.Log.Count(l => l.StartsWith("run level change")));
        }
    }

    [Fact]
    public void Let_go_it_takes_the_nearest_step_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("drag").EnumerateArray())
        {
            var (_, _, slider) = Slider(c, null);
            slider.DropAt(Number(Text(c, "at")!));
            Assert.Equal(Number(Text(c, "value")!), slider.Value);
        }
    }

    [Fact]
    public void A_drag_writes_nothing_until_it_is_let_go()
    {
        var host = new TestHost();
        host.Values["v"] = 0L;
        var (screen, _, parts) = Made.Window("""<Slider at="1,1" span="6" name="level" bind="v" min="0" max="10" onchange="moved" />""", host);
        var slider = (SliderPart)parts[0];
        var y = slider.Bounds.Y + 18;
        screen.PointerPressed(slider.Bounds.X + 2, y, PointerButton.Left);
        screen.PointerMoved(slider.Bounds.X + slider.Bounds.Width / 2, y);
        Assert.Equal(5m, Math.Round(slider.Shown));
        Assert.Empty(host.Log);
        screen.PointerMoved(slider.Bounds.Right + 50, y);
        screen.PointerReleased(slider.Bounds.Right + 50, y, PointerButton.Left);
        Assert.Equal(10L, host.Values["v"]);
        Assert.Equal(["write level v 10", "run level change moved"], host.Log);
    }

    [Fact]
    public void Unbound_it_keeps_its_value_in_its_state()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Slider at="1,1" span="6" name="level" min="0" max="1" step="0.1" />""", host);
        screen.Focus(parts[0]);
        screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
        Assert.Equal("0.2", host.State("level")!.Text);
    }

    [Fact]
    public void A_read_only_slider_does_not_move_and_focus_tells_enter_and_leave()
    {
        var host = new TestHost();
        host.Values["v"] = 3L;
        var (screen, _, parts) = Made.Window("""<Slider at="1,1" span="6" name="level" bind="v" readonly="true" onenter="came" onleave="went" />""", host);
        screen.Focus(parts[0]);
        screen.KeyPressed(new KeyInput(Key.Up, Modifiers.None));
        screen.Focus(null);
        Assert.Equal(3L, host.Values["v"]);
        Assert.Equal(["run level enter came", "run level leave went"], host.Log);
    }
}
