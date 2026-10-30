// A canvas drawn by code: the shapes its draw handler drew, in the canvas's
// own units, scaled to fit the control in proportion and centered, as the
// web renderer draws them (conformance/drawing.json). A color is the
// theme's by name, through its Clarity<Name> brush, or written out. A canvas
// with `on click` runs its Command with where it was clicked, in its units.
// A canvas that `runs` an engine beside the screen (DESIGN 5.12) is drawn
// by its frames while it shows; when the engine stops, it says why. Its
// pointer and keys go to the engine, when it takes them: the canvas keeps
// the pointer while it is down, and takes the keyboard when clicked or
// tabbed to. What handlers tell it through its State (hears) goes to the
// engine; what the engine tells comes out as Told, which runs its handler.
// With Zoom it moves as Clarity's own diagram designer does
// (conformance/drawing.json, views): it starts whole; the wheel pans (with
// Shift sideways), Control or Command with the wheel zooms around the
// pointer, and the buttons in its corner zoom out, back to whole, and in.
// With DragCommand a press that moves runs it as the pointer goes, with
// where it was pressed, and DropCommand when it is let go. A drag that moves
// nothing (the drawing stays the same after its first run), and any drag
// without DragCommand, pans. With Grid it shows a dot every so many units
// over the whole view, after the drawing's clear (conformance/drawing.json, grids).

using System.Globalization;
using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Threading;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public class CanvasView : Control, global::Avalonia.Rendering.ICustomHitTest
{
    /// The pointer is the canvas's anywhere on it, where it drew nothing too.
    public bool HitTest(Point point) => new Rect(Bounds.Size).Contains(point);

    public static readonly StyledProperty<IReadOnlyList<Shape>> DrawingProperty =
        AvaloniaProperty.Register<CanvasView, IReadOnlyList<Shape>>(nameof(Drawing), []);

    public static readonly StyledProperty<double> UnitsWidthProperty =
        AvaloniaProperty.Register<CanvasView, double>(nameof(UnitsWidth), 100);

    public static readonly StyledProperty<double> UnitsHeightProperty =
        AvaloniaProperty.Register<CanvasView, double>(nameof(UnitsHeight), 100);

    public static readonly StyledProperty<ICommand?> CommandProperty =
        AvaloniaProperty.Register<CanvasView, ICommand?>(nameof(Command));

    public static readonly StyledProperty<bool> ZoomProperty =
        AvaloniaProperty.Register<CanvasView, bool>(nameof(Zoom));

    public static readonly StyledProperty<string> PaperProperty =
        AvaloniaProperty.Register<CanvasView, string>(nameof(Paper), "");

    /// <summary>What a clear without a color fills the canvas with: its part's background; empty is the theme's panel.</summary>
    public string Paper
    {
        get => GetValue(PaperProperty);
        set => SetValue(PaperProperty, value);
    }

    public static readonly StyledProperty<double> DotSizeProperty =
        AvaloniaProperty.Register<CanvasView, double>(nameof(DotSize), 2);

    public static readonly StyledProperty<string> DotColorProperty =
        AvaloniaProperty.Register<CanvasView, string>(nameof(DotColor), "");

    /// <summary>How many pixels across the grid's dots are.</summary>
    public double DotSize
    {
        get => GetValue(DotSizeProperty);
        set => SetValue(DotSizeProperty, value);
    }

    /// <summary>The color of the grid's dots; empty is the theme's border.</summary>
    public string DotColor
    {
        get => GetValue(DotColorProperty);
        set => SetValue(DotColorProperty, value);
    }

    public static readonly StyledProperty<double> GridProperty =
        AvaloniaProperty.Register<CanvasView, double>(nameof(Grid));

    /// <summary>The units between the grid's dots; 0 for no grid.</summary>
    public double Grid
    {
        get => GetValue(GridProperty);
        set => SetValue(GridProperty, value);
    }

    /// <summary>The units between a grid's shown dots: every <paramref name="grid"/>, doubled until
    /// they are 8 pixels apart (conformance/drawing.json, grids).</summary>
    public static double GridEvery(double grid, double scale)
    {
        var every = grid;
        while (every * scale < 8 && every < 1e6)
            every *= 2;
        return every;
    }

    public static readonly StyledProperty<ICommand?> DragCommandProperty =
        AvaloniaProperty.Register<CanvasView, ICommand?>(nameof(DragCommand));

    public static readonly StyledProperty<ICommand?> DropCommandProperty =
        AvaloniaProperty.Register<CanvasView, ICommand?>(nameof(DropCommand));

    /// <summary>Whether the person zooms and scrolls the drawing.</summary>
    public bool Zoom
    {
        get => GetValue(ZoomProperty);
        set => SetValue(ZoomProperty, value);
    }

    /// <summary>What a drag runs as the pointer moves, given where it is and where it was pressed.</summary>
    public ICommand? DragCommand
    {
        get => GetValue(DragCommandProperty);
        set => SetValue(DragCommandProperty, value);
    }

    /// <summary>What runs when the pointer is let go after a drag.</summary>
    public ICommand? DropCommand
    {
        get => GetValue(DropCommandProperty);
        set => SetValue(DropCommandProperty, value);
    }

    public static readonly StyledProperty<Func<IFrames>?> RunsProperty =
        AvaloniaProperty.Register<CanvasView, Func<IFrames>?>(nameof(Runs));

    public static readonly StyledProperty<string?> StoppedProperty =
        AvaloniaProperty.Register<CanvasView, string?>(nameof(Stopped));

    public static readonly StyledProperty<PartState?> StateProperty =
        AvaloniaProperty.Register<CanvasView, PartState?>(nameof(State));

    /// <summary>The part's state, through which handlers tell its engine something (<c>sky.start(2)</c>).</summary>
    public PartState? State
    {
        get => GetValue(StateProperty);
        set => SetValue(StateProperty, value);
    }

    /// <summary>Its engine told the window something: the tell and its values.</summary>
    public event Action<string, System.Text.Json.JsonElement[]>? Told;

    /// <summary>The engine beside the screen that draws it: one instance from when it shows until it goes.</summary>
    public Func<IFrames>? Runs
    {
        get => GetValue(RunsProperty);
        set => SetValue(RunsProperty, value);
    }

    /// <summary>Why its engine stopped, which it shows in place of its drawing; none while it runs.</summary>
    public string? Stopped
    {
        get => GetValue(StoppedProperty);
        set => SetValue(StoppedProperty, value);
    }

    FrameLoop? loop;

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        if (Runs is not { } make || TopLevel.GetTopLevel(this) is not { } top)
            return;
        Stopped = null;
        try
        {
            loop = new FrameLoop(make(), f => top.RequestAnimationFrame(f), work => Task.Run(work), ui => Dispatcher.UIThread.Post(ui), shapes => Drawing = shapes, why => Stopped = why, tell => Told?.Invoke(tell.Tell, tell.Values));
            Focusable = loop.Hears.Keys;
            loop.Start();
        }
        catch (EngineException failed)
        {
            Stopped = failed.Message;
        }
        // What handlers told before it showed comes first.
        State?.Listen((_, _) => { }, (hear, values) => loop?.Hear(hear, values));
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnDetachedFromVisualTree(e);
        if (Runs is not null)
            State?.StopListening();
        loop?.Dispose();
        loop = null;
        held.All();
        engineDown = null;
    }

    /// The pointer that is down on the drawing, which the canvas keeps for its engine.
    IPointer? engineDown;
    readonly HeldKeys held = new();

    bool Pointing => loop is { Hears.Pointer: true } && Stopped is null;

    /// Where `at` is for the engine: on the drawing, or at its nearest edge while down.
    Pointer? EnginePoint(Point at, bool near)
    {
        var units = new Size(UnitsWidth, UnitsHeight);
        return near ? PointNear(units, Bounds.Size, at) : at.X < 0 || at.Y < 0 || at.X > Bounds.Width || at.Y > Bounds.Height ? null : PointOf(units, Bounds.Size, at);
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);
        // A zoom button says what it does, in Clarity's words.
        var button = ZoomButtonAt(e.GetPosition(this));
        ToolTip.SetTip(this, button is null ? null : ZoomWord(button));
        if (pressed)
        {
            Dragged(e);
            return;
        }
        if (!Pointing || (engineDown is not null && engineDown != e.Pointer))
            return;
        if (EnginePoint(e.GetPosition(this), engineDown is not null) is { } point)
            loop!.Pointer("move", point.X, point.Y);
    }

    protected override void OnPointerCaptureLost(PointerCaptureLostEventArgs e)
    {
        base.OnPointerCaptureLost(e);
        engineDown = null;
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        // A shortcut with Control, Alt or the command key is the window's.
        if (loop is not { Hears.Keys: true } || ScreenKeys.NameOf(e.Key) is not { } name)
            return;
        if ((e.KeyModifiers & (KeyModifiers.Control | KeyModifiers.Alt | KeyModifiers.Meta)) != 0 && name is not ("Control" or "Alt" or "Shift"))
            return;
        e.Handled = true;
        if (held.Down(name))
            loop.Key("down", name);
    }

    protected override void OnKeyUp(KeyEventArgs e)
    {
        base.OnKeyUp(e);
        if (ScreenKeys.NameOf(e.Key) is { } name && held.Up(name))
            loop?.Key("up", name);
    }

    protected override void OnLostFocus(FocusChangedEventArgs e)
    {
        base.OnLostFocus(e);
        foreach (var name in held.All())
            loop?.Key("up", name);
    }

    /// What a click runs, given the Pointer of where it was; none: clicks do nothing.
    public ICommand? Command
    {
        get => GetValue(CommandProperty);
        set => SetValue(CommandProperty, value);
    }

    public IReadOnlyList<Shape> Drawing
    {
        get => GetValue(DrawingProperty);
        set => SetValue(DrawingProperty, value);
    }

    /// The width of the canvas in its own units.
    public double UnitsWidth
    {
        get => GetValue(UnitsWidthProperty);
        set => SetValue(UnitsWidthProperty, value);
    }

    public double UnitsHeight
    {
        get => GetValue(UnitsHeightProperty);
        set => SetValue(UnitsHeightProperty, value);
    }

    static CanvasView()
    {
        AffectsRender<CanvasView>(DrawingProperty, UnitsWidthProperty, UnitsHeightProperty, StoppedProperty, GridProperty, PaperProperty, DotSizeProperty, DotColorProperty);
    }

    public CanvasView()
    {
        // The theme's colors may change after a drawing: light or dark, or another theme.
        ActualThemeVariantChanged += (_, _) => InvalidateVisual();
        ResourcesChanged += (_, _) => InvalidateVisual();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == CommandProperty)
            Cursor = Command is null ? null : new Cursor(StandardCursorType.Hand);
        // Only a canvas of other units starts over: a redraw keeps the view.
        if (change.Property == UnitsWidthProperty || change.Property == UnitsHeightProperty)
            panned = null;
    }

    /// Where the person zoomed and panned to; none while the canvas shows whole.
    CanvasViewport? panned;

    /// <summary>What the control shows: the canvas whole, fitted and centered, until the person zooms or pans.</summary>
    public CanvasViewport Viewport => Zoom && panned is { } v ? v : Whole(new Size(UnitsWidth, UnitsHeight), Bounds.Size);

    /// <summary>The whole canvas, fitted in proportion and centered (conformance/drawing.json, views).</summary>
    public static CanvasViewport Whole(Size units, Size part)
    {
        var scale = Math.Min(part.Width / units.Width, part.Height / units.Height);
        if (!(scale > 0) || !double.IsFinite(scale))
            return new(1, 0, 0);
        return new(scale, (units.Width - part.Width / scale) / 2, (units.Height - part.Height / scale) / 2);
    }

    /// <summary>The view after zooming by <paramref name="factor"/> (above 1 closer), the unit under the pixel
    /// <paramref name="at"/> staying where it is; from a quarter (or the whole fit, when less) up to 3 (or 4
    /// times the whole fit, when more).</summary>
    public static CanvasViewport Zoomed(Size units, Size part, CanvasViewport view, double factor, Point at)
    {
        var fit = Math.Min(part.Width / units.Width, part.Height / units.Height);
        var scale = Math.Min(Math.Max(view.Scale * factor, Math.Min(0.25, fit)), Math.Max(3, fit * 4));
        var ux = view.X + at.X / view.Scale;
        var uy = view.Y + at.Y / view.Scale;
        return new(scale, ux - at.X / scale, uy - at.Y / scale);
    }

    /// <summary>The view moved by <paramref name="by"/> pixels, past the canvas's edges too.</summary>
    public static CanvasViewport Scrolled(CanvasViewport view, Vector by) => view with { X = view.X + by.X / view.Scale, Y = view.Y + by.Y / view.Scale };

    /// <summary>The unit under the pixel <paramref name="at"/>, rounded down; null outside the canvas's units.</summary>
    public static Pointer? PointIn(Size units, CanvasViewport view, Point at)
    {
        var x = view.X + at.X / view.Scale;
        var y = view.Y + at.Y / view.Scale;
        if (!(x >= 0 && y >= 0 && x < units.Width && y < units.Height))
            return null;
        return new Pointer((long)Math.Floor(x), (long)Math.Floor(y));
    }

    /// <summary>Whether two drawings show the same: texts by what they show, as records compare their values by reference.</summary>
    public static bool SameDrawing(IReadOnlyList<Shape> a, IReadOnlyList<Shape> b) =>
        a.Count == b.Count && a.Zip(b).All(p => p switch
        {
            (Shape.Words x, Shape.Words y) => (x.X, x.Y, x.Color, x.Size, Locale.Show(x.Text)) == (y.X, y.Y, y.Color, y.Size, Locale.Show(y.Text)),
            var (x, y) => Equals(x, y),
        });

    /// The unit under a pointer that is down, at the canvas's nearest edge beside it.
    Pointer Held(Point at)
    {
        var v = Viewport;
        static long Edge(double u, double end) => (long)Math.Min(Math.Max(Math.Floor(u), 0), Math.Ceiling(end) - 1);
        return new Pointer(Edge(v.X + at.X / v.Scale, UnitsWidth), Edge(v.Y + at.Y / v.Scale, UnitsHeight));
    }

    /// Zooms by <paramref name="factor"/> around a pixel of the control; its middle without one.
    void ZoomBy(double factor, Point? at = null)
    {
        panned = Zoomed(new Size(UnitsWidth, UnitsHeight), Bounds.Size, Viewport, factor, at ?? new Point(Bounds.Width / 2, Bounds.Height / 2));
        InvalidateVisual();
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);
        if (!Zoom || Runs is not null)
            return;
        e.Handled = true;
        if ((e.KeyModifiers & (KeyModifiers.Control | KeyModifiers.Meta)) != 0)
        {
            ZoomBy(e.Delta.Y > 0 ? 1.15 : 1 / 1.15, e.GetPosition(this));
            return;
        }
        // A wheel's notch is 50 pixels, as browsers scroll about.
        var across = (e.KeyModifiers & KeyModifiers.Shift) != 0 ? -e.Delta.Y : -e.Delta.X;
        var down = (e.KeyModifiers & KeyModifiers.Shift) != 0 ? 0 : -e.Delta.Y;
        panned = Scrolled(Viewport, new Vector(across * 50, down * 50));
        InvalidateVisual();
    }

    /// The zoom buttons in the top right corner, as the designer's: out, back to whole, in.
    (Rect Out, Rect Whole, Rect In) ZoomButtons()
    {
        var top = 6.0;
        var right = Bounds.Width - 6;
        var zoomIn = new Rect(right - 26, top, 26, 24);
        var whole = new Rect(zoomIn.X - 2 - 52, top, 52, 24);
        var zoomOut = new Rect(whole.X - 2 - 26, top, 26, 24);
        return (zoomOut, whole, zoomIn);
    }

    /// Which zoom button a pixel is on, if any.
    string? ZoomButtonAt(Point at)
    {
        if (!Zoom || Runs is not null)
            return null;
        var (zoomOut, whole, zoomIn) = ZoomButtons();
        return zoomOut.Contains(at) ? "zoomOut" : whole.Contains(at) ? "zoomWhole" : zoomIn.Contains(at) ? "zoomIn" : null;
    }

    /// Where in a drawing of `units` a click `at` a part of size `part` is,
    /// the drawing fitted as it is drawn, whole and centered; rounded down.
    /// Null beside the drawing, where the part shows none of it.
    public static Pointer? PointOf(Size units, Size part, Point at)
    {
        var scale = Math.Min(part.Width / units.Width, part.Height / units.Height);
        if (!(scale > 0) || !double.IsFinite(scale))
            return null;
        var x = (at.X - (part.Width - units.Width * scale) / 2) / scale;
        var y = (at.Y - (part.Height - units.Height * scale) / 2) / scale;
        if (x < 0 || y < 0 || x >= units.Width || y >= units.Height)
            return null;
        return new Pointer((long)Math.Floor(x), (long)Math.Floor(y));
    }

    /// Where a pointer that is down is, as PointOf finds it, but beside the
    /// drawing at its nearest edge; null when the part shows nothing.
    public static Pointer? PointNear(Size units, Size part, Point at)
    {
        var scale = Math.Min(part.Width / units.Width, part.Height / units.Height);
        if (!(scale > 0) || !double.IsFinite(scale))
            return null;
        var x = (at.X - (part.Width - units.Width * scale) / 2) / scale;
        var y = (at.Y - (part.Height - units.Height * scale) / 2) / scale;
        static long Edge(double v, double size) => (long)Math.Min(Math.Max(Math.Floor(v), 0), Math.Ceiling(size) - 1);
        return new Pointer(Edge(x, units.Width), Edge(y, units.Height));
    }

    // A click is the primary button pressed and let go on the canvas.
    bool pressed;

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);
        if (Pointing)
        {
            if (Focusable)
                Focus(NavigationMethod.Pointer);
            if (engineDown is null && e.GetCurrentPoint(this).Properties.IsLeftButtonPressed && EnginePoint(e.GetPosition(this), false) is { } point)
            {
                engineDown = e.Pointer;
                e.Pointer.Capture(this);
                loop!.Pointer("down", point.X, point.Y);
                e.Handled = true;
            }
            return;
        }
        if (e.GetCurrentPoint(this).Properties.IsLeftButtonPressed && ZoomButtonAt(e.GetPosition(this)) is { } button)
        {
            e.Handled = true;
            if (button == "zoomWhole")
            {
                panned = null;
                InvalidateVisual();
            }
            else
                ZoomBy(button == "zoomIn" ? 1.2 : 1 / 1.2);
            return;
        }
        pressed = (Command is not null || DragCommand is not null || Zoom) && e.GetCurrentPoint(this).Properties.IsLeftButtonPressed;
        if (!pressed)
            return;
        e.Handled = true;
        pressAt = e.GetPosition(this);
        viewAt = Viewport;
        from = PointIn(new Size(UnitsWidth, UnitsHeight), viewAt, pressAt);
        moved = false;
        movesView = DragCommand is null;
        decided = movesView;
        if (DragCommand is not null || Zoom)
            e.Pointer.Capture(this);
    }

    // Where the press was, in pixels and in units, the view then, and whether it moved since.
    Point pressAt;
    Pointer? from;
    CanvasViewport viewAt;
    bool moved;
    // Whether the drag moves the view, and whether its first run said so yet.
    bool movesView;
    bool decided;

    /// A pixel moved this far is a drag, not a click.
    const double Still = 4;

    void Dragged(PointerEventArgs e)
    {
        var at = e.GetPosition(this);
        if (!moved && Math.Sqrt(Math.Pow(at.X - pressAt.X, 2) + Math.Pow(at.Y - pressAt.Y, 2)) < Still)
            return;
        moved = true;
        if (!movesView && DragCommand is not null && from is not null)
        {
            var now = Held(at) with { FromX = from.X, FromY = from.Y };
            var before = Drawing;
            if (DragCommand.CanExecute(now))
                DragCommand.Execute(now);
            // The first run says what the drag does: a drawing that stayed the same moves the view.
            if (!decided)
            {
                decided = true;
                movesView = Zoom && SameDrawing(before, Drawing);
            }
        }
        if (movesView && Zoom)
        {
            // Dragging that moves nothing pans the view with the pointer.
            panned = Scrolled(viewAt, pressAt - at);
            InvalidateVisual();
        }
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);
        if (engineDown is not null && engineDown == e.Pointer)
        {
            engineDown = null;
            if (EnginePoint(e.GetPosition(this), true) is { } released)
                loop?.Pointer("up", released.X, released.Y);
            e.Pointer.Capture(null);
            e.Handled = true;
            return;
        }
        if (!pressed)
            return;
        pressed = false;
        e.Pointer.Capture(null);
        var at = e.GetPosition(this);
        if (moved)
        {
            if (!movesView && DragCommand is not null && from is not null && DropCommand is not null)
            {
                var dropped = Held(at) with { FromX = from.X, FromY = from.Y };
                if (DropCommand.CanExecute(dropped))
                    DropCommand.Execute(dropped);
            }
            e.Handled = true;
            return;
        }
        if (at.X < 0 || at.Y < 0 || at.X > Bounds.Width || at.Y > Bounds.Height)
            return;
        var point = PointIn(new Size(UnitsWidth, UnitsHeight), Viewport, at);
        if (point is not null && Command?.CanExecute(point) == true)
        {
            Command.Execute(point);
            e.Handled = true;
        }
    }

    /// The brush of a color: the theme's by name, written out (#rrggbb), or
    /// the theme's text color for a name the theme does not have.
    public IBrush BrushOf(string color)
    {
        if (color.StartsWith('#') && Color.TryParse(color, out var written))
            return new SolidColorBrush(written);
        var key = "Clarity" + string.Concat(color.Split('_', StringSplitOptions.RemoveEmptyEntries).Select(w => char.ToUpperInvariant(w[0]) + w[1..]));
        if (this.TryFindResource(key, ActualThemeVariant, out var found) && found is IBrush brush)
            return brush;
        if (this.TryFindResource("ClarityText", ActualThemeVariant, out var text) && text is IBrush textBrush)
            return textBrush;
        return Brushes.Black;
    }

    /// Pixels a unit, and where the canvas's units start: kept in
    /// proportion, centered in the control.
    public (double Scale, double Left, double Top) Placement()
    {
        var v = Viewport;
        if (!double.IsFinite(v.Scale) || v.Scale <= 0)
            return (0, 0, 0);
        return (v.Scale, -v.X * v.Scale, -v.Y * v.Scale);
    }

    /// The grid's dots over the whole view, the same size whatever the zoom, thinned to 8 pixels apart or more.
    void DrawGrid(DrawingContext context, double scale, Func<double, double, Point> at)
    {
        if (Grid <= 0)
            return;
        var v = Viewport;
        var every = GridEvery(Grid, scale);
        var dot = BrushOf(DotColor != "" ? DotColor : "border");
        var r = Math.Max(DotSize, 1) / 2;
        var right = v.X + Bounds.Width / scale;
        var bottom = v.Y + Bounds.Height / scale;
        for (var x = Math.Ceiling(v.X / every) * every; x <= right; x += every)
            for (var y = Math.Ceiling(v.Y / every) * every; y <= bottom; y += every)
                context.DrawEllipse(dot, null, at(x, y), r, r);
    }

    public override void Render(DrawingContext context)
    {
        if (Stopped is { } why)
        {
            // In Clarity's words, with the host's message in its slot.
            var said = new Text("The engine stopped: {problem}") { Key = "screenEngineFailed" }.With("problem", Slot.OfText(why));
            var font = this.TryFindResource("ClarityFont", ActualThemeVariant, out var named) && named is FontFamily ff ? ff : FontFamily.Default;
            var shown = new FormattedText(Locale.Show(said), CultureInfo.CurrentUICulture, FlowDirection.LeftToRight, new Typeface(font), 13, BrushOf("muted")) { MaxTextWidth = Math.Max(1, Bounds.Width - 16), TextAlignment = TextAlignment.Center };
            context.DrawText(shown, new Point(8, Math.Max(0, (Bounds.Height - shown.Height) / 2)));
            return;
        }
        var (scale, left, top) = Placement();
        if (scale <= 0)
            return;
        // A zoomed drawing shows only the part of it in view.
        using var clip = context.PushClip(new Rect(Bounds.Size));
        Point At(double x, double y) => new(left + x * scale, top + y * scale);
        // The grid goes after the drawing's clear, so the shapes cover it; first when there is none.
        var gridDrawn = !Drawing.OfType<Shape.Clear>().Any();
        if (gridDrawn)
            DrawGrid(context, scale, At);
        foreach (var shape in Drawing)
        {
            switch (shape)
            {
                case Shape.Clear clear:
                    // A canvas that zooms is a sheet without edges: its clear fills the whole view.
                    var color = clear.Color != "" ? clear.Color : Paper != "" ? Paper : "panel";
                    context.FillRectangle(BrushOf(color), Zoom ? new Rect(Bounds.Size) : new Rect(left, top, UnitsWidth * scale, UnitsHeight * scale));
                    if (!gridDrawn)
                    {
                        gridDrawn = true;
                        DrawGrid(context, scale, At);
                    }
                    break;
                case Shape.Line line:
                    context.DrawLine(new Pen(BrushOf(line.Color), line.Width * scale, lineCap: PenLineCap.Round), At(line.X1, line.Y1), At(line.X2, line.Y2));
                    break;
                case Shape.Rect rect:
                    context.FillRectangle(BrushOf(rect.Color), new Rect(At(rect.X, rect.Y), new Size(rect.Width * scale, rect.Height * scale)));
                    break;
                case Shape.Circle circle:
                    context.DrawEllipse(BrushOf(circle.Color), null, At(circle.X, circle.Y), circle.R * scale, circle.R * scale);
                    break;
                case Shape.Ring ring:
                    context.DrawEllipse(null, new Pen(BrushOf(ring.Color), ring.Width * scale), At(ring.X, ring.Y), ring.R * scale, ring.R * scale);
                    break;
                case Shape.Image image when ResourcePictures.Of(image.Picture) is { } picture:
                    context.DrawImage(picture, new Rect(At(image.X, image.Y), new Size(image.Width * scale, image.Height * scale)));
                    break;
                case Shape.Words words:
                    var font = this.TryFindResource("ClarityFont", ActualThemeVariant, out var family) && family is FontFamily f ? f : FontFamily.Default;
                    var text = new FormattedText(Locale.Show(words.Text), CultureInfo.CurrentUICulture, FlowDirection.LeftToRight, new Typeface(font), words.Size * scale, BrushOf(words.Color));
                    context.DrawText(text, At(words.X, words.Y) - new Point(text.Width / 2, text.Height / 2));
                    break;
            }
        }
        if (Zoom && Runs is null)
            DrawZoomButtons(context);
    }

    /// The zoom buttons, as the designer's: out, the zoom it is at (back to whole), in.
    void DrawZoomButtons(DrawingContext context)
    {
        var (zoomOut, whole, zoomIn) = ZoomButtons();
        var font = this.TryFindResource("ClarityFont", ActualThemeVariant, out var family) && family is FontFamily f ? f : FontFamily.Default;
        var face = BrushOf("panel");
        var edge = new Pen(BrushOf("border"), 1);
        foreach (var (box, label) in new[] { (zoomOut, "-"), (whole, $"{Math.Round(Viewport.Scale * 100)}%"), (zoomIn, "+") })
        {
            context.DrawRectangle(face, edge, new RoundedRect(box, 4));
            var text = new FormattedText(label, CultureInfo.CurrentUICulture, FlowDirection.LeftToRight, new Typeface(font), 12, BrushOf("text"));
            context.DrawText(text, box.Center - new Point(text.Width / 2, text.Height / 2));
        }
    }

    /// Clarity's words for the zoom buttons, in the person's language.
    static string ZoomWord(string key) => key switch
    {
        "zoomOut" => Locale.Translate("zoomOut", "Zoom out"),
        "zoomWhole" => Locale.Translate("zoomWhole", "Show it whole"),
        _ => Locale.Translate("zoomIn", "Zoom in"),
    };
}

/// <summary>What a canvas that zooms shows: the pixels a unit takes, and the unit at its top left corner.</summary>
public readonly record struct CanvasViewport(double Scale, double X, double Y);
