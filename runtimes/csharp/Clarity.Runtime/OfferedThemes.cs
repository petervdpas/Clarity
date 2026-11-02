// The themes the application offers, by name and label, the one it starts
// with first: what a settings field of the theme chooses from. The screen
// that shows the themes fills it (Themes.Start).

namespace Clarity.Runtime;

public static class OfferedThemes
{
    public static IReadOnlyList<(string Name, string Label)> All { get; set; } = [];
}
