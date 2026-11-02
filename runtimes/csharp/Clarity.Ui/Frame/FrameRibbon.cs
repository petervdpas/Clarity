// A ribbon (conformance/ribbons.json): its tabs in a strip, and under it the
// groups of the open tab, each a row of buttons (a large one with its icon
// above its caption) with its title below. The open tab, counted from 1,
// and whether a collapsible ribbon is folded to its tabs are the ribbon's
// part state (Tab, IsCollapsed), so code reads and sets them; an unnamed
// ribbon keeps them itself. A hidden tab gives way to the first visible
// one; opening a tab of a folded ribbon unfolds it, and a double click on
// the tabs folds it again, as in an office suite.

using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class FrameRibbon(WindowFrame frame, RibbonSpec ribbon) : FrameRegion(frame)
{
    /// <summary>How high the strip of tabs is, and the groups under it.</summary>
    public const double TabsHeight = 30, BodyHeight = 92;

    const double TitleHeight = 18;

    readonly List<(int Tab, Box Box)> tabs = [];
    readonly List<(CommandSpec Item, Box Box)> buttons = [];
    readonly List<(RibbonGroupSpec Group, Box Box)> groups = [];
    long ownTab = 1;
    bool ownCollapsed = ribbon.Collapsed;
    int pressedAt = -1;

    public RibbonSpec Ribbon { get; } = ribbon;

    PartState? State => Ribbon.Name is null ? null : Host.State(Ribbon.Name);

    /// <summary>Whether it shows: its state is not hidden and one of its tabs shows; with nothing to show it takes no room.</summary>
    public override bool IsShown => FrameRules.Shows(Host, Ribbon.Name) && ShownTab is not null;

    public bool Enabled => FrameRules.Takes(Host, Ribbon.Name);

    /// <summary>Whether it is folded to its tabs: only a collapsible one, as its state says.</summary>
    public bool Collapsed => Ribbon.Collapsible && (State?.IsCollapsed ?? ownCollapsed);

    /// <summary>The tab it shows, from 1: the open one while it is visible, else the first visible one; none when no tab is.</summary>
    public int? ShownTab => FrameRules.ShownTab([.. Ribbon.Tabs.Select(t => FrameRules.Shows(Host, t.Name))], State?.Tab ?? ownTab);

    public override double Wanted => TabsHeight + (Collapsed || ShownTab is null ? 0 : BodyHeight);

    /// <summary>Its tabs as they show, each with its place (from 1).</summary>
    public IReadOnlyList<(int Tab, Box Box)> Tabs => tabs;

    /// <summary>A button says its caption and its keys when the pointer rests on it.</summary>
    public override string? TooltipAt(double x, double y) =>
        buttons.FirstOrDefault(b => b.Box.Contains(x, y)) is { Item: { } item } && !FrameRules.IsSeparator(item)
            ? FrameRules.TipOf(Host.Text(item.Text("caption")), Frame.KeysOf(item, Frame.Shown.Ribbon?.Inherited == true))
            : null;

    /// <summary>The buttons of the tab shown, each with its place.</summary>
    public IReadOnlyList<(CommandSpec Item, Box Box)> Buttons => buttons;

    /// <summary>The person opened tab <paramref name="tab"/> (from 1): it shows, and a folded ribbon unfolds.</summary>
    public void Open(int tab)
    {
        if (State is { } state)
        {
            state.Tab = tab;
            state.IsCollapsed = false;
        }
        else
        {
            ownTab = tab;
            ownCollapsed = false;
        }
        Invalidate();
    }

    /// <summary>Folds a collapsible ribbon to its tabs, or unfolds it.</summary>
    public void Fold(bool folded)
    {
        if (!Ribbon.Collapsible)
            return;
        if (State is { } state)
            state.IsCollapsed = folded;
        else
            ownCollapsed = folded;
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        tabs.Clear();
        buttons.Clear();
        groups.Clear();
        var look = LookNow;
        var fonts = FontsNow;
        var x = box.X + 6;
        for (var i = 0; i < Ribbon.Tabs.Count; i++)
        {
            var tab = Ribbon.Tabs[i];
            if (!FrameRules.Shows(Host, tab.Name))
                continue;
            var wide = fonts.Measure(Host.Text(tab.Title), "text", look.Size("small"), false) + 28;
            tabs.Add((i + 1, new Box(x, box.Y + 4, wide, TabsHeight - 4)));
            x += wide + 2;
        }
        if (Collapsed || ShownTab is not { } shown)
            return;
        x = box.X + 6;
        var top = box.Y + TabsHeight + 6;
        foreach (var group in Ribbon.Tabs[shown - 1].Groups.Where(g => FrameRules.Shows(Host, g.Name)))
        {
            var start = x;
            x += 6;
            foreach (var item in group.Items.Where(i => FrameRules.Shows(Host, i.Text("name"))))
            {
                var wide = FrameButtons.Width(this, item, large: true);
                var high = item.Flag("large") || FrameRules.IsSeparator(item) ? FrameButtons.LargeHeight : 28;
                buttons.Add((item, new Box(x, top, wide, high)));
                x += wide + 4;
            }
            var title = fonts.Measure(Host.Text(group.Title), "text", look.Size("small"), false) + 12;
            x = Math.Max(x + 2, start + title);
            groups.Add((group, new Box(start, top, x - start, BodyHeight - 12)));
            x += 4;
        }
    }

    public override void OnPointerPressed(PointerInput at)
    {
        pressedAt = buttons.FindIndex(b => b.Box.Contains(at.ScreenX, at.ScreenY));
        if (!Enabled)
            return;
        foreach (var (tab, box) in tabs)
            if (box.Contains(at.ScreenX, at.ScreenY))
            {
                // A double click on a tab folds or unfolds the ribbon; a press opens the tab.
                if (at.Clicks >= 2 && Ribbon.Collapsible)
                    Fold(!Collapsed);
                else
                    Open(tab);
                return;
            }
        if (at.Clicks >= 2 && Ribbon.Collapsible && at.ScreenY < Bounds.Y + TabsHeight)
            Fold(!Collapsed);
    }

    public override void OnClick()
    {
        if (pressedAt >= 0 && pressedAt < buttons.Count)
            Click(buttons[pressedAt].Item);
    }

    /// <summary>A button of the tab shown clicked: it runs, unless it or the ribbon is disabled.</summary>
    public void Click(CommandSpec item)
    {
        var at = buttons.FindIndex(b => ReferenceEquals(b.Item, item));
        if (!Enabled || at < 0 || FrameRules.IsSeparator(item) || !FrameRules.Takes(Host, item.Text("name")))
            return;
        Frame.Run(item, buttons[at].Box, Screen);
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        painter.Fill(Bounds, look.Color("panel"));
        painter.Fill(new Box(Bounds.X, Bounds.Y, Bounds.Width, TabsHeight), look.Color("frame"));
        painter.Fill(new Box(Bounds.X, Bounds.Bottom - look.Border, Bounds.Width, look.Border), look.Color("border"));
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        var shown = Collapsed ? null : ShownTab;
        var small = look.Size("small");
        foreach (var (tab, box) in tabs)
        {
            var open = tab == shown;
            var values = look.Part("tab", Over(box) && !open ? "hover" : null);
            if (open)
                painter.Fill(box, look.Color("panel"), look.Radius("button"));
            else if (Over(box))
                painter.Fill(box, values.Background ?? look.Color("accent_soft"), look.Radius("button"));
            painter.Text(Host.Text(Ribbon.Tabs[tab - 1].Title), box, open ? look.Color("text") : look.Color("frame_text"), small, bold: open, align: TextAlign.Center);
        }
        foreach (var (group, box) in groups)
        {
            painter.Text(Host.Text(group.Title), new Box(box.X, box.Bottom - TitleHeight, box.Width, TitleHeight), look.Color("muted"), small, align: TextAlign.Center);
            painter.Fill(new Box(box.Right, box.Y, look.Border, box.Height), look.Color("border"));
        }
        for (var i = 0; i < buttons.Count; i++)
        {
            var (item, box) = buttons[i];
            FrameButtons.Draw(painter, Host, item, box, large: true, Over(box), Pressed && pressedAt == i && Over(box));
        }
    }
}
