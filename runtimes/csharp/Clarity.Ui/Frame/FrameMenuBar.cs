// The menu bar at the top of a window: its menus by their captions. A press
// on one opens its menu below it, and another press closes it; while one is
// open, the pointer over another opens that one. A top item without items
// runs as a click. The bar hides and is greyed as its part state says, and
// so does each item. Each top item has an access letter (the first of its
// caption no item before it took), underlined while Alt is held or after Alt
// went alone; Alt with it opens the item's menu.

namespace Clarity.Ui;

public sealed class FrameMenuBar(WindowFrame frame, MenuSpec menu) : FrameRegion(frame)
{
    /// <summary>How high the menu bar is.</summary>
    public const double Height = 30;

    const double Padding = 10;

    readonly List<(CommandSpec Item, Box Box)> placed = [];

    public MenuSpec Menu { get; } = menu;

    public override double Wanted => Height;

    /// <summary>Whether it shows: its state is not hidden and one of its items shows; with nothing to show it takes no room.</summary>
    public override bool IsShown => FrameRules.Shows(Host, Menu.Name) && Menu.Items.Any(i => !FrameRules.IsSeparator(i) && FrameRules.Shows(Host, i.Text("name")));

    /// <summary>Whether it takes clicks: its state is not disabled.</summary>
    public bool Enabled => FrameRules.Takes(Host, Menu.Name);

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

    void Open(CommandSpec item, Box box, Screen screen)
    {
        var entries = item.Items.Count > 0 ? Frame.EntriesOf(item.Items) : MenuEntries.Choices(Frame.PickerOf(FrameRules.PickerOf(item)!));
        Frame.Menus.Open(screen, item, entries, box, () => Bounds);
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
            if (takes && (ReferenceEquals(Frame.Menus.Owner, item) || Over(box)))
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
