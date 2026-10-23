// A slider and a spin box against the cases every renderer shares
// (conformance/ranges.json), run through the real SliderInput and
// NumberInput: keys pressed, a slider let go, numbers typed.

using System.Globalization;
using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class RangeCaseTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "ranges.json"))).RootElement;

    static string? Text(JsonElement c, string name) => c.GetProperty(name).ValueKind == JsonValueKind.Null ? null : c.GetProperty(name).GetString();

    static decimal Number(string text) => decimal.Parse(text, CultureInfo.InvariantCulture);

    static object? Held(string? text, bool whole) => text is null ? null : NumberRange.Held(Number(text), whole);

    static readonly Dictionary<string, RangeKey> Keys = new()
    {
        ["up"] = RangeKey.Up,
        ["down"] = RangeKey.Down,
        ["pageUp"] = RangeKey.PageUp,
        ["pageDown"] = RangeKey.PageDown,
        ["home"] = RangeKey.Home,
        ["end"] = RangeKey.End,
    };

    static NumberInput Spin(JsonElement c, string? step)
    {
        var whole = c.GetProperty("whole").GetBoolean();
        return new NumberInput
        {
            Format = new FieldFormat(whole ? FormatKind.Integer : FormatKind.Decimal, whole ? null : 3),
            Min = Text(c, "min"),
            Max = Text(c, "max"),
            Step = step,
        };
    }

    [AvaloniaFact]
    public void A_key_moves_a_spin_box_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("keys").EnumerateArray())
        {
            var whole = c.GetProperty("whole").GetBoolean();
            var input = Spin(c, Text(c, "step"));
            input.Value = Held(Text(c, "value"), whole);
            input.Spin(Keys[c.GetProperty("key").GetString()!]);
            Assert.Equal(Number(Text(c, "after")!), NumberRange.Number(input.Value));
            Assert.Equal(whole, input.Value is long);
        }
    }

    [AvaloniaFact]
    public void A_key_moves_a_slider_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("keys").EnumerateArray().Where(c => Text(c, "min") is not null && Text(c, "max") is not null))
        {
            var whole = c.GetProperty("whole").GetBoolean();
            var slider = new SliderInput { Min = Text(c, "min"), Max = Text(c, "max"), Step = Text(c, "step"), Whole = whole };
            slider.Value = Held(Text(c, "value"), whole);
            slider.Press(Keys[c.GetProperty("key").GetString()!]);
            Assert.Equal(Number(Text(c, "after")!), NumberRange.Number(slider.Value));
        }
    }

    [AvaloniaFact]
    public void A_slider_takes_the_nearest_step_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("drag").EnumerateArray())
        {
            var whole = c.GetProperty("whole").GetBoolean();
            var slider = new SliderInput { Min = Text(c, "min"), Max = Text(c, "max"), Step = Text(c, "step"), Whole = whole };
            var changed = 0;
            slider.ValueChanged += (_, _) => changed++;
            slider.DropAt(Number(Text(c, "at")!));
            Assert.Equal(Number(Text(c, "value")!), NumberRange.Number(slider.Value));
            Assert.Equal(whole, slider.Value is long);
            Assert.Equal(1, changed);
        }
    }

    [AvaloniaFact]
    public void A_typed_number_stays_within_the_range_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("typed").EnumerateArray())
        {
            var input = Spin(c, "1");
            input.Text = Text(c, "value")!.Replace(".", CultureInfo.CurrentCulture.NumberFormat.NumberDecimalSeparator);
            input.Take();
            Assert.Equal(Number(Text(c, "after")!), NumberRange.Number(input.Value));
        }
    }

    [AvaloniaFact]
    public void Only_a_value_that_differs_is_a_change_and_the_arrow_keys_spin()
    {
        var input = new NumberInput { Format = new FieldFormat(FormatKind.Integer), Min = "1", Max = "3" };
        input.Value = 3L;
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        input.Spin(RangeKey.Up);
        Assert.Equal(0, changed);
        input.RaiseEvent(new KeyEventArgs { RoutedEvent = InputElement.KeyDownEvent, Key = Key.Down });
        Assert.Equal(2L, input.Value);
        Assert.Equal(1, changed);
        Assert.NotNull(input.InnerRightContent);
        Assert.Null(new NumberInput().InnerRightContent);
    }
}
