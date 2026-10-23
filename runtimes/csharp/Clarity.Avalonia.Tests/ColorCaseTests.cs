// The color input against the cases every renderer shares
// (conformance/colors.json), run through the real ColorInput: colors
// typed into its box, picked from its palette.

using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Interactivity;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class ColorCaseTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "colors.json"))).RootElement;

    [AvaloniaFact]
    public void A_typed_color_is_read_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("read").EnumerateArray())
        {
            var typed = c.GetProperty("typed").GetString()!;
            var color = c.GetProperty("color").ValueKind == JsonValueKind.Null ? null : c.GetProperty("color").GetString();
            Assert.Equal(color, Palette.Read(typed));
            var input = new ColorInput { Value = "#123456" };
            var changed = 0;
            input.ValueChanged += (_, _) => changed++;
            input.Box.Text = typed;
            input.Take();
            Assert.Equal(color ?? "#123456", input.Value);
            Assert.Equal(color is null, input.Box.Classes.Contains("error"));
            Assert.Equal(color is null ? 0 : 1, changed);
        }
    }

    [AvaloniaFact]
    public void The_palette_is_the_one_every_renderer_offers()
    {
        var rows = File.GetProperty("palette").EnumerateArray().Select(r => r.EnumerateArray().Select(c => c.GetString()!).ToList()).ToList();
        Assert.Equal(rows, Palette.Rows.Select(r => r.ToList()).ToList());
        Assert.Equal(60, new ColorInput().Choices.Count);
    }

    [AvaloniaFact]
    public void A_picked_color_is_a_change_only_when_it_differs()
    {
        var input = new ColorInput { Value = "#3b82f6" };
        var changed = 0;
        input.ValueChanged += (_, _) => changed++;
        var blue = input.Choices.Single(b => Equals(b.Tag, "#3b82f6"));
        Assert.Contains("chosen", blue.Classes);
        blue.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Equal(0, changed);
        input.Choices.Single(b => Equals(b.Tag, "#ef4444")).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        Assert.Equal("#ef4444", input.Value);
        Assert.Equal(1, changed);
        input.IsReadOnly = true;
        input.Pick("#000000");
        Assert.Equal("#ef4444", input.Value);
    }
}
