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
// With Zoom the control shows a view of the units (conformance/drawing.json,
// views): the wheel scrolls it (with Shift sideways), Control or Command
// with the wheel zooms around the pointer, and bars say where the view is.
// With DragCommand a press that moves runs it as the pointer goes, with
// where it was pressed, and DropCommand when it is let go; without it,
// dragging moves the view.

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
        AffectsRender<CanvasView>(DrawingProperty, UnitsWidthProperty, UnitsHeightProperty, StoppedProperty);
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
            view = null;
    }

    /// The units shown, once the person zoomed or scrolled; the whole canvas until then.
    Rect? view;

    /// <summary>The rectangle of units the control shows.</summary>
    public Rect View => view ?? new Rect(0, 0, UnitsWidth, UnitsHeight);

    /// A canvas zooms in to a sixteenth of its width at most.
    const double Closest = 16;

    static Rect Within(Size units, double x, double y, double width, double height) =>
        new(Math.Min(Math.Max(x, 0), units.Width - width), Math.Min(Math.Max(y, 0), units.Height - height), width, height);

    /// <summary>The view after zooming by <paramref name="factor"/> (above 1 closer) around
    /// <paramref name="at"/>, in units, which stays where it is (conformance/drawing.json, views).</summary>
    public static Rect Zoomed(Size units, Rect view, double factor, Point at)
    {
        var width = Math.Min(Math.Max(view.Width / factor, units.Width / Closest), units.Width);
        var height = width * units.Height / units.Width;
        var kept = width / view.Width;
        return Within(units, at.X - (at.X - view.X) * kept, at.Y - (at.Y - view.Y) * kept, width, height);
    }

    /// <summary>The view moved by <paramref name="by"/> units, kept inside the canvas.</summary>
    public static Rect Scrolled(Size units, Rect view, Vector by) => Within(units, view.X + by.X, view.Y + by.Y, view.Width, view.Height);

    /// <summary>Where in units a pointer in the part is, through the view it shows, fitted and
    /// centered, rounded down; null beside the drawing. With the whole canvas as its view this is PointOf.</summary>
    public static Pointer? PointIn(Rect view, Size part, Point at)
    {
        var scale = Math.Min(part.Width / view.Width, part.Height / view.Height);
        if (!(scale > 0) || !double.IsFinite(scale))
            return null;
        var x = (at.X - (part.Width - view.Width * scale) / 2) / scale;
        var y = (at.Y - (part.Height - view.Height * scale) / 2) / scale;
        if (x < 0 || y < 0 || x >= view.Width || y >= view.Height)
            return null;
        return new Pointer((long)Math.Floor(view.X + x), (long)Math.Floor(view.Y + y));
    }

    /// Where a pixel is in units, not rounded; beside the drawing too.
    Point Exact(Point at)
    {
        var (scale, left, top) = Placement();
        return scale <= 0 ? default : new Point((at.X - left) / scale, (at.Y - top) / scale);
    }

    /// Where a pointer that is down is, in whole units, at the view's nearest edge beside the drawing.
    Pointer Held(Point at)
    {
        var p = Exact(at);
        var v = View;
        static long Edge(double value, double from, double size) => (long)Math.Min(Math.Max(Math.Floor(value), Math.Floor(from)), Math.Ceiling(from + size) - 1);
        return new Pointer(Edge(p.X, v.X, v.Width), Edge(p.Y, v.Y, v.Height));
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);
        if (!Zoom || Runs is not null)
            return;
        e.Handled = true;
        var units = new Size(UnitsWidth, UnitsHeight);
        if ((e.KeyModifiers & (KeyModifiers.Control | KeyModifiers.Meta)) != 0)
            view = Zoomed(units, View, Math.Exp(e.Delta.Y / 3), Exact(e.GetPosition(this)));
        else
        {
            // A wheel's notch is 50 pixels, as browsers scroll about.
            var (scale, _, _) = Placement();
            if (scale <= 0)
                return;
            var across = (e.KeyModifiers & KeyModifiers.Shift) != 0 ? -e.Delta.Y : -e.Delta.X;
            var down = (e.KeyModifiers & KeyModifiers.Shift) != 0 ? 0 : -e.Delta.Y;
            view = Scrolled(units, View, new Vector(across * 50 / scale, down * 50 / scale));
        }
        InvalidateVisual();
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
        pressed = (Command is not null || DragCommand is not null || Zoom) && e.GetCurrentPoint(this).Properties.IsLeftButtonPressed;
        if (!pressed)
            return;
        e.Handled = true;
        pressAt = e.GetPosition(this);
        from = PointIn(View, Bounds.Size, pressAt);
        viewAt = View;
        moved = false;
        if (DragCommand is not null || Zoom)
            e.Pointer.Capture(this);
    }

    // Where the press was, in pixels and in units, the view then, and whether it moved since.
    Point pressAt;
    Pointer? from;
    Rect viewAt;
    bool moved;

    /// A pixel moved this far is a drag, not a click.
    const double Still = 4;

    void Dragged(PointerEventArgs e)
    {
        var at = e.GetPosition(this);
        if (!moved && Math.Sqrt(Math.Pow(at.X - pressAt.X, 2) + Math.Pow(at.Y - pressAt.Y, 2)) < Still)
            return;
        moved = true;
        if (DragCommand is not null && from is not null)
        {
            var now = Held(at) with { FromX = from.X, FromY = from.Y };
            if (DragCommand.CanExecute(now))
                DragCommand.Execute(now);
        }
        else if (Zoom)
        {
            // Without a drag handler, dragging moves the view with the pointer.
            var (scale, _, _) = Placement();
            if (scale <= 0)
                return;
            view = Scrolled(new Size(UnitsWidth, UnitsHeight), viewAt, (pressAt - at) / scale);
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
            if (DragCommand is not null && from is not null && DropCommand is not null)
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
        var point = PointIn(View, Bounds.Size, at);
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
        var v = View;
        var scale = Math.Min(Bounds.Width / v.Width, Bounds.Height / v.Height);
        if (!double.IsFinite(scale) || scale <= 0)
            return (0, 0, 0);
        return (scale, (Bounds.Width - v.Width * scale) / 2 - v.X * scale, (Bounds.Height - v.Height * scale) / 2 - v.Y * scale);
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
        foreach (var shape in Drawing)
        {
            switch (shape)
            {
                case Shape.Clear clear:
                    context.FillRectangle(BrushOf(clear.Color), new Rect(left, top, UnitsWidth * scale, UnitsHeight * scale));
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
        // Where the view is in the whole canvas, while zoomed in.
        var v = View;
        if (Zoom && (v.Width < UnitsWidth || v.Height < UnitsHeight))
        {
            var bar = BrushOf("muted");
            var across = Bounds.Width - 12;
            var down = Bounds.Height - 12;
            context.DrawRectangle(bar, null, new RoundedRect(new Rect(2 + across * v.X / UnitsWidth, Bounds.Height - 8, across * v.Width / UnitsWidth, 6), 3));
            context.DrawRectangle(bar, null, new RoundedRect(new Rect(Bounds.Width - 8, 2 + down * v.Y / UnitsHeight, 6, down * v.Height / UnitsHeight), 3));
        }
    }
}
