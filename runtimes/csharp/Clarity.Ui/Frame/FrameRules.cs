// The rules of a frame every screen keeps alike: the tab a ribbon shows
// (conformance/ribbons.json), the width a sidebar is dragged to and whether
// it is folded or fills the window (conformance/sidebars.json), and what of
// a frame's commands shows, takes clicks and answers keys, as their part
// state says (conformance/overlays.json `item`, DESIGN 5.28).

using Clarity.Runtime;

namespace Clarity.Ui;

public static class FrameRules
{
    /// <summary>The widths a sidebar may have (clarity_spec::model::SIDEBAR_WIDTHS).</summary>
    public const double Narrowest = 100, Widest = 1600;

    /// <summary>The width of a folded sidebar: room for its button.</summary>
    public const double StripWidth = 28;

    /// <summary>
    /// The tab a ribbon shows (from 1) of tabs that are visible or not, when
    /// <paramref name="asked"/> is wanted: it, when it is there and visible,
    /// else the first visible one; none when no tab is visible.
    /// </summary>
    public static int? ShownTab(IReadOnlyList<bool> visible, long asked)
    {
        if (asked >= 1 && asked <= visible.Count && visible[(int)asked - 1])
            return (int)asked;
        for (var i = 0; i < visible.Count; i++)
            if (visible[i])
                return i + 1;
        return null;
    }

    /// <summary>The width a drag to <paramref name="dragged"/> pixels gives: within the limits, in whole pixels.</summary>
    public static double SidebarWidth(double dragged, double? min, double? max) =>
        Math.Round(Math.Min(max ?? Widest, Math.Max(min ?? Narrowest, dragged)), MidpointRounding.AwayFromZero);

    /// <summary>
    /// Whether a sidebar is folded and fills the window now: as its part
    /// state says (a handler or the person set it), else as designed; only
    /// what it allows.
    /// </summary>
    public static (bool Collapsed, bool Maximized) SidebarNow(SidebarSpec sidebar, bool? collapsed, bool? maximized) =>
        (sidebar.Collapsible && (collapsed ?? sidebar.Collapsed), sidebar.Maximizable && (maximized ?? false));

    /// <summary>
    /// The access letter of each top item of a menu bar, by its caption as
    /// shown: the place of its first letter or digit that no item before it
    /// took, case aside; none for an item without one free. Alt with that
    /// letter opens the item's menu, as on desktop programs.
    /// </summary>
    public static int?[] AccessKeys(IReadOnlyList<string> captions)
    {
        var taken = new List<char>();
        var keys = new int?[captions.Count];
        for (var i = 0; i < captions.Count; i++)
        {
            var caption = captions[i];
            for (var at = 0; at < caption.Length; at++)
            {
                var c = char.ToLowerInvariant(caption[at]);
                if (!char.IsAsciiLetterOrDigit(c) || taken.Contains(c))
                    continue;
                taken.Add(c);
                keys[i] = at;
                break;
            }
        }
        return keys;
    }

    /// <summary>The letter or digit a key writes, for an access letter; none for any other key.</summary>
    public static char? AccessChar(Key key) => key switch
    {
        >= Key.A and <= Key.Z => (char)('a' + (key - Key.A)),
        >= Key.D0 and <= Key.D9 => (char)('0' + (key - Key.D0)),
        _ => null,
    };

    /// <summary>Whether a menu item is a line between items.</summary>
    public static bool IsSeparator(CommandSpec item) => item.Text("kind") == "separator";

    /// <summary>Whether a named region or command shows: while its state is not hidden.</summary>
    public static bool Shows(IPartHost host, string? name) => name is null || host.State(name)?.IsVisible != false;

    /// <summary>Whether a named region or command takes clicks: while its state is not disabled.</summary>
    public static bool Takes(IPartHost host, string? name) => name is null || host.State(name)?.IsEnabled != false;

    /// <summary>Whether a named menu item shows a check mark: its state is checked.</summary>
    public static bool Checked(IPartHost host, string? name) => name is not null && host.State(name)?.IsChecked == true;

    /// <summary>The built-in click target a command picks in place, <c>language</c> or <c>theme</c>; none for any other.</summary>
    public static string? PickerOf(CommandSpec item) => item.Text("onClick") is "language" or "theme" && item.Text("handler") is not { Length: > 0 } ? item.Text("onClick") : null;

    /// <summary>
    /// The keys a command answers: the person's where they set their own (a
    /// command of the app's frame by its name, a window's own by
    /// <c>Window.name</c> of <paramref name="window"/>), else its designed one.
    /// </summary>
    public static IReadOnlyList<string> KeysOf(IPartHost host, string? window, string? name, bool app, string? designed)
    {
        designed = designed is { Length: > 0 } ? designed : null;
        var id = name is null ? null : app ? name : window is not null ? $"{window}.{name}" : null;
        return id is null ? designed is null ? [] : [designed] : host.KeysOf(id, designed);
    }

    /// <summary>What a toolbar's or ribbon's button says when the pointer rests on it: its caption and its keys.</summary>
    public static string? TipOf(string caption, IReadOnlyList<string> keys) =>
        (caption.Length, keys.Count) switch
        {
            (0, 0) => null,
            (_, 0) => caption,
            (0, _) => string.Join(", ", keys),
            _ => $"{caption} ({string.Join(", ", keys)})",
        };

    /// <summary>
    /// The frame's commands a key may run, in the order they are written: the
    /// menu, the toolbar, the ribbon (whichever tab is open), while they and
    /// what holds them show and take clicks; each with a key of its design or
    /// a name the person's keys know it by, and whether it is of the app's
    /// frame (<paramref name="frame"/>'s region is inherited).
    /// </summary>
    public static IEnumerable<(CommandSpec Item, bool App)> Keyed(ShownFrame frame, IPartHost host)
    {
        IEnumerable<(CommandSpec, bool)> Walk(IEnumerable<CommandSpec> items, bool app)
        {
            foreach (var item in items)
            {
                var name = item.Text("name");
                if (IsSeparator(item) || !Shows(host, name) || !Takes(host, name))
                    continue;
                if (item.Text("shortcut") is { Length: > 0 } || name is not null && item.Items.Count == 0)
                    yield return (item, app);
                foreach (var inner in Walk(item.Items, app))
                    yield return inner;
            }
        }
        bool Usable(string? name) => Shows(host, name) && Takes(host, name);
        if (frame.Menu is { } menu && Usable(menu.Value.Name))
            foreach (var item in Walk(menu.Value.Items, menu.Inherited))
                yield return item;
        if (frame.Toolbar is { } toolbar && Usable(toolbar.Value.Name))
            foreach (var item in Walk(toolbar.Value.Items, toolbar.Inherited))
                yield return item;
        if (frame.Ribbon is { } ribbon && Usable(ribbon.Value.Name))
            foreach (var tab in ribbon.Value.Tabs.Where(t => Shows(host, t.Name)))
                foreach (var group in tab.Groups.Where(g => Shows(host, g.Name)))
                    foreach (var item in Walk(group.Items, ribbon.Inherited))
                        yield return item;
    }
}

/// <summary>
/// The choices of a built-in picker, <c>language</c> or <c>theme</c>: each
/// with its value, its label and whether it is the one shown, and what
/// choosing one does. Set in place, no window opens for them.
/// </summary>
public sealed record FramePicker(Func<IReadOnlyList<(string Value, string Label, bool Shown)>> Choices, Action<string> Choose)
{
    /// <summary>The languages the application offers, each by its own name, through Locale.</summary>
    public static readonly FramePicker Language = new(
        () => [.. Locale.Offered.Select(l => (l, Languages.Name(l), l == Locale.Current))],
        Locale.Choose);

    /// <summary>The themes the application offers, each labelled as the settings dialog's dropdown offers it (Choice.ThemesOffered), through Themes.</summary>
    public static readonly FramePicker Theme = new(
        () => [.. Choice.ThemesOffered.Select(c => ((string)c.Value!, c.Label, (string)c.Value! == Themes.Current))],
        Themes.Choose);
}
