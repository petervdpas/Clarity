// The toolbar under the menu bar: a row of buttons, each with its icon and
// caption, and lines between groups of them. A click runs the button's
// handler or opens the window it targets; the `language` and `theme`
// targets open their choices below it. The toolbar hides and is greyed as
// its part state says, and so does each button.

namespace Clarity.Ui;

public sealed class FrameToolbar : FrameRegion
{
    /// <summary>How high the toolbar is.</summary>
    public const double Height = 38;

    readonly List<(CommandSpec Item, Box Box)> placed = [];
    readonly FrameButtonRow row;

    public FrameToolbar(WindowFrame frame, ToolbarSpec toolbar) : base(frame)
    {
        Toolbar = toolbar;
        row = new FrameButtonRow(this, toolbar.Items, large: false, Click);
    }

    public ToolbarSpec Toolbar { get; }

    public override double Wanted => Height;

    /// <summary>Whether it shows: its state is not hidden and one of its buttons shows; with nothing to show it takes no room.</summary>
    public override bool IsShown => FrameRules.Shows(Host, Toolbar.Name) && Toolbar.Items.Any(i => !FrameRules.IsSeparator(i) && FrameRules.Shows(Host, i.Text("name")));

    public bool Enabled => FrameRules.Takes(Host, Toolbar.Name);

    /// <summary>A disabled toolbar's buttons take no press, focus or key.</summary>
    public override bool IsEnabled => Enabled;

    /// <summary>A button says its caption and its keys when the pointer rests on it.</summary>
    public override string? TooltipAt(double x, double y) =>
        placed.FirstOrDefault(p => p.Box.Contains(x, y)) is { Item: { } item } && !FrameRules.IsSeparator(item)
            ? FrameRules.TipOf(Host.Text(item.Text("caption")), Frame.KeysOf(item, Frame.Shown.Toolbar?.Inherited == true))
            : null;

    /// <summary>Its buttons as they show, each with its place.</summary>
    public IReadOnlyList<(CommandSpec Item, Box Box)> Placed => placed;

    public override void Arrange(Box box)
    {
        Bounds = box;
        placed.Clear();
        row.Hide();
        var x = box.X + 6;
        foreach (var item in Toolbar.Items.Where(i => FrameRules.Shows(Host, i.Text("name"))))
        {
            var wide = FrameButtons.Width(this, item, large: false);
            var place = new Box(x, box.Y + 4, wide, box.Height - 8);
            placed.Add((item, place));
            row.Place(item, place);
            x += wide + 4;
        }
    }

    /// <summary>A button clicked: it runs, unless it or the toolbar is disabled.</summary>
    public void Click(CommandSpec item)
    {
        var at = placed.FindIndex(p => ReferenceEquals(p.Item, item));
        if (!Enabled || at < 0 || FrameRules.IsSeparator(item) || !FrameRules.Takes(Host, item.Text("name")))
            return;
        Frame.Run(item, placed[at].Box, Screen);
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        Plate(painter, Bounds, new Box(Bounds.X, Bounds.Bottom - look.Border, Bounds.Width, look.Border));
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        base.Draw(painter);
    }
}

/// <summary>
/// The buttons of a toolbar or a ribbon as elements of their region, one
/// per item, so each takes the pointer, the focus (Tab reaches it) and
/// Enter or Space, as Avalonia's buttons did. A region places those it
/// shows; the others are hidden.
/// </summary>
sealed class FrameButtonRow
{
    readonly Dictionary<CommandSpec, FrameButton> buttons = new(ReferenceEqualityComparer.Instance);

    public FrameButtonRow(FrameRegion region, IEnumerable<CommandSpec> items, bool large, Action<CommandSpec> click)
    {
        foreach (var item in items)
            if (!buttons.ContainsKey(item))
                buttons[item] = region.Hold(new FrameButton(region.Host, item, large, click));
    }

    /// <summary>Every button leaves its place until the region places it again.</summary>
    public void Hide()
    {
        foreach (var button in buttons.Values)
            button.Placed = false;
    }

    public void Place(CommandSpec item, Box box)
    {
        if (!buttons.TryGetValue(item, out var button))
            return;
        button.Placed = true;
        button.Arrange(box);
    }
}

/// <summary>
/// A button of a toolbar or a ribbon: a click, Enter or Space runs its
/// item; it shows the focus ring while it has the focus. A separator is a
/// line that takes no focus. One code hid shows nowhere, one it disabled is
/// greyed and passed over by Tab.
/// </summary>
public sealed class FrameButton : Element
{
    readonly IPartHost host;
    readonly bool large;
    readonly Action<CommandSpec> click;

    internal FrameButton(IPartHost host, CommandSpec item, bool large, Action<CommandSpec> click)
    {
        this.host = host;
        this.large = large;
        this.click = click;
        Item = item;
    }

    public CommandSpec Item { get; }

    /// <summary>Whether its region placed it this time.</summary>
    internal bool Placed { get; set; }

    public override bool IsShown => Placed;

    public override bool IsEnabled => FrameRules.Takes(host, Item.Text("name"));

    public override bool Focusable => !FrameRules.IsSeparator(Item);

    public override void OnClick() => click(Item);

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is not (Key.Enter or Key.Space) || key.Modifiers != Modifiers.None || FrameRules.IsSeparator(Item))
            return false;
        click(Item);
        return true;
    }

    public override void OnPointerMoved(PointerInput at) => Invalidate();

    public override void Draw(Painter painter)
    {
        FrameButtons.Draw(painter, host, Item, Bounds, large, Hovered, Pressed && Hovered);
        if (Focused)
            PressLooks.FocusRing(painter, Bounds, painter.Look.Radius("button"));
    }
}

/// <summary>How a toolbar's and a ribbon's buttons are measured and drawn: in the button look, an icon beside the caption or, large, above it.</summary>
static class FrameButtons
{
    public const double SeparatorWidth = 9, LargeHeight = 58, LargeIcon = 24, SmallIcon = 16;

    static bool IsLarge(CommandSpec item, bool large) => large && item.Flag("large");

    public static double Width(FrameRegion region, CommandSpec item, bool large)
    {
        if (FrameRules.IsSeparator(item))
            return SeparatorWidth;
        var look = region.LookNow;
        var caption = region.Host.Text(item.Text("caption"));
        var text = caption.Length == 0 ? 0 : region.FontsNow.Measure(caption, "text", look.Size("small"), false);
        var icon = item.Text("icon") is { Length: > 0 };
        if (IsLarge(item, large))
            return Math.Ceiling(Math.Max(LargeIcon, text) + 20);
        return Math.Ceiling((icon ? SmallIcon : 0) + (icon && text > 0 ? 6 : 0) + text + 20);
    }

    public static void Draw(Painter painter, IPartHost host, CommandSpec item, Box box, bool large, bool hover, bool held)
    {
        var look = painter.Look;
        if (FrameRules.IsSeparator(item))
        {
            painter.Fill(new Box(box.X + box.Width / 2, box.Y + 2, look.Border, box.Height - 4), look.Color("strong_border"));
            return;
        }
        var enabled = FrameRules.Takes(host, item.Text("name"));
        using var dimmed = PressLooks.Dimmed(painter, enabled);
        var values = look.Part("button", enabled && held ? "pressed" : enabled && hover ? "hover" : null);
        var radius = values.Radius ?? look.Radius("button");
        painter.Fill(box, values.Background ?? look.Color("panel"), radius);
        painter.Stroke(box, values.Border ?? look.Color("border"), look.Border, radius);
        var color = values.Text ?? look.Color("text");
        var caption = host.Text(item.Text("caption"));
        var size = look.Size("small");
        var path = item.Text("icon") is { Length: > 0 } icon ? host.IconPath(icon) : null;
        if (IsLarge(item, large))
        {
            if (path is not null)
                IconPart.DrawIcon(painter, path, new Box(box.X + (box.Width - LargeIcon) / 2, box.Y + 6, LargeIcon, LargeIcon), color);
            painter.Text(caption, new Box(box.X, box.Bottom - 24, box.Width, 20), color, size, align: TextAlign.Center);
            return;
        }
        var x = box.X + 10;
        if (path is not null)
        {
            IconPart.DrawIcon(painter, path, new Box(x, box.Y + (box.Height - SmallIcon) / 2, SmallIcon, SmallIcon), color);
            x += SmallIcon + 6;
        }
        painter.Text(caption, new Box(x, box.Y, box.Right - x, box.Height), color, size);
    }
}
