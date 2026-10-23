// Clarity's icons (crates/clarity-spec/icons/lucide.json) as Avalonia draws
// them: every path parses as a geometry inside the 24 by 24 box, and an
// icon's line takes the foreground of what holds it, as the generated
// style Path.icon binds it.

using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Headless.XUnit;
using Avalonia.Media;
using Avalonia.Threading;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class IconTests
{
    static readonly string Icons = System.IO.Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "..", "crates", "clarity-spec", "icons", "lucide.json");

    [AvaloniaFact]
    public void Every_icon_parses_as_a_geometry_in_its_box()
    {
        var icons = JsonSerializer.Deserialize<Dictionary<string, string>>(File.ReadAllText(Icons))!;
        Assert.True(icons.Count > 1800);
        var outside = new List<string>();
        foreach (var (name, path) in icons)
        {
            var bounds = Geometry.Parse(path).Bounds;
            if (bounds.X < -1 || bounds.Y < -1 || bounds.Right > 25 || bounds.Bottom > 25)
                outside.Add($"{name} {bounds}");
        }
        Assert.Empty(outside);
    }

    [AvaloniaFact]
    public void An_icons_line_takes_the_foreground_of_its_button()
    {
        var line = new global::Avalonia.Controls.Shapes.Path { Data = Geometry.Parse("M 20 6 L 9 17 l -5 -5") };
        // As `{Binding $self.(TextElement.Foreground)}` in the generated style binds it.
        line.Bind(global::Avalonia.Controls.Shapes.Shape.StrokeProperty, line.GetObservable(TextElement.ForegroundProperty));
        var button = new Button { Foreground = Brushes.Crimson, Content = new Viewbox { Child = new Canvas { Width = 24, Height = 24, Children = { line } } } };
        var window = new Window { Content = button };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        Assert.Equal(Brushes.Crimson, line.Stroke);
        window.Close();
    }
}
