// The menu bar at the top of a window: its menus by their captions. A press
// on one opens its menu below it, and another press closes it; while one is
// open, the pointer over another opens that one. A top item without items
// runs as a click. The bar hides and is greyed as its part state says, and
// so does each item. Each top item has an access letter (the first of its
// caption no item before it took), underlined while Alt is held or after Alt
// went alone; Alt with it opens the item's menu. F10, or Alt alone, gives
// the bar the focus (FocusFirst) with its first top item marked: Left and
// Right move the mark round the bar, Down, Enter or Space open its menu
// with the first item marked, Escape gives the focus back. In an open menu
// Left and Right open the menu beside it.

namespace Clarity.Ui;

public sealed class FrameMenuBar(WindowFrame frame, MenuSpec menu) : FrameRegion(frame)
{
    /// <summary>How high the menu bar is.</summary>
    public const double Height = 30;

    const double Padding = 10;

    readonly List<(CommandSpec Item, Box Box)> placed = [];
    CommandSpec? marked;
    Element? before;

    public MenuSpec Menu { get; } = menu;

    public override double Wanted => Height;

    /// <summary>Whether it shows: its state is not hidden and one of its items shows; with nothing to show it takes no room.</summary>
    public override bool IsShown => FrameRules.Shows(Host, Menu.Name) && Menu.Items.Any(i => !FrameRules.IsSeparator(i) && FrameRules.Shows(Host, i.Text("name")));

    /// <summary>Whether it takes clicks: its state is not disabled.</summary>
    public bool Enabled => FrameRules.Takes(Host, Menu.Name);

    public override bool IsEnabled => Enabled;

    /// <summary>The top item marked while the bar has the focus from the keyboard; none otherwise.</summary>
    public CommandSpec? Marked => marked;

    /// <summary>It takes the focus only from the keyboard (F10, Alt), so Tab passes it by.</summary>
    public override bool Focusable => marked is not null;

    /// <summary>Its top items as they show, each with its place.</summary>
    public IReadOnlyList<(CommandSpec Item, Box Box)> Placed => placed;

    public override void Arrange(Box box)
    {
        Bounds = box;
        placed.Clear();
        var fonts = FontsNow;
        var size = LookNow.Size("text");
        var x = box.X + 6;
        foreach (var item in Menu.Items.Where(i => FrameRules.Shows(Host, i.Text("name"))))
        {
            var wide = FrameRules.IsSeparator(item) ? 9 : fonts.Measure(Host.Text(item.Text("caption")), "text", size, false) + 2 * Padding;
            placed.Add((item, new Box(x, box.Y + 2, wide, box.Height - 4)));
            x += wide + 2;
        }
    }

    (CommandSpec Item, Box Box)? ItemAt(double x, double y)
    {
        foreach (var p in placed)
            if (p.Box.Contains(x, y) && !FrameRules.IsSeparator(p.Item))
                return p;
        return null;
    }

    /// <summary>Whether its access letters show now.</summary>
    public bool AccessShown => Frame.Access != FrameAccess.Off && IsShown && Enabled;

    /// <summary>Each placed top item's access letter: its place in the caption.</summary>
    List<(CommandSpec Item, Box Box, int? At)> AccessKeys()
    {
        var items = placed.Where(p => !FrameRules.IsSeparator(p.Item)).ToList();
        var keys = FrameRules.AccessKeys([.. items.Select(p => Host.Text(p.Item.Text("caption")))]);
        return [.. items.Select((p, i) => (p.Item, p.Box, keys[i]))];
    }

    /// <summary>Opens the menu of the top item whose access letter is <paramref name="letter"/> (or runs it, without items): false when none has it.</summary>
    public bool OpenByLetter(char letter)
    {
        if (!IsShown || !Enabled)
            return false;
        foreach (var (item, box, at) in AccessKeys())
        {
            var caption = Host.Text(item.Text("caption"));
            if (at is not { } i || char.ToLowerInvariant(caption[i]) != letter)
                continue;
            if (ReferenceEquals(Frame.Menus.Owner, item))
                return true;
            Frame.Menus.CloseAll();
            Press(item, box);
            return true;
        }
        return false;
    }

    /// <summary>The top items the keyboard reaches: shown, no separator, taking clicks.</summary>
    List<(CommandSpec Item, Box Box)> Reachable() => [.. placed.Where(p => !FrameRules.IsSeparator(p.Item) && FrameRules.Takes(Host, p.Item.Text("name")))];

    /// <summary>
    /// The bar takes the focus with its first top item marked (F10, Alt
    /// alone), the focus it had kept to give back on Escape; false when it
    /// shows none or takes no clicks.
    /// </summary>
    public bool FocusFirst()
    {
        if (!IsShown || !Enabled || Screen is not { } screen || Reachable() is not [var first, ..])
            return false;
        Frame.Menus.CloseAll();
        if (screen.Focused != this)
            before = screen.Focused;
        Mark(first.Item);
        return true;
    }

    void Mark(CommandSpec item)
    {
        marked = item;
        Screen?.Focus(this);
        Invalidate();
    }

    /// <summary>The focus goes back to where it was before the bar took it.</summary>
    void Leave()
    {
        marked = null;
        var back = before is { Screen: not null, ShowsInTree: true, EnabledInTree: true } ? before : null;
        before = null;
        Screen?.Focus(back);
        Invalidate();
    }

    /// <summary>The top item <paramref name="by"/> places on from <paramref name="from"/>, round the ends.</summary>
    (CommandSpec Item, Box Box)? Beside(CommandSpec? from, int by)
    {
        var reachable = Reachable();
        if (reachable.Count == 0)
            return null;
        var at = reachable.FindIndex(p => ReferenceEquals(p.Item, from));
        return reachable[((at < 0 ? 0 : at + by) % reachable.Count + reachable.Count) % reachable.Count];
    }

    /// <summary>From an open menu of the bar, the person went <paramref name="by"/> sideways: the menu beside opens, its first item marked; a top item without one is marked on the bar.</summary>
    void OpenBeside(int by)
    {
        if (Frame.Menus.Owner is not CommandSpec open || Beside(open, by) is not { } next || Screen is not { } screen)
            return;
        if (Opens(next.Item))
            Open(next.Item, next.Box, screen, keyed: true);
        else
        {
            Frame.Menus.CloseAll();
            Mark(next.Item);
        }
    }

    public override void OnFocusChanged(bool focused)
    {
        if (!focused && marked is not null)
        {
            marked = null;
            Invalidate();
        }
    }

    public override bool OnKey(KeyInput key)
    {
        if (marked is not { } item || Screen is not { } screen)
            return false;
        // Any key ends the access letters Alt alone kept (the bar still takes it).
        if (key.Key != Key.Alt && Frame.Access == FrameAccess.Kept)
        {
            Frame.Access = FrameAccess.Off;
            Invalidate();
        }
        if (key.Modifiers == Modifiers.None && FrameRules.AccessChar(key.Key) is { } letter && OpenByLetter(letter))
            return true;
        switch (key.Key)
        {
            case Key.Left or Key.Right:
                if (Beside(item, key.Key == Key.Right ? 1 : -1) is { } next)
                    Mark(next.Item);
                return true;
            case Key.Down or Key.Enter or Key.Space:
                if (placed.FirstOrDefault(p => ReferenceEquals(p.Item, item)) is not { Item: not null } at)
                    return true;
                if (Opens(item))
                    Open(item, at.Box, screen, keyed: true);
                else
                {
                    Leave();
                    Frame.Run(item, at.Box, screen);
                }
                return true;
            case Key.Escape:
                Leave();
                return true;
        }
        return false;
    }

    bool Opens(CommandSpec item) => item.Items.Count > 0 || FrameRules.PickerOf(item) is not null;

    /// <summary>Opens the menu of a top item below it, or closes it when it is the one open.</summary>
    public void Press(CommandSpec item, Box box)
    {
        if (!Enabled || !FrameRules.Takes(Host, item.Text("name")) || Screen is not { } screen)
            return;
        if (!Opens(item))
        {
            Frame.Run(item, box, screen);
            return;
        }
        if (ReferenceEquals(Frame.Menus.Owner, item))
        {
            Frame.Menus.CloseAll();
            return;
        }
        Open(item, box, screen);
    }

    void Open(CommandSpec item, Box box, Screen screen, bool keyed = false)
    {
        var entries = item.Items.Count > 0 ? Frame.EntriesOf(item.Items) : MenuEntries.Choices(Frame.PickerOf(FrameRules.PickerOf(item)!));
        Frame.Menus.Open(screen, item, entries, box, () => Bounds);
        Frame.Menus.Beside = OpenBeside;
        if (keyed)
            Frame.Menus.At(0)?.Move(1);
        Invalidate();
    }

    public override void OnPointerPressed(PointerInput at)
    {
        if (ItemAt(at.ScreenX, at.ScreenY) is { } hit)
            Press(hit.Item, hit.Box);
    }

    public override void OnPointerMoved(PointerInput at)
    {
        base.OnPointerMoved(at);
        // While a menu of the bar is open, the pointer over another opens that one.
        if (Frame.Menus.Owner is CommandSpec open && placed.Any(p => ReferenceEquals(p.Item, open))
            && ItemAt(at.ScreenX, at.ScreenY) is { } hit && !ReferenceEquals(hit.Item, open) && Opens(hit.Item)
            && FrameRules.Takes(Host, hit.Item.Text("name")) && Screen is { } screen)
            Open(hit.Item, hit.Box, screen);
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        Plate(painter, Bounds, new Box(Bounds.X, Bounds.Bottom - look.Border, Bounds.Width, look.Border));
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        var size = look.Size("text");
        var access = AccessShown ? AccessKeys() : [];
        foreach (var (item, box) in placed)
        {
            if (FrameRules.IsSeparator(item))
            {
                painter.Fill(new Box(box.X + 4, box.Y + 4, look.Border, box.Height - 8), look.Color("strong_border"));
                continue;
            }
            var takes = FrameRules.Takes(Host, item.Text("name"));
            if (takes && (ReferenceEquals(Frame.Menus.Owner, item) || ReferenceEquals(marked, item) || Over(box)))
                painter.Fill(box, look.Part("menu", "hover").Background ?? look.Color("accent_soft"), look.Radius("button"));
            var color = takes ? look.Color("frame_text") : look.Color("muted");
            var caption = Host.Text(item.Text("caption"));
            painter.Text(caption, box, color, size, align: TextAlign.Center);
            // The access letter, underlined.
            if (access.FirstOrDefault(a => ReferenceEquals(a.Item, item)).At is { } at)
            {
                var whole = painter.Fonts.Measure(caption, "text", size, false);
                var x = box.X + (box.Width - whole) / 2 + painter.Fonts.Measure(caption[..at], "text", size, false);
                var y = Math.Round(box.Y + box.Height / 2 + size * 0.5);
                painter.Fill(new Box(x, y, painter.Fonts.Measure(caption.Substring(at, 1), "text", size, false), 1), color);
            }
        }
    }
}
