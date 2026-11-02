// The themes an application offers (DESIGN 1.5): which one is shown, kept
// for the next start as the person's choice. Clarity.UI draws the theme
// shown; the generated AppThemes names the themes offered.

namespace Clarity.Runtime;

/// <summary>A theme the application offers: its name, and its label in the person's language.</summary>
public sealed record ThemeOffer(string Name, string Label);

public static class Themes
{
    const string Key = "theme";

    static ThemeOffer[] offered = [];

    /// <summary>The themes the application offers, the one it starts with first.</summary>
    public static IReadOnlyList<ThemeOffer> Offered => offered;

    /// <summary>The name of the theme shown; null when the application offers none.</summary>
    public static string? Current { get; private set; }

    /// <summary>The person chose another theme: the screen shows it.</summary>
    public static event Action? Changed;

    /// <summary>Shows the theme kept from an earlier run while it is still offered, else the first.</summary>
    public static void Start(ThemeOffer[] themes)
    {
        offered = themes;
        OfferedThemes.All = [.. themes.Select(t => (t.Name, t.Label))];
        var kept = Remembered.Get(Key);
        Current = (themes.FirstOrDefault(t => t.Name == kept) ?? themes.FirstOrDefault())?.Name;
    }

    /// <summary>Shows the theme named <paramref name="name"/>, and keeps it for the next start.</summary>
    public static void Choose(string name)
    {
        if (offered.All(t => t.Name != name) || name == Current)
            return;
        Remembered.Set(Key, name);
        Current = name;
        Changed?.Invoke();
    }
}
