// A popover (DESIGN 5.16): parts on a grid of their own, shown over the
// window while its part's state is visible, next to the part named in its
// `for` (below it, or above when there is no room under it), else at its own
// place. In the window's grid it takes no room and no pointer. A press
// outside it or Escape closes it, and its part's state hears that it is no
// longer visible, so handlers open and close it with `visible`. A popover
// without a name has no state and never opens.

using System.ComponentModel;

namespace Clarity.Ui;

[PartKind("popover")]
public sealed class PopoverPart : ContainerPart
{
    /// <summary>How wide a column of a popover's grid is.</summary>
    public const double Column = 80;

    Overlay? overlay;
    PopoverFrame frame = null!;

    /// <summary>What lies over the window while it is open; null while closed.</summary>
    public Overlay? Opened => overlay;

    public GridPanel Inner { get; private set; } = null!;

    /// <summary>Whether it is open: its part's state is visible.</summary>
    public bool IsOpen => State?.IsVisible ?? false;

    /// <summary>In the window's grid it shows nothing: what it holds lies over the window.</summary>
    public override bool IsShown => false;

    /// <summary>It takes no room in the window's grid, so it grows by none.</summary>
    public override bool Grows => false;

    protected internal override void Made()
    {
        base.Made();
        Inner = Grid(Spec.Children, Columns);
        frame = new PopoverFrame(Inner);
        if (State is INotifyPropertyChanged state)
            state.PropertyChanged += (_, _) => Follow();
    }

    /// <summary>How big it is: its columns and rows, with its padding and border around them.</summary>
    public (double Width, double Height) Size(ThemeLook look)
    {
        var columns = Columns;
        var rows = Math.Max(1, (int)(Spec.Number("height") ?? 1));
        var around = 2 * (2 * look.Padding + look.Border);
        return (Column * columns + GridLayout.Gap * (columns - 1) + around, GridLayout.RowHeight * rows + GridLayout.Gap * (rows - 1) + around);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        Follow();
    }

    /// <summary>Opens or closes it as its part's state says, at its place.</summary>
    void Follow()
    {
        if (Screen is not { } screen)
            return;
        if (!IsOpen)
        {
            if (overlay is { } open)
            {
                overlay = null;
                screen.Close(open);
            }
            return;
        }
        var place = Place(screen);
        if (overlay is null)
        {
            // A press on the part it is for is no press outside: that part's click opens or closes it.
            overlay = new Overlay(frame, place) { Closed = Closed, Anchor = () => Near(screen)?.Bounds ?? default };
            screen.Open(overlay);
        }
        else
            overlay.Place = place;
    }

    void Closed()
    {
        if (overlay is null)
            return;
        overlay = null;
        if (State is { } state)
            state.IsVisible = false;
    }

    /// <summary>The part it is for, when it shows.</summary>
    Part? Near(Screen screen) => Spec.Text("for") is { Length: > 0 } name
        ? screen.Root.Find<Part>(p => p.Spec.Name == name && p.ShowsInTree)
        : null;

    /// <summary>Where it lies on the screen: under its part, above it when there is no room below, else at its own place; inside the screen.</summary>
    Box Place(Screen screen)
    {
        var (width, height) = Size(screen.Look);
        var near = Near(screen);
        double x, y;
        if (near is not null)
        {
            var at = near.Bounds;
            x = at.X;
            y = at.Bottom + 4;
            if (y + height > screen.Height && at.Y - height - 4 > 0)
                y = at.Y - height - 4;
        }
        else
            (x, y) = (Bounds.X, Bounds.Y);
        x = Math.Max(4, Math.Min(x, screen.Width - width - 4));
        return new Box(x, y, width, height);
    }

    /// <summary>What lies over the window: the panel's color with a border, its grid inside.</summary>
    sealed class PopoverFrame : Element, IFramesParts
    {
        readonly GridPanel grid;

        public PopoverFrame(GridPanel grid)
        {
            this.grid = grid;
            AddChild(grid);
        }

        public bool FramesParts => true;

        public override void Arrange(Box box)
        {
            Bounds = box;
            var look = LookNow;
            grid.Arrange(box.Inset(2 * look.Padding + look.Border));
        }

        public override void Draw(Painter painter)
        {
            var look = painter.Look;
            var radius = look.Radius("panel");
            painter.Fill(Bounds, look.Color("panel"), radius);
            painter.Stroke(Bounds, look.Color("strong_border"), look.Border, radius);
            base.Draw(painter);
        }
    }
}
