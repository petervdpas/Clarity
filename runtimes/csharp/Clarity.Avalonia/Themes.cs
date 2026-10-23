// The themes an application offers (DESIGN section 1.5, T1). The generator
// merges Clarity's own theme with each of the project's into one
// ResourceDictionary (Avalonia has no cascade), with Light and Dark theme
// dictionaries; switching a theme swaps that dictionary in the application's
// resources and sets its light or dark mode. The choice is kept for the next
// start.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Styling;

namespace Clarity.Avalonia;

/// <summary>
/// A theme the application offers: its name in the project, its label as
/// written (translated when shown), its mode (Default follows the system),
/// and its resources.
/// </summary>
public sealed record Theme(string Name, string Label, ThemeVariant Mode, Func<ResourceDictionary> Resources);

public static class Themes
{
    const string Key = "theme";

    static Theme[] offered = [];
    static ResourceDictionary? shown;

    /// <summary>The themes the application offers, the one it starts with first.</summary>
    public static IReadOnlyList<Theme> Offered => offered;

    /// <summary>The name of the theme shown; null when the application has none.</summary>
    public static string? Current { get; private set; }

    /// <summary>The person chose another theme.</summary>
    public static event Action? Changed;

    /// <summary>Shows the theme kept from an earlier run while it is still offered, else the first.</summary>
    public static void Start(Theme[] themes)
    {
        offered = themes;
        var kept = Remembered.Get(Key);
        Show(themes.FirstOrDefault(t => t.Name == kept) ?? themes.FirstOrDefault());
    }

    /// <summary>No theme of the project: for tests.</summary>
    public static void Reset()
    {
        offered = [];
        Show(null);
    }

    /// <summary>Shows the theme named <paramref name="name"/>, and keeps it for the next start.</summary>
    public static void Choose(string name)
    {
        if (offered.FirstOrDefault(t => t.Name == name) is not { } theme)
            return;
        Remembered.Set(Key, name);
        Show(theme);
        Changed?.Invoke();
    }

    static void Show(Theme? theme)
    {
        Current = theme?.Name;
        if (Application.Current is not { } app)
            return;
        if (shown is not null)
            app.Resources.MergedDictionaries.Remove(shown);
        shown = theme?.Resources();
        if (shown is not null)
            app.Resources.MergedDictionaries.Add(shown);
        app.RequestedThemeVariant = theme?.Mode ?? ThemeVariant.Default;
    }
}
