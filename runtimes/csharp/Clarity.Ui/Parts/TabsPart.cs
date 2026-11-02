// Tabs: a strip of tabs in the theme's tab look, and under it the open
// tab's page, a grid of its parts as wide as the part spans. The open tab is
// its part's state's Tab, counted from 1, so code reads and sets it; a tab
// whose parts code has all hidden gives way to the first one that shows, as
// a ribbon's tab does (conformance/ribbons.json `shown`).

namespace Clarity.Ui;

[PartKind("tabs")]
public sealed class TabsPart : ContainerPart
{
    readonly List<(PageSpec Page, GridPanel Grid, TabButton Tab)> pages = [];
    long tabHere = 1;

    /// <summary>How high the strip of tabs is.</summary>
    public const double StripHeight = 32;

    /// <summary>The tabs of the strip, in order.</summary>
    public IReadOnlyList<Element> Tabs => [.. pages.Select(p => (Element)p.Tab)];

    public IReadOnlyList<GridPanel> Pages => [.. pages.Select(p => p.Grid)];

    /// <summary>The tab shown, from 1, of tabs that show or not, when <paramref name="asked"/> is wanted: it, when it is there and shows, else the first that shows; null when none does.</summary>
    public static long? ShownTab(IReadOnlyList<bool> visible, long asked)
    {
        if (asked >= 1 && asked <= visible.Count && visible[(int)asked - 1])
            return asked;
        for (var i = 0; i < visible.Count; i++)
            if (visible[i])
                return i + 1;
        return null;
    }

    /// <summary>The tab asked to be open, from 1: the part's state's, or its own without a name.</summary>
    public long Tab
    {
        get => State?.Tab ?? tabHere;
        set
        {
            if (State is { } state)
                state.Tab = value;
            else
                tabHere = value;
            Invalidate();
        }
    }

    /// <summary>The tab open now, from 1; null when no tab shows.</summary>
    public long? Open => ShownTab([.. pages.Select(p => PageShows(p.Page, p.Grid))], Tab);

    /// <summary>A page shows unless code hid every part on it.</summary>
    static bool PageShows(PageSpec page, GridPanel grid) => page.Children.Count == 0 || grid.Children.Any(c => c.IsShown);

    protected internal override void Made()
    {
        base.Made();
        foreach (var page in Spec.Pages)
        {
            var number = pages.Count + 1;
            var grid = Grid(page.Children, Columns);
            var tab = new TabButton(this, number, () => Shown(page.Title));
            AddChild(tab);
            pages.Add((page, grid, tab));
        }
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var look = LookNow;
        var x = box.X;
        foreach (var (_, _, tab) in pages)
        {
            var width = tab.WidthFor(Screen?.Fonts, look);
            tab.Arrange(new Box(x, box.Y, width, StripHeight));
            x += width + 2;
        }
        var open = Open;
        for (var i = 0; i < pages.Count; i++)
        {
            var shown = open == i + 1;
            Show(pages[i].Grid, shown);
            if (shown)
                pages[i].Grid.Arrange(new Box(box.X, box.Y + StripHeight + 2 * look.Padding, box.Width, Math.Max(0, box.Height - StripHeight - 4 * look.Padding)));
        }
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        var radius = look.Radius("panel");
        var page = FrameOf(Bounds with { Y = Bounds.Y + StripHeight, Height = Math.Max(0, Bounds.Height - StripHeight) });
        painter.Fill(page, look.Color("panel"), radius);
        painter.Stroke(page, look.Color("strong_border"), look.Border, radius);
        base.Draw(painter);
    }

    /// <summary>A tab of the strip: pressed, it opens its page.</summary>
    sealed class TabButton(TabsPart tabs, int number, Func<string> title) : Element
    {
        public override bool Focusable => tabs.Enabled;

        public double WidthFor(Fonts? fonts, ThemeLook look)
        {
            var size = look.Size("text");
            var text = title();
            return (fonts?.Measure(text, "text", size, false) ?? text.Length * size * 0.6) + 6 * look.Padding;
        }

        public override bool IsShown => tabs.pages.Count < number || PageShows(tabs.pages[number - 1].Page, tabs.pages[number - 1].Grid);

        public override void OnClick()
        {
            if (tabs.Enabled)
                tabs.Tab = number;
        }

        public override bool OnKey(KeyInput key)
        {
            switch (key.Key)
            {
                case Key.Enter or Key.Space:
                    OnClick();
                    return true;
                case Key.Left or Key.Right when tabs.Enabled:
                    var next = number + (key.Key == Key.Left ? -1 : 1);
                    if (next < 1 || next > tabs.pages.Count)
                        return false;
                    tabs.Tab = next;
                    Screen?.Focus(tabs.pages[next - 1].Tab);
                    return true;
            }
            return false;
        }

        public override void Draw(Painter painter)
        {
            if (!IsShown)
                return;
            var look = painter.Look;
            var open = tabs.Open == number;
            var values = look.Part("tab", Hovered && !open ? "hover" : null);
            var radius = values.Radius ?? look.Radius("button");
            var background = open ? look.Color("panel") : values.Background ?? look.Color("raised");
            var color = open ? look.Color("text") : values.Text ?? look.Color("muted");
            // Its top corners rounded: the bottom runs on into the page under it.
            var box = Bounds with { Height = Bounds.Height + radius };
            using (painter.Clip(Bounds with { Height = Bounds.Height + (open ? look.Border : 0) }))
            {
                painter.Fill(box, background, radius);
                painter.Stroke(box, open ? look.Color("strong_border") : values.Border ?? look.Color("border"), look.Border, radius);
                if (open)
                    painter.Fill(new Box(Bounds.X + look.Border, Bounds.Bottom - look.Border, Bounds.Width - 2 * look.Border, 2 * look.Border), background);
            }
            if (Focused)
                painter.Stroke(Bounds.Inset(2), look.Color("focus"), 2, radius);
            painter.Text(title(), Bounds.Inset(look.Padding), color, values.Size ?? look.Size("text"), values.Font ?? "text", values.Bold, TextAlign.Center);
        }
    }
}
