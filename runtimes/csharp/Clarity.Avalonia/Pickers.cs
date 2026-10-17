// The built-in click targets `language` and `theme`: the choices as a
// submenu of a menu item, or as a flyout under a button, each with a check
// mark at the one shown. No desktop window opens for them.

using Avalonia.Controls;
using Avalonia.Threading;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Avalonia;

public static class Pickers
{
    /// <summary>Lets <paramref name="target"/> pick the language, each by its own name.</summary>
    public static void Language(Control target) =>
        Attach(target, "language", Locale.Offered.Select(l => (l, Languages.Name(l), l == Locale.Current)), Locale.Choose);

    /// <summary>Lets <paramref name="target"/> pick the theme, each by its translated label.</summary>
    public static void Theme(Control target) =>
        Attach(target, "theme", Themes.Offered.Select(t => (t.Name, Locale.Translate(t.Label), t.Name == Themes.Current)), Themes.Choose);

    static void Attach(Control target, string group, IEnumerable<(string Value, string Label, bool Shown)> choices, Action<string> choose)
    {
        var items = choices
            .Select(c => new MenuItem
            {
                Header = c.Label,
                ToggleType = MenuItemToggleType.Radio,
                GroupName = group,
                IsChecked = c.Shown,
                // The views are rebuilt for a language, so the menu closes first.
                Command = new RelayCommand(() =>
                {
                    if (Dispatcher.UIThread.CheckAccess() && target.IsLoaded)
                        Dispatcher.UIThread.Post(() => choose(c.Value));
                    else
                        choose(c.Value);
                }),
            })
            .ToList();
        switch (target)
        {
            case MenuItem menu:
                menu.ItemsSource = items;
                break;
            case Button button:
                button.Flyout = new MenuFlyout { ItemsSource = items };
                break;
            default:
                throw new ArgumentException($"a {target.GetType().Name} cannot pick the {group}");
        }
    }
}
