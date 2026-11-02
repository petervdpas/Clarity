// What every container part shares (DESIGN 7.8, 7.9): a panel, a form, tabs,
// steps, an accordion, a splitter, a scroll box and a popover each hold grids
// of parts. A container grows when a part in it does, and it frames what it
// holds: a list, table or tree inside it shows no border of its own
// (DESIGN 7.8 "frames"). It draws again when its part's state or the window's
// data changes. Also here: the panel and the form, and the few pieces the
// containers draw with (a button in Clarity's own words, a header's chevron).

using System.ComponentModel;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>Something that frames the parts in it: a list, table or tree inside shows no border of its own.</summary>
public interface IFramesParts
{
    bool FramesParts { get; }
}

public abstract class ContainerPart : Part, IFramesParts
{
    readonly List<GridPanel> grids = [];

    /// <summary>Whether a list, table or tree in it shows no border of its own; every container frames what it holds.</summary>
    public virtual bool FramesParts => true;

    /// <summary>
    /// Whether <paramref name="part"/> is inside a container that frames it, so a
    /// list, table or tree draws no border of its own; on the window and in a
    /// sidebar it keeps one. Asked as it draws: as it is made it has no parent yet.
    /// </summary>
    public static bool IsInside(Element part)
    {
        for (var e = part.Parent; e is not null; e = e.Parent)
            if (e is IFramesParts frames)
                return frames.FramesParts;
        return false;
    }

    /// <summary>Every grid it holds, shown or not (a tab that is not open keeps its grid here).</summary>
    public IReadOnlyList<GridPanel> Grids => grids;

    /// <summary>A container grows when a part in it does.</summary>
    public override bool Grows => grids.Any(g => g.Children.OfType<Part>().Any(p => p.Grows));

    /// <summary>How many columns its own grids have: as many as it spans.</summary>
    protected int Columns => Math.Max(1, (int)(Spec.Number("width") ?? 1));

    /// <summary>The grid of a page or of its own parts, which it holds.</summary>
    protected GridPanel Grid(IReadOnlyList<PartSpec> parts, int columns)
    {
        var grid = GridOf(parts, columns);
        grids.Add(grid);
        return grid;
    }

    /// <summary>How high a grid of <paramref name="parts"/> is at the least: its rows at 36, and the room its own containers ask around theirs.</summary>
    protected static double LeastOf(GridPanel grid, IReadOnlyList<PartSpec> parts)
    {
        if (parts.Count == 0)
            return 0;
        var items = parts.Select((p, i) => p.Place(false) with { Around = i < grid.Children.Count ? grid.Children[i].Around : 0 }).ToList();
        return GridLayout.Place(items, grid.Columns, 1000, 0).Content;
    }

    /// <summary>How high its own rows are at the least.</summary>
    protected double OwnLeast
    {
        get
        {
            var rows = Math.Max(1, Spec.Place(false).Height);
            return rows * GridLayout.RowHeight + (rows - 1) * GridLayout.Gap;
        }
    }

    /// <summary>The room it asks beyond its rows: what it puts around its grid plus what the grid needs more than the rows give, never less than nothing.</summary>
    protected double AroundFor(double frame, double needed) => Math.Max(0, frame + needed - OwnLeast);

    /// <summary>Shows one of its grids or takes it away: one that is away draws nothing and takes no pointer.</summary>
    protected void Show(Element grid, bool shown)
    {
        var there = grid.Parent == this;
        if (shown && !there)
            AddChild(grid);
        else if (!shown && there)
            RemoveChild(grid);
    }

    /// <summary>A text as the project writes it, in the language shown.</summary>
    protected string Shown(string? written) => string.IsNullOrEmpty(written) ? "" : Host.Text(written);

    /// <summary>Its caption: as a handler set it, else as designed.</summary>
    protected string Caption => State?.Caption is { Length: > 0 } set ? set : Shown(Spec.Text("caption"));

    protected internal override void Made()
    {
        Host.Changed += Invalidate;
        if (State is INotifyPropertyChanged state)
            state.PropertyChanged += (_, _) => Invalidate();
    }

    public override void Draw(Painter painter)
    {
        if (IsShown)
            base.Draw(painter);
    }

    /// <summary>The frame around what it holds, out in the gap between columns as on the web, so its grid keeps the window's column lines.</summary>
    internal static Box FrameOf(Box box) => new(box.X - 3, box.Y, box.Width + 6, box.Height);

    /// <summary>How high a header is: a line of small text with room around it.</summary>
    internal static double HeaderHeight(ThemeLook look) => look.Size("small") + 4 * look.Padding;

    /// <summary>A chevron pointing down (open) or right (folded), in a text's color, centered in <paramref name="box"/>.</summary>
    internal static void Chevron(Painter painter, Box box, SKColor color, bool open)
    {
        using var path = SKPath.ParseSvgPathData(open ? "M 0 0 L 8 0 L 4 5 Z" : "M 0 0 L 5 4 L 0 8 Z");
        using var paint = new SKPaint { Color = color, IsAntialias = true, Style = SKPaintStyle.Fill };
        var (w, h) = open ? (8.0, 5.0) : (5.0, 8.0);
        painter.Canvas.Save();
        painter.Canvas.Translate((float)(box.X + (box.Width - w) / 2), (float)(box.Y + (box.Height - h) / 2));
        painter.Canvas.DrawPath(path, paint);
        painter.Canvas.Restore();
    }
}

/// <summary>
/// A button a container draws in Clarity's own words or a page's title: a
/// wizard's Back, Next and Finish, an accordion's section header. It looks as
/// the theme's button does (hover, pressed), the accent one in the accent.
/// </summary>
public sealed class ContainerButton(Func<string> text, Action pressed) : Element
{
    public string Text => text();

    /// <summary>The window's default action here (Next, Finish): drawn in the accent.</summary>
    public bool Accent { get; init; }

    /// <summary>Its text on the left with a chevron before it, as a section's header.</summary>
    public bool? Chevron { get; set; }

    /// <summary>Whether it is shown: a wizard shows only the buttons its step has.</summary>
    public Func<bool>? Shows { get; init; }

    /// <summary>Whether the person may press it.</summary>
    public Func<bool>? Usable { get; init; }

    public override bool IsShown => Shows?.Invoke() ?? true;

    public override bool Focusable => Usable?.Invoke() ?? true;

    /// <summary>Pressed, as a click or Enter or Space does.</summary>
    public void Press()
    {
        if (IsShown && (Usable?.Invoke() ?? true))
            pressed();
    }

    public override void OnClick() => Press();

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is not (Key.Enter or Key.Space))
            return false;
        Press();
        return true;
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        var values = look.Part("button", Pressed ? "pressed" : Hovered ? "hover" : null);
        var radius = values.Radius ?? look.Radius("button");
        var background = Accent ? look.Color("accent") : values.Background ?? look.Color("raised");
        var color = Accent ? look.Color("on_accent") : values.Text ?? look.Color("text");
        var usable = Usable?.Invoke() ?? true;
        if (!usable)
            color = look.Color("muted");
        painter.Fill(Bounds, background, radius);
        painter.Stroke(Bounds, Accent ? background : values.Border ?? look.Color("strong_border"), look.Border, radius);
        if (Focused)
            painter.Stroke(Bounds.Inset(-2), look.Color("focus"), 2, radius + 2);
        var size = values.Size ?? look.Size("text");
        if (Chevron is { } open)
        {
            var pad = 2 * look.Padding;
            ContainerPart.Chevron(painter, new Box(Bounds.X + pad, Bounds.Y, 10, Bounds.Height), color, open);
            painter.Text(Text, new Box(Bounds.X + pad + 18, Bounds.Y, Math.Max(0, Bounds.Width - pad * 2 - 18), Bounds.Height), color, size, values.Font ?? "text", values.Bold || open);
        }
        else
            painter.Text(Text, Bounds.Inset(look.Padding), color, size, values.Font ?? "text", values.Bold, TextAlign.Center);
    }

    /// <summary>How wide it is for its text.</summary>
    public double WidthFor(Fonts? fonts, ThemeLook look)
    {
        var size = look.Size("text");
        var text = Text;
        var measured = fonts?.Measure(text, "text", size, false) ?? text.Length * size * 0.6;
        return measured + 6 * look.Padding;
    }
}

/// <summary>
/// A panel or a form: a frame with its caption in a header (the theme's
/// header look) around a grid of its parts, as wide as it spans. A form
/// without a caption has no frame, and its grid lines up with the grid
/// around it. A foldable one folds to its header when the person presses the
/// header, as its part's state keeps (conformance/folding.json).
/// </summary>
[PartKind("panel", "form")]
public sealed class PanelPart : ContainerPart
{
    GridPanel grid = null!;
    bool foldedHere;
    bool pressedHeader;

    public GridPanel Inner => grid;

    public bool Foldable => Spec.Flag("foldable");

    /// <summary>A form without a caption: no frame and no header.</summary>
    public bool Bare => Spec.Kind == "form" && string.IsNullOrEmpty(Spec.Text("caption")) && !Foldable;

    /// <summary>Folded, as the person or a handler left it in its part's state, else as designed.</summary>
    public bool IsFolded => Foldable && (State?.IsFolded ?? foldedHere);

    /// <summary>A folded part grows no more: it is its header.</summary>
    public override bool Grows => !IsFolded && base.Grows;

    protected internal override void Made()
    {
        base.Made();
        foldedHere = Spec.Flag("folded");
        grid = Grid(Spec.Children, Columns);
        AddChild(grid);
    }

    /// <summary>Folds it, or unfolds it, as a press on its header does.</summary>
    public void ToggleFold()
    {
        if (!Foldable || !Enabled)
            return;
        if (State is { } state)
            state.IsFolded = !state.IsFolded;
        else
            foldedHere = !foldedHere;
        Invalidate();
    }

    double Header => Bare ? 0 : HeaderHeight(LookNow);

    /// <summary>The room it needs beyond its rows: its header and padding, when its grid needs more than its rows give.</summary>
    public override double Around => AroundFor(Bare ? 0 : Header + 2 * LookNow.Padding, LeastOf(grid, Spec.Children));

    public override void Arrange(Box box)
    {
        var folded = IsFolded;
        Show(grid, !folded);
        Bounds = folded ? box with { Height = Math.Min(box.Height, Header) } : box;
        if (Bare)
            grid.Arrange(box);
        else
        {
            var pad = LookNow.Padding;
            grid.Arrange(new Box(box.X, box.Y + Header + pad, box.Width, Math.Max(0, box.Height - Header - 2 * pad)));
        }
    }

    public override void OnPointerPressed(PointerInput at) => pressedHeader = at.Y < Header;

    public override void OnClick()
    {
        if (pressedHeader)
            ToggleFold();
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        if (!Bare)
        {
            var look = painter.Look;
            var radius = look.Radius("panel");
            var frame = FrameOf(Bounds);
            var header = look.Part("header", null);
            var headerBox = frame with { Height = HeaderHeight(look) };
            using (painter.Clip(headerBox))
                painter.Fill(frame.Inset(look.Border), header.Background ?? look.Color("raised"), radius);
            painter.Stroke(frame, look.Color("strong_border"), look.Border, radius);
            var color = header.Text ?? look.Color("muted");
            var size = header.Size ?? look.Size("small");
            var x = frame.X + 2 * look.Padding;
            if (Foldable)
            {
                Chevron(painter, new Box(x, headerBox.Y, 10, headerBox.Height), color, !IsFolded);
                x += 16;
            }
            painter.Text(Caption, new Box(x, headerBox.Y, Math.Max(0, frame.Right - x - 2 * look.Padding), headerBox.Height), color, size, header.Font ?? "text", header.Bold);
        }
        base.Draw(painter);
    }
}
