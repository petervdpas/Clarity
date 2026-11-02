// Parts made from markup as Made.Window makes them, but through
// PartVariants.Make, so a dropdown one types in, a search dropdown and a
// picture field are their own variant parts.

namespace Clarity.Ui.Tests;

public static class VariantMade
{
    /// <summary>The window's parts of <paramref name="markup"/> in a grid of 12, each the variant that fits, on a screen <paramref name="width"/> by <paramref name="height"/>.</summary>
    public static (Screen Screen, GridPanel Grid, List<Part> Parts) Window(string markup, IPartHost host, double width = 800, double height = 400)
    {
        var spec = WindowMarkup.Read($"<Window name=\"Test\">{markup}</Window>");
        var grid = new GridPanel(columns: 12);
        var parts = new List<Part>();
        foreach (var p in spec.Parts)
        {
            var part = PartVariants.Make(p, host);
            parts.Add(part);
            grid.Add(part, p.Place(part.Grows));
        }
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(width, height, 1);
        return (screen, grid, parts);
    }
}
