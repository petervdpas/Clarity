// The toolbar under the menu bar: a row of buttons, each with its icon and
// caption, and lines between groups of them. A click runs the button's
// handler or opens the window it targets; the `language` and `theme`
// targets open their choices below it. The toolbar hides and is greyed as
// its part state says, and so does each button.

namespace Clarity.Ui;

public sealed class FrameToolbar(WindowFrame frame, ToolbarSpec toolbar) : FrameRegion(frame)
{
    /// <summary>How high the toolbar is.</summary>
    public const double Height = 38;

    readonly List<(CommandSpec Item, Box Box)> placed = [];
    int pressedAt = -1;

    public ToolbarSpec Toolbar { get; } = toolbar;

    public override double Wanted => Height;

    /// <summary>Whether it shows: its state is not hidden and one of its buttons shows; with nothing to show it takes no room.</summary>
    public override bool IsShown => FrameRules.Shows(Host, Toolbar.Name) && Toolbar.Items.Any(i => !FrameRules.IsSeparator(i) && FrameRules.Shows(Host, i.Text("name")));

    public bool Enabled => FrameRules.Takes(Host, Toolbar.Name);

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
        var x = box.X + 6;
        foreach (var item in Toolbar.Items.Where(i => FrameRules.Shows(Host, i.Text("name"))))
        {
            var wide = FrameButtons.Width(this, item, large: false);
            placed.Add((item, new Box(x, box.Y + 4, wide, box.Height - 8)));
            x += wide + 4;
        }
    }

    public override void OnPointerPressed(PointerInput at) => pressedAt = placed.FindIndex(p => p.Box.Contains(at.ScreenX, at.ScreenY));

    public override void OnClick()
    {
        if (pressedAt >= 0 && pressedAt < placed.Count)
            Click(placed[pressedAt].Item);
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
        for (var i = 0; i < placed.Count; i++)
        {
            var (item, box) = placed[i];
            FrameButtons.Draw(painter, Host, item, box, large: false, Over(box), Pressed && pressedAt == i && Over(box));
        }
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
