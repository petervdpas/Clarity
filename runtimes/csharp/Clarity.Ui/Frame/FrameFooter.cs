// The footer at the bottom of a window: its status text at the left, in
// the language shown, and its parts on one row of its columns beside it.

namespace Clarity.Ui;

public sealed class FrameFooter : FrameRegion
{
    /// <summary>How high a footer is with parts, and with its status alone.</summary>
    public const double PartsHeight = GridLayout.RowHeight + 8, StatusHeight = 28;

    readonly GridPanel grid;
    readonly List<Part> parts = [];

    public FrameFooter(WindowFrame frame, FooterSpec footer) : base(frame)
    {
        Footer = footer;
        grid = new GridPanel(Math.Max(1, footer.Columns));
        foreach (var spec in footer.Parts)
        {
            var part = PartKinds.Make(spec, frame.Host);
            parts.Add(part);
            // One row: a part that grows keeps the row's height.
            var place = spec.Place(false);
            grid.Add(part, place with { Row = 1, Height = 1 });
        }
        AddChild(grid);
    }

    public FooterSpec Footer { get; }

    public IReadOnlyList<Part> Parts => parts;

    /// <summary>The status as shown, in the language shown; none without one.</summary>
    public string? Status => Footer.Status is { } status ? Host.Text(status) : null;

    public override double Wanted => parts.Count > 0 ? PartsHeight : StatusHeight;

    /// <summary>Where the status text is.</summary>
    public Box StatusBox { get; private set; }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var width = Status is { } status ? FontsNow.Measure(status, "text", LookNow.Size("small"), false) + 16 : 0;
        StatusBox = new Box(box.X + 4, box.Y, width, box.Height);
        var x = StatusBox.Right + (width > 0 ? 6 : 4);
        grid.Arrange(new Box(x, box.Y + (box.Height - GridLayout.RowHeight) / 2, Math.Max(0, box.Right - 8 - x), GridLayout.RowHeight));
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        Plate(painter, Bounds, new Box(Bounds.X, Bounds.Y, Bounds.Width, look.Border));
        if (Status is { } status)
            painter.Text(status, StatusBox.Inset(4) with { Y = StatusBox.Y, Height = StatusBox.Height }, look.Color("muted"), look.Size("small"));
        if (parts.Count > 0)
            base.Draw(painter);
    }
}
