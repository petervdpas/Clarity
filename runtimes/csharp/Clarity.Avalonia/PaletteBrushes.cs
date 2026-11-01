// A color of the palette (Clarity.Ui's Palette) as an Avalonia brush.

namespace Clarity.Avalonia;

public static class PaletteBrushes
{
    /// <summary>A kept color as a brush; null for no color.</summary>
    public static global::Avalonia.Media.IBrush? Brush(string? color) =>
        Palette.Read(color) is { Length: 7 } kept && global::Avalonia.Media.Color.TryParse(kept, out var c) ? new global::Avalonia.Media.SolidColorBrush(c) : null;
}
