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

    /// The swatch shows the color picked, as a filled square beside its text.
    [global::Avalonia.Headless.XUnit.AvaloniaFact]
    public void The_swatch_shows_the_color_picked()
    {
        var input = new ColorInput { Value = "#ff0000" };
        var window = new global::Avalonia.Controls.Window { Width = 200, Height = 40, Content = input };
        // As a generated app's theme: a button in a grid centers what it holds.
        var centered = new global::Avalonia.Styling.Style(x => global::Avalonia.Styling.Selectors.OfType(global::Avalonia.Styling.Selectors.Child(global::Avalonia.Styling.Selectors.OfType(x, typeof(global::Avalonia.Controls.Grid))), typeof(global::Avalonia.Controls.Button)));
        centered.Setters.Add(new global::Avalonia.Styling.Setter(global::Avalonia.Controls.ContentControl.HorizontalContentAlignmentProperty, global::Avalonia.Layout.HorizontalAlignment.Center));
        centered.Setters.Add(new global::Avalonia.Styling.Setter(global::Avalonia.Controls.ContentControl.VerticalContentAlignmentProperty, global::Avalonia.Layout.VerticalAlignment.Center));
        window.Styles.Add(centered);
        window.Show();
        global::Avalonia.Threading.Dispatcher.UIThread.RunJobs();
        var frame = global::Avalonia.Headless.HeadlessWindowExtensions.CaptureRenderedFrame(window)!;
        var pixels = new byte[frame.PixelSize.Width * frame.PixelSize.Height * 4];
        var handle = System.Runtime.InteropServices.GCHandle.Alloc(pixels, System.Runtime.InteropServices.GCHandleType.Pinned);
        try
        {
            frame.CopyPixels(new global::Avalonia.PixelRect(frame.PixelSize), handle.AddrOfPinnedObject(), pixels.Length, frame.PixelSize.Width * 4);
        }
        finally
        {
            handle.Free();
        }
        // The middle of the swatch, 32 pixels wide at the left: red, green, blue.
        var at = (frame.PixelSize.Height / 2 * frame.PixelSize.Width + 16) * 4;
        Assert.Equal((255, 0, 0), (pixels[at], pixels[at + 1], pixels[at + 2]));
        window.Close();
    }
}
