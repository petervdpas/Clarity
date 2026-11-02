// A button stays readable in every state: its text against its plate, at
// rest, hovered and pressed, for Clarity's own theme in light and dark, with
// no style and with a primary and a danger style, and for a theme whose hover
// sets a soft background (the shop's Day theme) under a primary style. A
// style that fills the button keeps its colors in a state; the state shades
// its fill away from its text. The same cases as the Rust version's
// tests/looks_contrast.rs.

namespace Clarity.Ui.Tests;

public class LooksContrastTests
{
    /// <summary>The least contrast a button's text keeps against its plate (WCAG 1.4.3).</summary>
    const double Least = 4.5;

    static ThemeLook Styled(bool dark) => ThemeLook.Over($$"""
        { "dark": {{(dark ? "true" : "false")}},
          "styles": {
            "primary": { "background": "@accent", "text": "@on_accent", "border": "@accent", "weight": "bold" },
            "danger": { "background": "@error", "text": "@on_accent", "border": "@error" } } }
        """, ThemeLook.Clarity(dark));

    static ThemeLook Day() => ThemeLook.Over("""
        { "dark": false,
          "colors": { "background": "#f4f6f9", "panel": "#ffffff", "accent": "#1f5fbf", "accent_soft": "#e4edfb" },
          "parts": {
            "button": { "background": "#ffffff", "text": "#1f5fbf", "border": "#1f5fbf" },
            "button:hover": { "background": "#e4edfb" } },
          "styles": { "primary": { "background": "#1f5fbf", "text": "#ffffff", "border": "#1f5fbf", "weight": "bold" } } }
        """, ThemeLook.Clarity(false));

    static void Check(string name, ThemeLook look, string? style)
    {
        foreach (var state in new string?[] { null, "hover", "pressed" })
        {
            var values = look.Pressable("button", style, state);
            var fill = values.Background ?? look.Color("raised");
            var text = values.Text ?? look.Color("text");
            var ratio = ThemeLook.Contrast(text, fill);
            Assert.True(ratio >= Least, $"{name}, style {style}, state {state}: {ratio:F2} ({text} on {fill})");
        }
    }

    [Fact]
    public void Every_built_in_theme_keeps_its_buttons_readable_in_every_state()
    {
        foreach (var dark in new[] { false, true })
        {
            var name = dark ? "Clarity dark" : "Clarity light";
            Check(name, ThemeLook.Clarity(dark), null);
            var look = Styled(dark);
            foreach (var style in new string?[] { null, "primary", "danger" })
                Check(name, look, style);
        }
    }

    [Fact]
    public void A_hover_that_fills_the_button_keeps_a_primary_button_readable()
    {
        var look = Day();
        Check("Day", look, "primary");
        Check("Day", look, null);
        // A plain button still takes the hover's soft fill.
        Assert.Equal(SKColor.Parse("#e4edfb"), look.Pressable("button", null, "hover").Background);
    }

    [Fact]
    public void A_styled_button_still_shows_it_is_hovered_and_pressed()
    {
        var look = Day();
        var rest = look.Pressable("button", "primary", null).Background;
        var hover = look.Pressable("button", "primary", "hover").Background;
        var pressed = look.Pressable("button", "primary", "pressed").Background;
        Assert.Equal(SKColor.Parse("#1f5fbf"), rest);
        Assert.NotEqual(rest, hover);
        Assert.NotEqual(hover, pressed);
        Assert.Equal(SKColor.Parse("#ffffff"), look.Pressable("button", "primary", "hover").Text);
    }

    [Fact]
    public void Contrast_is_measured_as_wcag_says()
    {
        Assert.Equal(21.0, ThemeLook.Contrast(SKColors.Black, SKColors.White), 2);
        Assert.Equal(1.0, ThemeLook.Contrast(SKColors.White, SKColors.White), 2);
    }
}
