// A ribbon (conformance/ribbons.json): its tabs in a strip, and under it the
// groups of the open tab, each a row of buttons (a large one with its icon
// above its caption) with its title below. The open tab, counted from 1,
// and whether a collapsible ribbon is folded to its tabs are the ribbon's
// part state (Tab, IsCollapsed), so code reads and sets them; an unnamed
// ribbon keeps them itself. A hidden tab gives way to the first visible
// one; opening a tab of a folded ribbon unfolds it, and a double click
// anywhere on it but on a button folds it again, as in an office suite.
// Tab reaches its strip of tabs, where Left and Right open the tab beside,
// and goes on into the buttons of the open tab.

using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class FrameRibbon : FrameRegion
{
    /// <summary>How high the strip of tabs is, and the groups under it.</summary>
    public const double TabsHeight = 30, BodyHeight = 92;

    const double TitleHeight = 18;

    readonly List<(int Tab, Box Box)> tabs = [];
    readonly List<(CommandSpec Item, Box Box)> buttons = [];
    readonly List<(RibbonGroupSpec Group, Box Box)> groups = [];
    readonly FrameButtonRow row;
    long ownTab = 1;
    bool ownCollapsed;

    public FrameRibbon(WindowFrame frame, RibbonSpec ribbon) : base(frame)
    {
        Ribbon = ribbon;
        ownCollapsed = ribbon.Collapsed;
        TabStrip = Hold(new RibbonTabStrip(this));
        row = new FrameButtonRow(this, ribbon.Tabs.SelectMany(t => t.Groups).SelectMany(g => g.Items), large: true, Click);
    }

    public RibbonSpec Ribbon { get; }

    /// <summary>Its strip of tabs: it takes the focus, and Left and Right open the tab beside.</summary>
    public RibbonTabStrip TabStrip { get; }

    PartState? State => Ribbon.Name is null ? null : Host.State(Ribbon.Name);

    /// <summary>Whether it shows: its state is not hidden and one of its tabs shows; with nothing to show it takes no room.</summary>
    public override bool IsShown => FrameRules.Shows(Host, Ribbon.Name) && ShownTab is not null;

    public bool Enabled => FrameRules.Takes(Host, Ribbon.Name);

    /// <summary>A disabled ribbon's tabs and buttons take no press, focus or key.</summary>
    public override bool IsEnabled => Enabled;

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
        row.Hide();
        TabStrip.Arrange(new Box(box.X, box.Y, box.Width, TabsHeight));
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
                var place = new Box(x, top, wide, high);
                buttons.Add((item, place));
                row.Place(item, place);
                x += wide + 4;
            }
            var title = fonts.Measure(Host.Text(group.Title), "text", look.Size("small"), false) + 12;
            x = Math.Max(x + 2, start + title);
            groups.Add((group, new Box(start, top, x - start, BodyHeight - 12)));
            x += 4;
        }
    }

    /// <summary>A double click on it, away from its buttons, folds or unfolds it.</summary>
    public override void OnPointerPressed(PointerInput at)
    {
        if (at.Clicks >= 2 && Enabled)
            Fold(!Collapsed);
    }

    /// <summary>Opens the visible tab <paramref name="by"/> places on from the one shown; none past either end.</summary>
    internal void Step(int by)
    {
        var shown = ShownTab;
        var at = tabs.FindIndex(t => t.Tab == shown);
        var next = at + by;
        if (at < 0 || next < 0 || next >= tabs.Count)
            return;
        Open(tabs[next].Tab);
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
        var small = look.Size("small");
        foreach (var (group, box) in groups)
        {
            painter.Text(Host.Text(group.Title), new Box(box.X, box.Bottom - TitleHeight, box.Width, TitleHeight), look.Color("muted"), small, align: TextAlign.Center);
            painter.Fill(new Box(box.Right, box.Y, look.Border, box.Height), look.Color("border"));
        }
        base.Draw(painter);
    }
}

/// <summary>
/// The strip of a ribbon's tabs: a press on a tab opens it, a double click
/// anywhere on the strip folds or unfolds the ribbon. It takes the focus
/// (Tab reaches it, the open tab ringed); Left and Right open the visible
/// tab beside, Enter or Space opens the one shown (a folded ribbon unfolds).
/// </summary>
public sealed class RibbonTabStrip : Element
{
    readonly FrameRibbon ribbon;
    (double X, double Y)? pointer;

    internal RibbonTabStrip(FrameRibbon ribbon) => this.ribbon = ribbon;

    public override bool Focusable => true;

    public override void OnPointerMoved(PointerInput at)
    {
        pointer = (at.ScreenX, at.ScreenY);
        Invalidate();
    }

    public override void OnPointerPressed(PointerInput at)
    {
        if (at.Clicks >= 2 && ribbon.Ribbon.Collapsible)
        {
            ribbon.Fold(!ribbon.Collapsed);
            return;
        }
        foreach (var (tab, box) in ribbon.Tabs)
            if (box.Contains(at.ScreenX, at.ScreenY))
            {
                ribbon.Open(tab);
                return;
            }
    }

    public override bool OnKey(KeyInput key)
    {
        if (key.Modifiers != Modifiers.None)
            return false;
        switch (key.Key)
        {
            case Key.Left:
                ribbon.Step(-1);
                return true;
            case Key.Right:
                ribbon.Step(1);
                return true;
            case Key.Enter or Key.Space when ribbon.ShownTab is { } shown:
                ribbon.Open(shown);
                return true;
        }
        return false;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var host = ribbon.Host;
        var shown = ribbon.Collapsed ? null : ribbon.ShownTab;
        var small = look.Size("small");
        foreach (var (tab, box) in ribbon.Tabs)
        {
            var open = tab == shown;
            var over = Hovered && pointer is { } p && box.Contains(p.X, p.Y);
            var values = look.Part("tab", over && !open ? "hover" : null);
            if (open)
                painter.Fill(box, look.Color("panel"), look.Radius("button"));
            else if (over)
                painter.Fill(box, values.Background ?? look.Color("accent_soft"), look.Radius("button"));
            painter.Text(host.Text(ribbon.Ribbon.Tabs[tab - 1].Title), box, open ? look.Color("text") : look.Color("frame_text"), small, bold: open, align: TextAlign.Center);
            // The focus ring goes round the tab shown, open or folded.
            if (Focused && tab == ribbon.ShownTab)
                PressLooks.FocusRing(painter, box, look.Radius("button"));
        }
    }
}
