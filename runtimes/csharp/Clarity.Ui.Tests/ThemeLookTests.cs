// A theme as Clarity.UI reads it: Clarity's own, as Rust resolves it, and a
// project's theme on top of it (conformance/themes.json).

using System.Text.Json;

namespace Clarity.Ui.Tests;

public class ThemeLookTests
{
    [Fact]
    public void Clarity_s_own_theme_is_carried_for_light_and_dark()
    {
        var light = ThemeLook.Clarity(dark: false);
        var dark = ThemeLook.Clarity(dark: true);
        Assert.False(light.Dark);
        Assert.True(dark.Dark);
        Assert.Equal(SKColor.Parse("#1f5fbf"), light.Color("accent"));
        Assert.Equal(SKColor.Parse("#6fa3f2"), dark.Color("accent"));
        Assert.Equal(14, light.Size("text"));
        Assert.Equal(6, light.Radius("button"));
        Assert.Equal(1, light.Border);
        Assert.Equal(4, light.Padding);
    }

    [Fact]
    public void A_part_s_look_in_a_state_falls_back_to_the_part_s_own()
    {
        var look = ThemeLook.Clarity(dark: false);
        var hover = look.Part("button", "hover");
        Assert.Equal(SKColor.Parse("#1f5fbf"), hover.Border);
        // What the hover leaves out is the button's.
        Assert.Equal(SKColor.Parse("#f6f7f9"), hover.Background);
        Assert.Equal(SKColor.Parse("#1d2330"), hover.Text);
        Assert.Null(look.Part("no_such_part", null).Background);
    }

    [Fact]
    public void A_project_s_theme_changes_only_what_it_sets()
    {
        var cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "themes.json"))).RootElement.GetProperty("resolves");
        var first = cases[0];
        var look = ThemeLook.Over(first.GetProperty("gives").GetRawText(), ThemeLook.Clarity(first.GetProperty("dark").GetBoolean()));
        Assert.Equal(SKColor.Parse("#2f6fdf"), look.Color("accent"));
        Assert.Equal(SKColor.Parse("#f5c400"), look.Color("brand"));
        // What it leaves out is Clarity's own.
        Assert.Equal(SKColor.Parse("#eef0f3"), look.Color("background"));
        Assert.Equal(SKColor.Parse("#1d2330"), look.Style("loud").Text);
        Assert.Equal(SKColor.Parse("#f5c400"), look.Style("loud").Background);
        Assert.Equal("Inter", look.Font("text"));
        Assert.Equal(6, look.Radius("button"));
    }
}
