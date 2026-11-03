// A menu opened over the window (DESIGN 5.16): a menu bar's menu, the
// choices of a toolbar button that picks the language or theme, and the
// flyout of a sub menu. Each item shows its check mark, icon, caption and
// key, an arrow for a sub menu; a line between items for a separator. An
// item code hid is left out, one it disabled is greyed and takes no click,
// as every part. Picking an item closes every menu first, then runs it.

using SkiaSharp;

namespace Clarity.Ui;

/// <summary>An item of a menu as it shows: its caption, its icon's SVG path, its key, check mark, whether it takes clicks, its sub menu and what picking it does.</summary>
public sealed record MenuEntry(string Caption, string? Icon, string? Key, bool Checked, bool Enabled, bool Separator, IReadOnlyList<MenuEntry> Items, Action? Run)
{
    public static readonly MenuEntry Line = new("", null, null, false, false, true, [], null);
}

public static class MenuEntries
{
    /// <summary>
    /// The items of a menu as they show over <paramref name="host"/>: hidden
    /// ones left out; the <c>language</c> and <c>theme</c> targets with their
    /// choices as a sub menu, the one shown checked; any other runs through
    /// <paramref name="run"/>. Each shows the key <paramref name="keyOf"/>
    /// gives it (the person's), else its designed one.
    /// </summary>
    public static IReadOnlyList<MenuEntry> Of(IPartHost host, IEnumerable<CommandSpec> items, Action<CommandSpec> run, Func<string, FramePicker> pickers, Func<CommandSpec, string?>? keyOf = null)
    {
        var entries = new List<MenuEntry>();
        foreach (var item in items)
        {
            var name = item.Text("name");
            if (!FrameRules.Shows(host, name))
                continue;
            if (FrameRules.IsSeparator(item))
            {
                entries.Add(MenuEntry.Line);
                continue;
            }
            var caption = host.Text(item.Text("caption"));
            var enabled = FrameRules.Takes(host, name);
            var sub = item.Items.Count > 0
                ? Of(host, item.Items, run, pickers, keyOf)
                : FrameRules.PickerOf(item) is { } picker ? Choices(pickers(picker)) : [];
            var icon = item.Text("icon") is { Length: > 0 } named ? host.IconPath(named) : null;
            var key = keyOf is null ? item.Text("shortcut") : keyOf(item);
            entries.Add(new MenuEntry(caption, icon, key, FrameRules.Checked(host, name), enabled, false, sub, sub.Count > 0 ? null : () => run(item)));
        }
        return entries;
    }

    /// <summary>A picker's choices as items, the one shown checked.</summary>
    public static IReadOnlyList<MenuEntry> Choices(FramePicker picker) =>
        [.. picker.Choices().Select(c => new MenuEntry(c.Label, null, null, c.Shown, true, false, [], () => picker.Choose(c.Value)))];
}

/// <summary>
/// The menus open over a window: one menu and the flyouts of its sub menus,
/// one per level. A press outside or Escape closes them all; picking an
/// item closes them and then runs it.
/// </summary>
public sealed class FrameMenus
{
    readonly List<(Overlay Overlay, FrameMenuList List, MenuEntry? From)> open = [];
    bool closing;

    /// <summary>What Left and Right do in the first menu (a menu bar opens the menu beside, -1 left, 1 right); none for a menu on its own.</summary>
    public Action<int>? Beside { get; set; }

    /// <summary>What the open menu belongs to (a menu bar's item, a toolbar button); none while no menu is open.</summary>
    public object? Owner { get; private set; }

    public bool IsOpen => open.Count > 0;

    /// <summary>The menus open now, the first menu first.</summary>
    public IReadOnlyList<FrameMenuList> Lists => [.. open.Select(o => o.List)];

    /// <summary>Opens <paramref name="entries"/> below <paramref name="under"/> for <paramref name="owner"/>; a press on <paramref name="anchor"/> goes to what opened it.</summary>
    public void Open(Screen screen, object owner, IReadOnlyList<MenuEntry> entries, Box under, Func<Box>? anchor = null)
    {
        CloseAll();
        var list = new FrameMenuList(this, entries, 0);
        var (width, height) = FrameMenuList.SizeOf(entries, screen.Fonts, screen.Look.Size("text"));
        var x = Math.Max(0, Math.Min(under.X, screen.Width - width));
        var y = under.Bottom + 1;
        if (y + height > screen.Height && under.Y - 1 - height >= 0)
            y = under.Y - 1 - height;
        Owner = owner;
        Add(screen, list, new Box(x, y, width, height), anchor, null);
        screen.Focus(list);
    }

    /// <summary>Opens the sub menu of <paramref name="entry"/>, which is at <paramref name="row"/> of the menu at <paramref name="level"/>, beside it.</summary>
    public void OpenSub(int level, MenuEntry entry, Box row)
    {
        if (level >= open.Count || open[level].List.Screen is not { } screen)
            return;
        if (open.Count > level + 1 && ReferenceEquals(open[level + 1].From, entry))
            return;
        CloseFrom(level + 1);
        var (width, height) = FrameMenuList.SizeOf(entry.Items, screen.Fonts, screen.Look.Size("text"));
        var parent = open[level].List.Bounds;
        var x = parent.Right - 2;
        if (x + width > screen.Width)
            x = Math.Max(0, parent.X - width + 2);
        var y = Math.Max(0, Math.Min(row.Y - FrameMenuList.Inner, screen.Height - height));
        var list = new FrameMenuList(this, entry.Items, level + 1);
        Add(screen, list, new Box(x, y, width, height), () => open.Count > level ? open[level].List.Bounds : default, entry);
    }

    void Add(Screen screen, FrameMenuList list, Box place, Func<Box>? anchor, MenuEntry? from)
    {
        var overlay = new Overlay(list, place)
        {
            Anchor = anchor,
            // Closed from outside (a press elsewhere, Escape): every menu goes.
            Closed = () =>
            {
                if (!closing)
                    CloseAll();
            },
        };
        open.Add((overlay, list, from));
        screen.Open(overlay);
    }

    /// <summary>Closes the menus from <paramref name="level"/> on.</summary>
    public void CloseFrom(int level)
    {
        closing = true;
        try
        {
            while (open.Count > Math.Max(0, level))
            {
                var (overlay, list, _) = open[^1];
                open.RemoveAt(open.Count - 1);
                list.Screen?.Close(overlay);
            }
        }
        finally
        {
            closing = false;
        }
        if (open.Count == 0)
        {
            Owner = null;
            Beside = null;
        }
    }

    public void CloseAll() => CloseFrom(0);

    /// <summary>The person picked <paramref name="entry"/>: the menus close, then it runs.</summary>
    public void Picked(MenuEntry entry)
    {
        CloseAll();
        entry.Run?.Invoke();
    }

    /// <summary>The menu at <paramref name="level"/>, to give the focus back to.</summary>
    internal FrameMenuList? At(int level) => level >= 0 && level < open.Count ? open[level].List : null;
}

/// <summary>One open menu: its items in lines, the one under the pointer or reached with the arrow keys marked.</summary>
public sealed class FrameMenuList(FrameMenus menus, IReadOnlyList<MenuEntry> entries, int level) : Element
{
    /// <summary>How high an item's line is, a separator's, and the space inside the menu's border.</summary>
    public const double Line = 28, SeparatorLine = 9, Inner = 4;

    const double CheckWidth = 26, KeyGap = 28, ArrowWidth = 20, MinWidth = 180;

    int marked = -1;
    int pressedAt = -1;

    public IReadOnlyList<MenuEntry> Entries => entries;

    public int Level => level;

    /// <summary>The item marked now; -1 for none.</summary>
    public int Marked => marked;

    public override bool Focusable => true;

    /// <summary>How wide and high a menu of <paramref name="entries"/> is.</summary>
    public static (double Width, double Height) SizeOf(IReadOnlyList<MenuEntry> entries, Fonts fonts, double size)
    {
        double captions = 0, keys = 0;
        var arrow = false;
        foreach (var entry in entries.Where(e => !e.Separator))
        {
            captions = Math.Max(captions, fonts.Measure(entry.Caption, "text", size, false) + (entry.Icon is null ? 0 : 24));
            if (entry.Key is { } key)
                keys = Math.Max(keys, fonts.Measure(key, "text", size - 2, false));
            arrow |= entry.Items.Count > 0;
        }
        var width = CheckWidth + captions + (keys > 0 ? KeyGap + keys : 0) + (arrow ? ArrowWidth : 0) + 16 + 2 * Inner;
        var height = entries.Sum(e => e.Separator ? SeparatorLine : Line) + 2 * Inner;
        return (Math.Ceiling(Math.Max(MinWidth, width)), Math.Max(Line, height));
    }

    /// <summary>Where the item at <paramref name="at"/> is.</summary>
    public Box RowOf(int at)
    {
        var y = Bounds.Y + Inner;
        for (var i = 0; i < at; i++)
            y += entries[i].Separator ? SeparatorLine : Line;
        return new Box(Bounds.X + Inner, y, Bounds.Width - 2 * Inner, entries[at].Separator ? SeparatorLine : Line);
    }

    int RowAt(double screenY)
    {
        for (var i = 0; i < entries.Count; i++)
        {
            var row = RowOf(i);
            if (screenY >= row.Y && screenY < row.Bottom)
                return entries[i].Separator ? -1 : i;
        }
        return -1;
    }

    public override void OnPointerMoved(PointerInput at)
    {
        var row = RowAt(at.ScreenY);
        if (row < 0 || row == marked)
            return;
        marked = row;
        // Over an item with a sub menu, its flyout opens; over another, an open flyout goes.
        if (entries[row] is { Enabled: true, Items.Count: > 0 } entry)
            menus.OpenSub(level, entry, RowOf(row));
        else
            menus.CloseFrom(level + 1);
        Invalidate();
    }

    public override void OnPointerPressed(PointerInput at) => pressedAt = RowAt(at.ScreenY);

    public override void OnClick()
    {
        if (pressedAt >= 0)
            Activate(pressedAt);
    }

    /// <summary>The item at <paramref name="at"/> picked: a sub menu opens, an item that takes clicks runs.</summary>
    public void Activate(int at)
    {
        if (at < 0 || at >= entries.Count || entries[at] is not { Separator: false, Enabled: true } entry)
            return;
        marked = at;
        if (entry.Items.Count > 0)
        {
            menus.OpenSub(level, entry, RowOf(at));
            if (menus.At(level + 1) is { } sub && Screen is { } screen)
            {
                screen.Focus(sub);
                sub.Move(1);
            }
            return;
        }
        menus.Picked(entry);
    }

    /// <summary>Marks the next item that is no separator, <paramref name="by"/> one down or up.</summary>
    public void Move(int by)
    {
        if (entries.All(e => e.Separator))
            return;
        var at = marked;
        do
            at = ((at < 0 && by < 0 ? entries.Count : at) + by + entries.Count) % entries.Count;
        while (entries[at].Separator);
        marked = at;
        Invalidate();
    }

    public override bool OnKey(KeyInput key)
    {
        switch (key.Key)
        {
            case Key.Down:
                Move(1);
                return true;
            case Key.Up:
                Move(-1);
                return true;
            case Key.Enter or Key.Space:
                Activate(marked);
                return true;
            case Key.Right when marked >= 0 && entries[marked].Items.Count > 0:
                Activate(marked);
                return true;
            case Key.Left when level > 0:
                // The screen, kept before this menu closes and leaves it.
                var screen = Screen;
                menus.CloseFrom(level);
                screen?.Focus(menus.At(level - 1));
                return true;
            // In a menu bar's menu, Left and Right open the menu beside it on the bar.
            case Key.Left when menus.Beside is { } beside:
                beside(-1);
                return true;
            case Key.Right when menus.Beside is { } beside:
                beside(1);
                return true;
        }
        return false;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var menu = look.Part("menu", null);
        var radius = menu.Radius ?? look.Radius("panel");
        painter.Fill(Bounds, menu.Background ?? look.Color("raised"), radius);
        painter.Stroke(Bounds, menu.Border ?? look.Color("border"), look.Border, radius);
        var size = menu.Size ?? look.Size("text");
        for (var i = 0; i < entries.Count; i++)
        {
            var entry = entries[i];
            var row = RowOf(i);
            if (entry.Separator)
            {
                painter.Fill(new Box(row.X + 6, row.Y + row.Height / 2, row.Width - 12, look.Border), look.Color("strong_border"));
                continue;
            }
            var lit = i == marked && entry.Enabled;
            // A disabled entry is greyed as a disabled button is: muted and dimmed.
            using var dimmed = PressLooks.Dimmed(painter, entry.Enabled);
            var values = lit ? look.Part("menu", "hover") : menu;
            if (lit)
                painter.Fill(row, values.Background ?? look.Color("accent_soft"), look.Radius("button"));
            var text = entry.Enabled ? values.Text ?? look.Color("text") : look.Color("muted");
            if (entry.Checked)
                Check(painter, new Box(row.X, row.Y, CheckWidth, row.Height), text);
            var x = row.X + CheckWidth;
            if (entry.Icon is { } path)
            {
                IconPart.DrawIcon(painter, path, new Box(x, row.Y + (row.Height - 16) / 2, 16, 16), text);
                x += 24;
            }
            painter.Text(entry.Caption, new Box(x, row.Y, row.Right - x, row.Height), text, size);
            var right = row.Right - 8;
            if (entry.Items.Count > 0)
            {
                Arrow(painter, new Box(right - 12, row.Y, 12, row.Height), text);
                right -= ArrowWidth;
            }
            if (entry.Key is { } key)
                painter.Text(key, new Box(row.X, row.Y, right - row.X, row.Height), look.Color("muted"), size - 2, "text", align: TextAlign.Right);
        }
    }

    /// <summary>A check mark in the middle of <paramref name="box"/>.</summary>
    static void Check(Painter painter, Box box, SKColor color)
    {
        var cx = (float)(box.X + box.Width / 2);
        var cy = (float)(box.Y + box.Height / 2);
        using var paint = new SKPaint { Color = color, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.8f, StrokeCap = SKStrokeCap.Round, StrokeJoin = SKStrokeJoin.Round };
        painter.Canvas.DrawLine(cx - 5, cy, cx - 1.5f, cy + 3.5f, paint);
        painter.Canvas.DrawLine(cx - 1.5f, cy + 3.5f, cx + 5, cy - 4, paint);
    }

    /// <summary>A small arrow pointing right: the item opens a sub menu.</summary>
    static void Arrow(Painter painter, Box box, SKColor color)
    {
        var cx = (float)(box.X + box.Width / 2);
        var cy = (float)(box.Y + box.Height / 2);
        using var paint = new SKPaint { Color = color, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.5f, StrokeCap = SKStrokeCap.Round };
        painter.Canvas.DrawLine(cx - 2, cy - 4, cx + 2, cy, paint);
        painter.Canvas.DrawLine(cx + 2, cy, cx - 2, cy + 4, paint);
    }
}
