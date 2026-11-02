// The themes an application offers (DESIGN 1.5): it starts with the one kept
// from an earlier run while that is still offered, else the first; the
// person's choice is kept for the next start, and the screen hears of it.
// Kept in memory here: Remembered has no folder.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

[Collection("Remembered")]
public class ThemesTests
{
    static readonly ThemeOffer[] Offered = [new("Day", "Day"), new("Night", "Night")];

    [Fact]
    public void An_application_starts_with_its_first_theme_and_keeps_the_one_chosen()
    {
        Remembered.Folder = null;
        Themes.Start(Offered);
        Assert.Equal("Day", Themes.Current);
        Assert.Equal(["Day", "Night"], Themes.Offered.Select(t => t.Name));
        var heard = 0;
        Themes.Changed += () => heard++;
        Themes.Choose("Night");
        Assert.Equal("Night", Themes.Current);
        Assert.Equal(1, heard);
        // One not offered changes nothing.
        Themes.Choose("Sepia");
        Assert.Equal("Night", Themes.Current);
        Assert.Equal(1, heard);
        // The next start shows the one kept.
        Themes.Start(Offered);
        Assert.Equal("Night", Themes.Current);
        // A kept theme no longer offered gives way to the first.
        Themes.Start([new("Day", "Day")]);
        Assert.Equal("Day", Themes.Current);
    }

    [Fact]
    public void The_choices_of_a_theme_setting_are_the_offered_themes_by_their_labels()
    {
        Themes.Start([new("Day", "Daylight")]);
        Assert.Equal([("Day", "Daylight")], Choice.ThemesOffered.Select(c => ((string)c.Value!, c.Label)));
    }
}
