// A splitter: panes side by side (or one above the other when vertical),
// each a grid of its parts, with a divider between them the person drags.
// `pane span 4` gives the first of two side by side panes 4 of the part's
// columns and the second the rest (clarity_spec::widgets::pane_columns);
// otherwise each pane has all the part's columns and they share the room.
// A pane whose parts go further down than its room scrolls, so none of them
// is out of reach in a low window.

namespace Clarity.Ui;

[PartKind("splitter")]
public sealed class SplitterPart : ContainerPart
{
    readonly List<GridPanel> panes = [];
    readonly List<ScrollView> views = [];
    readonly List<Divider> dividers = [];
    double[] shares = [];

    /// <summary>How thick a divider is.</summary>
    public const double DividerThickness = 4;

    public bool Vertical => Spec.Text("orientation") == "vertical";

    public IReadOnlyList<GridPanel> Panes => panes;

    public IReadOnlyList<Element> Dividers => dividers;

    /// <summary>The place of each pane, which scrolls its grid when the grid is higher.</summary>
    public IReadOnlyList<ScrollView> Views => views;

    /// <summary>How much of the room each pane has, from 0 to 1, as designed or as the person dragged it.</summary>
    public IReadOnlyList<double> Shares => shares;

    /// <summary>The columns of each pane: <c>pane span n</c> on the first of two side by side, else all of the part's for each.</summary>
    public static int[] PaneColumns(IReadOnlyList<PageSpec> pages, int width, bool vertical)
    {
        if (!vertical && pages.Count == 2 && pages[0].Columns is { } first && first >= 1 && first < width)
            return [first, width - first];
        return [.. pages.Select(_ => width)];
    }

    protected internal override void Made()
    {
        base.Made();
        var columns = PaneColumns(Spec.Pages, Columns, Vertical);
        for (var i = 0; i < Spec.Pages.Count; i++)
        {
            if (i > 0)
            {
                var divider = new Divider(this, i);
                dividers.Add(divider);
                AddChild(divider);
            }
            var grid = Grid(Spec.Pages[i].Children, columns[i]);
            panes.Add(grid);
            var view = new ScrollView(grid) { KeepsBarRoom = true };
            views.Add(view);
            AddChild(view);
        }
        var total = columns.Sum();
        shares = [.. columns.Select(c => total == 0 ? 0 : (double)c / total)];
    }

    /// <summary>The room it needs beyond its rows: each pane's padding and grid, side by side the highest, one above the other all of them with the dividers.</summary>
    public override double Around
    {
        get
        {
            var pad = 4 * LookNow.Padding;
            var needs = panes.Select((g, i) => pad + LeastOf(g, Spec.Pages[i].Children)).ToList();
            if (needs.Count == 0)
                return 0;
            return AroundFor(0, Vertical ? needs.Sum() + dividers.Count * DividerThickness : needs.Max());
        }
    }

    double Room => (Vertical ? Bounds.Height : Bounds.Width) - DividerThickness * dividers.Count;

    public override void Arrange(Box box)
    {
        Bounds = box;
        var look = LookNow;
        var pad = 2 * look.Padding;
        var at = Vertical ? box.Y : box.X;
        for (var i = 0; i < panes.Count; i++)
        {
            if (i > 0)
            {
                dividers[i - 1].Arrange(Vertical ? new Box(box.X, at, box.Width, DividerThickness) : new Box(at, box.Y, DividerThickness, box.Height));
                at += DividerThickness;
            }
            var length = Math.Max(0, Room * shares[i]);
            var pane = Vertical ? new Box(box.X, at, box.Width, length) : new Box(at, box.Y, length, box.Height);
            views[i].Arrange(pane.Inset(pad));
            at += length;
        }
    }

    /// <summary>The divider before pane <paramref name="index"/> dragged to <paramref name="along"/>, in the screen's units: the two panes beside it share their room anew, each keeping a tenth of the part at least.</summary>
    public void DragDivider(int index, double along)
    {
        var room = Room;
        if (room <= 0)
            return;
        var start = Vertical ? Bounds.Y : Bounds.X;
        var before = shares.Take(index - 1).Sum() * room + (index - 1) * DividerThickness;
        var pair = shares[index - 1] + shares[index];
        var least = Math.Min(0.1, pair / 2);
        var first = Math.Clamp((along - start - before) / room, least, pair - least);
        shares[index - 1] = first;
        shares[index] = pair - first;
        Arrange(Bounds);
        Invalidate();
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        var radius = look.Radius("panel");
        using (painter.Clip(Bounds))
            base.Draw(painter);
        painter.Stroke(Bounds, look.Color("strong_border"), look.Border, radius);
    }

    /// <summary>The line between two panes, which the person drags.</summary>
    sealed class Divider(SplitterPart splitter, int index) : Element
    {
        double grab;

        public override void OnPointerPressed(PointerInput at) => grab = splitter.Vertical ? at.Y : at.X;

        /// <summary>A divider shows it is dragged along the splitter.</summary>
        public override Cursor? CursorAt(double x, double y) => splitter.Vertical ? Cursor.ResizeUpDown : Cursor.ResizeLeftRight;

        public override void OnPointerMoved(PointerInput at)
        {
            if (Pressed && splitter.Enabled)
                splitter.DragDivider(index, (splitter.Vertical ? at.ScreenY : at.ScreenX) - grab);
        }

        public override void Draw(Painter painter) =>
            painter.Fill(Bounds, painter.Look.Color(Pressed || Hovered ? "accent" : "strong_border"));
    }
}
