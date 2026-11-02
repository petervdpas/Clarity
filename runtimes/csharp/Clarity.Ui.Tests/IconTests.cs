// Clarity's icons as Clarity.UI draws them (IconTests of Avalonia): every
// path of crates/clarity-spec/icons/lucide.json parses inside its 24 by 24
// box, and an icon's line takes the foreground of the button it is on (a
// disabled one's dimmed). How the C# packages are split is SplitTests'.

using System.Text.Json;

namespace Clarity.Ui.Tests;

public class IconTests
{
    /// <summary>The repository the tests were built in: the folder above the test binaries that holds crates/.</summary>
    static string Repo()
    {
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
            if (Directory.Exists(Path.Combine(dir.FullName, "crates", "clarity-spec")))
                return dir.FullName;
        throw new DirectoryNotFoundException("no repository above the tests");
    }

    [Fact]
    public void Every_icon_parses_as_a_geometry_in_its_box()
    {
        var icons = JsonSerializer.Deserialize<Dictionary<string, string>>(File.ReadAllText(Path.Combine(Repo(), "crates", "clarity-spec", "icons", "lucide.json")))!;
        Assert.True(icons.Count > 1800);
        var outside = new List<string>();
        foreach (var (name, data) in icons)
        {
            using var path = SKPath.ParseSvgPathData(data);
            if (path is null)
            {
                outside.Add($"{name} does not parse");
                continue;
            }
            // The drawn shape's bounds, without the curves' control points, as Avalonia's Geometry.Bounds.
            var bounds = path.TightBounds;
            if (bounds.Left < -1 || bounds.Top < -1 || bounds.Right > 25 || bounds.Bottom > 25)
                outside.Add($"{name} {bounds}");
        }
        Assert.True(outside.Count == 0, string.Join("; ", outside.Take(10)));
    }

    [Fact]
    public void An_icons_line_takes_the_foreground_of_its_button()
    {
        // Three lines across the middle: a solid band where the icon is.
        const string band = "M 2 10 L 22 10 M 2 12 L 22 12 M 2 14 L 22 14";
        foreach (var dark in new[] { false, true })
        {
            var host = new TestHost();
            host.Icons["band"] = band;
            var (screen, view, _) = Framed.Window("""
                <Toolbar name="tools">
                  <Item name="save" caption="Save" icon="band" onclick="save" />
                  <Item name="mail" caption="Mail" icon="band" onclick="mail" />
                </Toolbar>
                """, host: host);
            host.State("mail")!.IsEnabled = false;
            screen.Show(ThemeLook.Clarity(dark));
            using var shot = Shot.Of(screen, 2000, 1200, 2);
            var look = screen.Look;
            var text = look.Part("button", null).Text ?? look.Color("text");
            SKColor Middle(string name)
            {
                var box = view.Frame.Toolbar!.Placed.First(p => p.Item.Text("name") == name).Box;
                // The small icon sits 10 in from the left, 16 square, in the middle of the button's height.
                return shot.PixelAt((int)((box.X + 10 + 8) * 2), (int)((box.Y + box.Height / 2) * 2));
            }
            Assert.Equal(text, Middle("save"));
            // A disabled button's icon is dimmed with it.
            Assert.NotEqual(text, Middle("mail"));
        }
    }
}
