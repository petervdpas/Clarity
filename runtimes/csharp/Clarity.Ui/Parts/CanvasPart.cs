// A canvas drawn by code (DESIGN 5.7, conformance/drawing.json): the shapes
// its draw handler drew (its state's Drawing), in the canvas's own units
// (`units`, 100 x 100 without them), scaled to fit its place in proportion
// and centered. A color is the theme's by name or written out (#rrggbb).
// A canvas with `on click` runs it with where it was clicked, in its units;
// `on drag` runs as a pressed pointer moves, with where it was pressed, and
// `on drop` when it is let go. A drag that moves nothing (the drawing stays
// the same after its first run), and any drag without `on drag`, pans a
// canvas with `zoom`, which moves as Clarity's own diagram designer does: it
// starts whole, the wheel pans, and the buttons in its corner zoom out, back
// to whole, and in. With a grid it shows a dot every so many units over the
// whole view, after the drawing's clear. With `on bend` its links bend: a
// press on one adds a point there or takes the point under it, dragging
// moves that point, a double click on a point takes it away, and letting go
// runs `on bend` with the link's name and its points. With a mode the person
// drags its objects, paints or draws (CanvasPart.Modes.cs). A canvas that
// `runs` an engine beside the screen is drawn by it (EngineCanvasPart.cs).
// The window runs its handlers through its host (IPartHost.Pointed, Acted, Engine).

using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>What a canvas shows: the pixels a unit takes, and the unit at its top left corner.</summary>
public readonly record struct CanvasViewport(double Scale, double X, double Y);

[PartKind("canvas")]
public sealed partial class CanvasPart : Part
{
    /// <summary>A canvas's units when it names none, as on every renderer.</summary>
    public const double DefaultUnits = 100;

    /// <summary>A pointer moved this many pixels is a drag, not a click.</summary>
    const double Still = 4;


    /// <summary>The engine beside the screen that draws it; none for a canvas its code draws.</summary>
    public EngineCanvasPart? Engine { get; private set; }

    protected internal override void Made()
    {
        Host.Changed += Heard;
        if (Spec.Text("runs") is { Length: > 0 } runs)
        {
            Engine = new EngineCanvasPart { Spec = Spec, Host = Host, Canvas = this };
            Engine.Runs = Host.Engine(runs);
            AddChild(Engine);
        }
    }

    /// <summary>The window changed: a drawing drawn anew keeps a drag going and stops showing a bend of its own.</summary>
    void Heard()
    {
        if (!ReferenceEquals(seen, Drawing))
        {
            seen = Drawing;
            bending = null;
            DrawingChanged();
        }
        Invalidate();
    }

    IReadOnlyList<Shape>? seen;

    /// <summary>The width of the canvas in its own units.</summary>
    public double UnitsWidth => Units("width");

    public double UnitsHeight => Units("height");

    double Units(string side) =>
        Spec.Json("units") is System.Text.Json.Nodes.JsonObject units && CanvasModes.Number(units[side]) is { } n && n > 0 ? n : DefaultUnits;

    /// <summary>Its drawing: the shapes its draw handler drew last.</summary>
    public IReadOnlyList<Shape> Drawing => State?.Drawing ?? [];

    public bool Zoom => Spec.Flag("zoom") && Engine is null;

    // A canvas's grid, background and dots start as its line says; code changes them in its state.
    public double Grid => State is { } s ? s.Grid : Spec.Number("grid") ?? 0;

    public bool Snap => State is { } s ? s.Snap : Spec.Flag("snap");

    /// <summary>What a clear without a color fills it with; empty is the theme's panel.</summary>
    public string Paper => State is { Background.Length: > 0 } s ? s.Background : Spec.Text("background") ?? "";

    public double DotSize => State is { } s ? s.DotSize : Spec.Number("dotSize") ?? 2;

    public string DotColor => State is { DotColor.Length: > 0 } s ? s.DotColor : Spec.Text("dotColor") ?? "";


    bool Handles(string @event) => Spec.On(@event).Any();

    // -- The view ------------------------------------------------------------------

    /// <summary>Where the person zoomed and panned to; none while it shows whole.</summary>
    CanvasViewport? panned;

    /// <summary>What it shows: the canvas whole, fitted and centered, until the person zooms or pans.</summary>
    public CanvasViewport Viewport => Zoom && panned is { } v ? v : Whole(UnitsWidth, UnitsHeight, Bounds.Width, Bounds.Height);

    /// <summary>The whole canvas, fitted in proportion and centered (conformance/drawing.json, views).</summary>
    public static CanvasViewport Whole(double unitsWidth, double unitsHeight, double width, double height)
    {
        var scale = Math.Min(width / unitsWidth, height / unitsHeight);
        if (!(scale > 0) || !double.IsFinite(scale))
            return new(1, 0, 0);
        return new(scale, (unitsWidth - width / scale) / 2, (unitsHeight - height / scale) / 2);
    }

    /// <summary>
    /// The view after zooming by <paramref name="factor"/> (above 1 closer),
    /// the unit under the pixel <paramref name="atX"/>, <paramref name="atY"/>
    /// staying where it is; from a quarter (or the whole fit, when less) up to
    /// 3 (or 4 times the whole fit, when more).
    /// </summary>
    public static CanvasViewport Zoomed(double unitsWidth, double unitsHeight, double width, double height, CanvasViewport view, double factor, double atX, double atY)
    {
        var fit = Math.Min(width / unitsWidth, height / unitsHeight);
        var scale = Math.Min(Math.Max(view.Scale * factor, Math.Min(0.25, fit)), Math.Max(3, fit * 4));
        var ux = view.X + atX / view.Scale;
        var uy = view.Y + atY / view.Scale;
        return new(scale, ux - atX / scale, uy - atY / scale);
    }

    /// <summary>The view moved by <paramref name="byX"/>, <paramref name="byY"/> pixels, past the canvas's edges too.</summary>
    public static CanvasViewport Scrolled(CanvasViewport view, double byX, double byY) => view with { X = view.X + byX / view.Scale, Y = view.Y + byY / view.Scale };

    /// <summary>The unit under a pixel of the view, rounded down; null outside the canvas's units.</summary>
    public static Pointer? PointIn(double unitsWidth, double unitsHeight, CanvasViewport view, double atX, double atY)
    {
        var x = view.X + atX / view.Scale;
        var y = view.Y + atY / view.Scale;
        if (!(x >= 0 && y >= 0 && x < unitsWidth && y < unitsHeight))
            return null;
        return new Pointer((long)Math.Floor(x), (long)Math.Floor(y));
    }

    /// <summary>
    /// Where in a drawing of its units a click at a pixel of a part
    /// <paramref name="width"/> by <paramref name="height"/> is, the drawing
    /// fitted whole and centered; rounded down. Null beside the drawing.
    /// </summary>
    public static Pointer? PointOf(double unitsWidth, double unitsHeight, double width, double height, double atX, double atY)
    {
        var scale = Math.Min(width / unitsWidth, height / unitsHeight);
        if (!(scale > 0) || !double.IsFinite(scale))
            return null;
        var x = (atX - (width - unitsWidth * scale) / 2) / scale;
        var y = (atY - (height - unitsHeight * scale) / 2) / scale;
        if (x < 0 || y < 0 || x >= unitsWidth || y >= unitsHeight)
            return null;
        return new Pointer((long)Math.Floor(x), (long)Math.Floor(y));
    }

    /// <summary>Where a pointer that is down is, as <see cref="PointOf"/> finds it, but beside the drawing at its nearest edge; null when the part shows nothing.</summary>
    public static Pointer? PointNear(double unitsWidth, double unitsHeight, double width, double height, double atX, double atY)
    {
        var scale = Math.Min(width / unitsWidth, height / unitsHeight);
        if (!(scale > 0) || !double.IsFinite(scale))
            return null;
        var x = (atX - (width - unitsWidth * scale) / 2) / scale;
        var y = (atY - (height - unitsHeight * scale) / 2) / scale;
        return new Pointer(Edge(x, unitsWidth), Edge(y, unitsHeight));
    }

    static long Edge(double v, double size) => (long)Math.Min(Math.Max(Math.Floor(v), 0), Math.Ceiling(size) - 1);

    /// <summary>The units between a grid's shown dots: every <paramref name="grid"/>, doubled until they are 8 pixels apart (conformance/drawing.json, grids).</summary>
    public static double GridEvery(double grid, double scale)
    {
        var every = grid;
        while (every * scale < 8 && every < 1e6)
            every *= 2;
        return every;
    }

    /// <summary>How far left of its x a text starts: half its width centered (the default), none from its left end, all of it from its right.</summary>
    public static double TextLeft(string align, double width) => align switch
    {
        "left" => 0,
        "right" => width,
        _ => width / 2,
    };

    /// <summary>Whether two drawings show the same: texts by what they show, lists by their numbers.</summary>
    public static bool SameDrawing(IReadOnlyList<Shape> a, IReadOnlyList<Shape> b) =>
        a.Count == b.Count && a.Zip(b).All(p => p switch
        {
            (Shape.Words x, Shape.Words y) => (x.X, x.Y, x.Color, x.Size, x.Align, Locale.Show(x.Text)) == (y.X, y.Y, y.Color, y.Size, y.Align, Locale.Show(y.Text)),
            (Shape.Path x, Shape.Path y) => (x.Color, x.Width) == (y.Color, y.Width) && x.Points.SequenceEqual(y.Points),
            (Shape.Link x, Shape.Link y) => (x.Name, x.X1, x.Y1, x.X2, x.Y2, x.Color, x.Width) == (y.Name, y.X1, y.Y1, y.X2, y.Y2, y.Color, y.Width) && x.Via.SequenceEqual(y.Via),
            var (x, y) => Equals(x, y),
        });

    /// <summary>The unit under a pixel of the part (from its top left), not rounded.</summary>
    (double X, double Y) UnitAt(double x, double y)
    {
        var v = Viewport;
        return (v.X + x / v.Scale, v.Y + y / v.Scale);
    }

    /// <summary>The unit under a pointer that is down, at the canvas's nearest edge beside it.</summary>
    Pointer Held(double x, double y)
    {
        var (ux, uy) = UnitAt(x, y);
        return new Pointer(Edge(ux, UnitsWidth), Edge(uy, UnitsHeight));
    }

    void ZoomBy(double factor, double? atX = null, double? atY = null)
    {
        panned = Zoomed(UnitsWidth, UnitsHeight, Bounds.Width, Bounds.Height, Viewport, factor, atX ?? Bounds.Width / 2, atY ?? Bounds.Height / 2);
        Invalidate();
    }

    /// <summary>The zoom buttons in its top right corner, on the screen, as the designer's: out, back to whole, in.</summary>
    public (Box Out, Box Whole, Box In) ZoomButtons()
    {
        var top = Bounds.Y + 6;
        var right = Bounds.Right - 6;
        var zoomIn = new Box(right - 26, top, 26, 24);
        var whole = new Box(zoomIn.X - 2 - 52, top, 52, 24);
        var zoomOut = new Box(whole.X - 2 - 26, top, 26, 24);
        return (zoomOut, whole, zoomIn);
    }

    /// <summary>Which zoom button a point of the screen is on; none off them, or on a canvas that does not zoom.</summary>
    string? ZoomButtonAt(double x, double y)
    {
        if (!Zoom)
            return null;
        var (zoomOut, whole, zoomIn) = ZoomButtons();
        return zoomOut.Contains(x, y) ? "zoomOut" : whole.Contains(x, y) ? "zoomWhole" : zoomIn.Contains(x, y) ? "zoomIn" : null;
    }

    /// <summary>Clarity's words for the zoom buttons, in the person's language.</summary>
    public static string ZoomWord(string button) => button switch
    {
        "zoomOut" => Locale.Translate("zoomOut", "Zoom out"),
        "zoomWhole" => Locale.Translate("zoomWhole", "Show it whole"),
        _ => Locale.Translate("zoomIn", "Zoom in"),
    };

    public override bool OnWheel(double dx, double dy)
    {
        if (!Zoom)
            return false;
        panned = Scrolled(Viewport, dx, dy);
        Invalidate();
        return true;
    }

    /// <summary>Where the pointer was last over it, from its top left; the wheel with Ctrl zooms there.</summary>
    (double X, double Y)? pointerAt;

    /// <summary>The wheel with Ctrl zooms, closer as it turns up, at the pointer; without, it pans.</summary>
    public override bool OnWheel(double dx, double dy, Modifiers held)
    {
        if (!Zoom || !held.HasFlag(Modifiers.Control) || dy == 0)
            return OnWheel(dx, dy);
        ZoomBy(dy < 0 ? 1.2 : 1 / 1.2, pointerAt?.X, pointerAt?.Y);
        Invalidate();
        return true;
    }

    /// <summary>A zoom button says what it does when the pointer rests on it.</summary>
    public override string? TooltipAt(double x, double y) => ZoomButtonAt(x, y) is { } button ? ZoomWord(button) : null;

    // -- Links that bend -----------------------------------------------------------

    /// <summary>A link being bent: its points as the pointer has them, until the drawing comes back with them.</summary>
    (string Name, List<(double X, double Y)> Via)? bending;

    /// <summary>The link last pressed, whose bend points show.</summary>
    string? inHand;

    /// <summary>A press on a link: which, its bend points, the one in hand, where it started, where the pointer was and whether it moved.</summary>
    (string Name, List<(double X, double Y)> Via, int Point, (double X, double Y) Start, double AtX, double AtY, bool Moved)? bendPress;

    /// <summary>The drawing as it shows: an object dragged moved, a link being bent with the points the pointer gives it.</summary>
    public IEnumerable<Shape> Shown() => ModeShapes(Drawing).Select(shape => bending is { } b && shape is Shape.Link link && link.Name == b.Name
        ? link with { Via = Whole(b.Via) }
        : shape);

    static long[] Whole(List<(double X, double Y)> points) => [.. points.SelectMany(p => new[] { (long)Math.Round(p.X), (long)Math.Round(p.Y) })];

    /// <summary>The points a link goes through: its start, its bend points, its end.</summary>
    public static List<(double X, double Y)> LinkPoints(Shape.Link link)
    {
        var all = new List<(double X, double Y)> { (link.X1, link.Y1) };
        for (var i = 0; i + 1 < link.Via.Count; i += 2)
            all.Add((link.Via[i], link.Via[i + 1]));
        all.Add((link.X2, link.Y2));
        return all;
    }

    static double Distance((double X, double Y) a, (double X, double Y) b) => Math.Sqrt(Math.Pow(a.X - b.X, 2) + Math.Pow(a.Y - b.Y, 2));

    static double DistanceToSegment((double X, double Y) p, (double X, double Y) a, (double X, double Y) b)
    {
        var (dx, dy) = (b.X - a.X, b.Y - a.Y);
        var length = dx * dx + dy * dy;
        var t = length == 0 ? 0 : Math.Clamp(((p.X - a.X) * dx + (p.Y - a.Y) * dy) / length, 0, 1);
        return Distance(p, (a.X + t * dx, a.Y + t * dy));
    }

    /// <summary>The link under a unit, the topmost first: its bend point there, else a new one on the segment pressed.</summary>
    (string Name, List<(double X, double Y)> Via, int Point)? LinkAt((double X, double Y) p)
    {
        var near = 8 / Viewport.Scale;
        foreach (var shape in Shown().Reverse())
        {
            // A box drawn over the press takes it: the link under it is not pressed.
            if (shape is Shape.Rect r && p.X >= r.X && p.X <= r.X + r.Width && p.Y >= r.Y && p.Y <= r.Y + r.Height)
                return null;
            if (shape is not Shape.Link link)
                continue;
            var all = LinkPoints(link);
            var via = all.Skip(1).Take(all.Count - 2).ToList();
            var onPoint = via.FindIndex(v => Distance(v, p) <= near);
            if (onPoint >= 0)
                return (link.Name, via, onPoint);
            for (var i = 0; i < all.Count - 1; i++)
                if (DistanceToSegment(p, all[i], all[i + 1]) <= near)
                {
                    via.Insert(i, p);
                    return (link.Name, via, i);
                }
        }
        return null;
    }

    /// <summary>The bend points of the link in hand.</summary>
    List<(double X, double Y)> Handles()
    {
        if (!Handles("bend") || inHand is null || Shown().OfType<Shape.Link>().FirstOrDefault(l => l.Name == inHand) is not { } link)
            return [];
        var all = LinkPoints(link);
        return all.Skip(1).Take(all.Count - 2).ToList();
    }

    void Bent(string name, List<(double X, double Y)> via)
    {
        bending = (name, via);
        seen = Drawing;
        Invalidate();
        Point("bend", new Pointer(0, 0) { Bent = name, Via = Whole(via) });
    }

    /// <summary>A double click on a bend point of the link in hand takes it away.</summary>
    bool Unbend(double x, double y)
    {
        if (!Handles("bend") || inHand is not { } name)
            return false;
        var p = UnitAt(x, y);
        var near = 8 / Viewport.Scale;
        var handles = Handles();
        var point = handles.FindIndex(v => Distance(v, p) <= near);
        if (point < 0)
            return false;
        handles.RemoveAt(point);
        Bent(name, handles);
        return true;
    }

    // -- The pointer ---------------------------------------------------------------

    /// <summary>Runs its handler of <paramref name="event"/> with where it was: through the window's command, else with the pointer set in its state.</summary>
    bool Point(string @event, Pointer at)
    {
        if (Spec.Name is { } name && Host.Pointed(name, @event, at) is { } ran)
            return ran;
        if (State is { } state)
            state.Pointer = at;
        return Fire(@event);
    }

    bool pressed;
    double pressX, pressY;
    Pointer? from;
    CanvasViewport viewAt;
    bool moved;
    // Whether the drag moves the view, and whether its first run said so yet.
    bool movesView;
    bool decided;

    /// <summary>
    /// Whether it takes the focus: a vector canvas for its keys, and a canvas
    /// whose drags, drops and bends are edits with undo, so Ctrl+Z takes back
    /// its own (DESIGN 5.29), as a press on it gives it the focus.
    /// </summary>
    /// <summary>A canvas grows into the height its window has left, on every stack (clarity_spec widgets `grows`).</summary>
    public override bool Grows => true;

    public override bool Focusable => Mode == "vector" ? Acts : KeepsEdits;

    bool KeepsEdits =>
        Enabled && !ReadOnly && Engine is null && Spec.Name is not null && !Spec.Flag("noUndo")
        && (Mode is "objects" or "paint" || Handles("drag") || Handles("drop") || Handles("bend"));

    public override void OnPointerPressed(PointerInput at)
    {
        if (!Enabled || Engine is not null)
            return;
        if (ZoomButtonAt(at.ScreenX, at.ScreenY) is { } button)
        {
            if (button == "zoomWhole")
                panned = null;
            else
                ZoomBy(button == "zoomIn" ? 1.2 : 1 / 1.2);
            Invalidate();
            return;
        }
        if (at.Clicks >= 2 && Unbend(at.X, at.Y))
            return;
        if (ModePressed(at))
            return;
        if (Handles("bend"))
        {
            if (LinkAt(UnitAt(at.X, at.Y)) is { } found)
            {
                inHand = found.Name;
                bendPress = (found.Name, found.Via, found.Point, found.Via[found.Point], at.X, at.Y, false);
                Invalidate();
                return;
            }
            inHand = null;
        }
        pressed = Handles("click") || Handles("drag") || Zoom;
        if (!pressed)
            return;
        (pressX, pressY) = (at.X, at.Y);
        viewAt = Viewport;
        from = PointIn(UnitsWidth, UnitsHeight, viewAt, at.X, at.Y);
        moved = false;
        movesView = !Handles("drag");
        decided = movesView;
    }

    public override void OnPointerMoved(PointerInput at)
    {
        pointerAt = (at.X, at.Y);
        if (!Pressed)
            return;
        if (ModeMoved(at))
            return;
        if (bendPress is { } press)
        {
            if (!press.Moved && Math.Sqrt(Math.Pow(at.X - press.AtX, 2) + Math.Pow(at.Y - press.AtY, 2)) < Still)
                return;
            bendPress = press with { Moved = true };
            var via = new List<(double X, double Y)>(press.Via);
            var scale = Viewport.Scale;
            via[press.Point] = (press.Start.X + (at.X - press.AtX) / scale, press.Start.Y + (at.Y - press.AtY) / scale);
            bending = (press.Name, via);
            Invalidate();
            return;
        }
        if (!pressed)
            return;
        if (!moved && Math.Sqrt(Math.Pow(at.X - pressX, 2) + Math.Pow(at.Y - pressY, 2)) < Still)
            return;
        moved = true;
        if (!movesView && Handles("drag") && from is not null)
        {
            var before = Drawing;
            Point("drag", Held(at.X, at.Y) with { FromX = from.X, FromY = from.Y });
            // The first run says what the drag does: a drawing that stayed the same moves the view.
            if (!decided)
            {
                decided = true;
                movesView = Zoom && SameDrawing(before, Drawing);
            }
        }
        if (movesView && Zoom)
        {
            panned = Scrolled(viewAt, pressX - at.X, pressY - at.Y);
            Invalidate();
        }
    }

    public override void OnPointerReleased(PointerInput at)
    {
        if (ModeReleased(at))
            return;
        if (bendPress is { } bent)
        {
            bendPress = null;
            if (bent.Moved && bending is { } shown)
                Bent(bent.Name, shown.Via);
            return;
        }
        if (!pressed)
            return;
        pressed = false;
        if (moved)
        {
            if (!movesView && Handles("drag") && from is not null && Handles("drop"))
                Point("drop", Held(at.X, at.Y) with { FromX = from.X, FromY = from.Y });
            return;
        }
        if (at.X < 0 || at.Y < 0 || at.X > Bounds.Width || at.Y > Bounds.Height || !Handles("click"))
            return;
        if (PointIn(UnitsWidth, UnitsHeight, Viewport, at.X, at.Y) is { } point)
            Point("click", point);
    }

    // -- Drawing ---------------------------------------------------------------------

    /// <summary>Why the last drawing failed; none when it drew.</summary>
    public string? Failed { get; private set; }

    readonly Dictionary<string, SKBitmap?> pictures = [];

    /// <summary>A picture of the resources folder by its name, loaded once.</summary>
    SKBitmap? Picture(string name)
    {
        if (!pictures.TryGetValue(name, out var picture))
            pictures[name] = picture = ImagePart.Picture(name, Host.Resource(name));
        return picture;
    }

    public override void Arrange(Box box)
    {
        base.Arrange(box);
        Engine?.Arrange(box);
    }

    public override void Draw(Painter painter)
    {
        if (Engine is not null)
        {
            base.Draw(painter);
            return;
        }
        // Fail safe: a drawing that fails says why in its place, and the window works on.
        try
        {
            using (painter.Clip(Bounds))
                DrawCanvas(painter);
            Failed = null;
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Failed = e.Message;
            painter.Text(e.Message, Bounds.Inset(6), painter.Look.Color("error"), painter.Look.Size("small"));
        }
    }

    void DrawCanvas(Painter painter)
    {
        var v = Viewport;
        if (!(v.Scale > 0) || !double.IsFinite(v.Scale))
            return;
        var shapes = Shown().ToList();
        Paint(painter, Bounds, shapes, v, UnitsWidth, UnitsHeight, Paper, Zoom, Picture, Grid > 0 ? (on, _) => DrawGrid(on, v) : null);
        DrawMode(painter, v);
        // The bend points of the link in hand: drag one to move it, double-click it to take it away.
        var canvas = painter.Canvas;
        using var face = new SKPaint { Color = painter.Look.Color("panel"), IsAntialias = true };
        using var edge = new SKPaint { Color = painter.Look.Color("accent"), IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.5f };
        foreach (var handle in Handles())
        {
            var p = At(v, handle.X, handle.Y);
            canvas.DrawCircle(p, 5, face);
            canvas.DrawCircle(p, 5, edge);
        }
        if (Zoom)
            DrawZoomButtons(painter);
    }

    /// <summary>The pixel on the screen of a unit, in the view <paramref name="v"/>.</summary>
    SKPoint At(CanvasViewport v, double x, double y) => new((float)(Bounds.X + (x - v.X) * v.Scale), (float)(Bounds.Y + (y - v.Y) * v.Scale));

    /// <summary>The grid's dots over the whole view, the same size whatever the zoom, thinned to 8 pixels apart or more.</summary>
    void DrawGrid(Painter painter, CanvasViewport v)
    {
        var every = GridEvery(Grid, v.Scale);
        using var dot = new SKPaint { Color = ColorOf(painter.Look, DotColor != "" ? DotColor : "border"), IsAntialias = true };
        var r = (float)(Math.Max(DotSize, 1) / 2);
        var right = v.X + Bounds.Width / v.Scale;
        var bottom = v.Y + Bounds.Height / v.Scale;
        for (var x = Math.Ceiling(v.X / every) * every; x <= right; x += every)
            for (var y = Math.Ceiling(v.Y / every) * every; y <= bottom; y += every)
                painter.Canvas.DrawCircle(At(v, x, y), r, dot);
    }

    /// <summary>The zoom buttons, as the designer's: out, the zoom it is at (back to whole), in.</summary>
    void DrawZoomButtons(Painter painter)
    {
        var look = painter.Look;
        var (zoomOut, whole, zoomIn) = ZoomButtons();
        foreach (var (box, label) in new[] { (zoomOut, "-"), (whole, $"{Math.Round(Viewport.Scale * 100)}%"), (zoomIn, "+") })
        {
            painter.Fill(box, look.Color("panel"), look.Radius("button"));
            painter.Stroke(box, look.Color("border"), look.Border, look.Radius("button"));
            painter.Text(label, box, look.Color("text"), look.Size("small"), align: TextAlign.Center);
        }
    }

    /// <summary>The color of a shape: written out (#rrggbb), else the theme's by name, its text color for a name it has not.</summary>
    public static SKColor ColorOf(ThemeLook look, string color) =>
        color.StartsWith('#') && SKColor.TryParse(color, out var written) ? written : look.Color(color);

    /// <summary>
    /// Draws <paramref name="shapes"/> of a canvas of its units in
    /// <paramref name="bounds"/>, in the view <paramref name="v"/>: a clear
    /// fills its units (the whole view on a sheet that zooms) with its color,
    /// else <paramref name="paper"/>, else the theme's panel; the grid goes
    /// after the clear, first when there is none.
    /// </summary>
    public static void Paint(Painter painter, Box bounds, IReadOnlyList<Shape> shapes, CanvasViewport v, double unitsWidth, double unitsHeight, string paper, bool sheet, Func<string, SKBitmap?> pictures, Action<Painter, double>? grid = null)
    {
        SKPoint At(double x, double y) => new((float)(bounds.X + (x - v.X) * v.Scale), (float)(bounds.Y + (y - v.Y) * v.Scale));
        var gridFirst = grid is not null && !shapes.OfType<Shape.Clear>().Any();
        if (gridFirst)
            grid!(painter, v.Scale);
        var gridDrawn = grid is null || gridFirst;
        foreach (var shape in shapes)
        {
            if (shape is Shape.Clear clear)
            {
                var color = ColorOf(painter.Look, clear.Color != "" ? clear.Color : paper != "" ? paper : "panel");
                var start = At(0, 0);
                painter.Fill(sheet ? bounds : new Box(start.X, start.Y, unitsWidth * v.Scale, unitsHeight * v.Scale), color);
                if (!gridDrawn)
                {
                    gridDrawn = true;
                    grid!(painter, v.Scale);
                }
                continue;
            }
            DrawShape(painter, shape, v.Scale, At, pictures);
        }
    }

    /// <summary>The font of a canvas's text at a size on the screen: unhinted, so its letters and spaces scale with the zoom and keep their widths.</summary>
    public static SKFont WordsFont(Fonts fonts, double size)
    {
        var font = fonts.Font("text", size, false);
        font.Hinting = SKFontHinting.None;
        font.LinearMetrics = true;
        return font;
    }

    /// <summary>A line through <paramref name="points"/> in <paramref name="paint"/>; one point is a dot of its round cap.</summary>
    static void Polyline(SKCanvas canvas, SKPoint[] points, SKPaint paint) =>
        canvas.DrawPoints(points.Length == 1 ? SKPointMode.Points : SKPointMode.Polygon, points, paint);

    /// <summary>Draws one shape of a drawing, in its units, <paramref name="at"/> giving each unit's pixel.</summary>
    public static void DrawShape(Painter painter, Shape shape, double scale, Func<double, double, SKPoint> at, Func<string, SKBitmap?> pictures)
    {
        var canvas = painter.Canvas;
        var look = painter.Look;
        SKPaint Fill(string color) => new() { Color = ColorOf(look, color), IsAntialias = true, Style = SKPaintStyle.Fill };
        SKPaint Line(string color, double width) => new()
        {
            Color = ColorOf(look, color),
            IsAntialias = true,
            Style = SKPaintStyle.Stroke,
            StrokeWidth = (float)(width * scale),
            StrokeCap = SKStrokeCap.Round,
            StrokeJoin = SKStrokeJoin.Round,
        };
        void Through(IReadOnlyList<(double X, double Y)> points, SKPaint paint)
        {
            if (points.Count > 0)
                Polyline(canvas, [.. points.Select(p => at(p.X, p.Y))], paint);
        }
        switch (shape)
        {
            case Shape.Line line:
                {
                    using var paint = Line(line.Color, line.Width);
                    canvas.DrawLine(at(line.X1, line.Y1), at(line.X2, line.Y2), paint);
                    break;
                }
            case Shape.Rect rect:
                {
                    using var paint = Fill(rect.Color);
                    var p = at(rect.X, rect.Y);
                    canvas.DrawRect(p.X, p.Y, (float)(rect.Width * scale), (float)(rect.Height * scale), paint);
                    break;
                }
            case Shape.Circle circle:
                {
                    using var paint = Fill(circle.Color);
                    canvas.DrawCircle(at(circle.X, circle.Y), (float)(circle.R * scale), paint);
                    break;
                }
            case Shape.Ring ring:
                {
                    using var paint = Line(ring.Color, ring.Width);
                    canvas.DrawCircle(at(ring.X, ring.Y), (float)(ring.R * scale), paint);
                    break;
                }
            case Shape.Ellipse ellipse:
                {
                    using var paint = Fill(ellipse.Color);
                    var c = at(ellipse.X, ellipse.Y);
                    canvas.DrawOval(c, new SKSize((float)(ellipse.Rx * scale), (float)(ellipse.Ry * scale)), paint);
                    break;
                }
            case Shape.Link link:
                {
                    using var paint = Line(link.Color, link.Width);
                    Through(LinkPoints(link), paint);
                    break;
                }
            case Shape.Path path when path.Points.Count >= 2:
                {
                    using var paint = Line(path.Color, path.Width);
                    var points = new List<(double X, double Y)>();
                    for (var i = 0; i + 1 < path.Points.Count; i += 2)
                        points.Add((path.Points[i], path.Points[i + 1]));
                    Through(points, paint);
                    break;
                }
            case Shape.Image image when pictures(image.Picture) is { } picture:
                {
                    var p = at(image.X, image.Y);
                    using var drawn = SKImage.FromBitmap(picture);
                    canvas.DrawImage(drawn, new SKRect(p.X, p.Y, p.X + (float)(image.Width * scale), p.Y + (float)(image.Height * scale)), new SKSamplingOptions(SKFilterMode.Linear, SKMipmapMode.Linear));
                    break;
                }
            case Shape.Words words:
                {
                    var text = Locale.Show(words.Text);
                    var size = words.Size * scale;
                    if (text.Length == 0 || !(size > 0))
                        break;
                    using var font = WordsFont(painter.Fonts, size);
                    using var paint = Fill(words.Color);
                    var width = font.MeasureText(text);
                    var metrics = font.Metrics;
                    var p = at(words.X, words.Y);
                    // Centered on its point down, by its alignment across.
                    var baseline = p.Y - (metrics.Ascent + metrics.Descent) / 2;
                    canvas.DrawText(text, (float)(p.X - TextLeft(words.Align, width)), baseline, SKTextAlign.Left, font, paint);
                    break;
                }
            // An object's box draws nothing itself: the shapes after it show it.
        }
    }
}
