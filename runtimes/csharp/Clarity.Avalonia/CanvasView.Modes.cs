// A canvas with a mode (DESIGN 5.31): what the person does on it. The
// screen shows the gesture and sends what was done as one action
// (ActionCommand, a JSON object); the runtime decides what it does, snaps
// it and keeps it (WindowViewModel.CanvasEvent), and the canvas shows the
// data that comes back.
//   objects: an `object` shape of the drawing marks a record's box. A press
//     in it and a drag moves the box and the shapes drawn after it, until the
//     next object, as a preview; the drop sends {"move": {id, x, y}}.
//   paint: strokes of the brush, Brush pixels wide in Ink, on the picture of
//     the bound image field; a stroke's end sends {"stroke": {"data": PNG}}.
//   vector: the shapes of the bound text field's SVG, with a toolbar of the
//     mode's tools. Drawing a shape adds it, select and drag moves the chosen
//     one, Delete deletes it.

using System.Globalization;
using System.Text.Json.Nodes;
using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public partial class CanvasView
{
    public static readonly StyledProperty<string> ModeProperty = AvaloniaProperty.Register<CanvasView, string>(nameof(Mode), "");
    public static readonly StyledProperty<ICommand?> ActionCommandProperty = AvaloniaProperty.Register<CanvasView, ICommand?>(nameof(ActionCommand));
    public static readonly StyledProperty<string> ToolsProperty = AvaloniaProperty.Register<CanvasView, string>(nameof(Tools), "");
    public static readonly StyledProperty<string> ToolProperty = AvaloniaProperty.Register<CanvasView, string>(nameof(Tool), "", defaultBindingMode: BindingMode.TwoWay);
    public static readonly StyledProperty<double> BrushProperty = AvaloniaProperty.Register<CanvasView, double>(nameof(Brush), 2);
    public static readonly StyledProperty<string> InkProperty = AvaloniaProperty.Register<CanvasView, string>(nameof(Ink), "text");
    public static readonly StyledProperty<Picture?> PictureProperty = AvaloniaProperty.Register<CanvasView, Picture?>(nameof(Picture));
    public static readonly StyledProperty<Func<Task<byte[]?>>?> FetchProperty = AvaloniaProperty.Register<CanvasView, Func<Task<byte[]?>>?>(nameof(Fetch));
    public static readonly StyledProperty<string?> SvgProperty = AvaloniaProperty.Register<CanvasView, string?>(nameof(Svg));
    public static readonly StyledProperty<object?> ChosenProperty = AvaloniaProperty.Register<CanvasView, object?>(nameof(Chosen), defaultBindingMode: BindingMode.TwoWay);
    public static readonly StyledProperty<bool> IsReadOnlyProperty = AvaloniaProperty.Register<CanvasView, bool>(nameof(IsReadOnly));

    /// <summary>What the person does on the canvas: objects, paint or vector; empty: its draw handler draws it, as before.</summary>
    public string Mode { get => GetValue(ModeProperty); set => SetValue(ModeProperty, value); }

    /// <summary>What runs with the person's action, a JSON object (DESIGN 5.31).</summary>
    public ICommand? ActionCommand { get => GetValue(ActionCommandProperty); set => SetValue(ActionCommandProperty, value); }

    /// <summary>The tools of the mode the person may use, by name, comma separated; empty: all of them.</summary>
    public string Tools { get => GetValue(ToolsProperty); set => SetValue(ToolsProperty, value); }

    /// <summary>The tool in hand; empty or one the canvas has not: its first.</summary>
    public string Tool { get => GetValue(ToolProperty); set => SetValue(ToolProperty, value); }

    /// <summary>How wide the canvas paints or draws, in pixels of the picture (its units).</summary>
    public double Brush { get => GetValue(BrushProperty); set => SetValue(BrushProperty, value); }

    /// <summary>The color it paints or draws with: the theme's by name, or written out.</summary>
    public string Ink { get => GetValue(InkProperty); set => SetValue(InkProperty, value); }

    /// <summary>The picture of the bound image field (paint).</summary>
    public Picture? Picture { get => GetValue(PictureProperty); set => SetValue(PictureProperty, value); }

    /// <summary>Gives the stored picture of the bound field, fetched when it shows.</summary>
    public Func<Task<byte[]?>>? Fetch { get => GetValue(FetchProperty); set => SetValue(FetchProperty, value); }

    /// <summary>The drawing of the bound text field, as SVG (vector).</summary>
    public string? Svg { get => GetValue(SvgProperty); set => SetValue(SvgProperty, value); }

    /// <summary>The shape chosen, its place in the drawing from 0 (a long), as the runtime chose it or the person did; null for none.</summary>
    public object? Chosen { get => GetValue(ChosenProperty); set => SetValue(ChosenProperty, value); }

    /// <summary>A canvas the person only looks at: no action.</summary>
    public bool IsReadOnly { get => GetValue(IsReadOnlyProperty); set => SetValue(IsReadOnlyProperty, value); }

    /// <summary>The tools of each mode, in the order its toolbar shows them.</summary>
    public static readonly IReadOnlyDictionary<string, string[]> ModeTools = new Dictionary<string, string[]>
    {
        ["paint"] = ["pen", "eraser"],
        ["vector"] = ["select", "line", "rect", "ellipse", "path", "text"],
    };

    /// <summary>Clarity's words for the tools, by key, in the person's language.</summary>
    static readonly Dictionary<string, (string Key, string Written)> ToolWords = new()
    {
        ["select"] = ("toolSelect", "Select"),
        ["line"] = ("toolLine", "Line"),
        ["rect"] = ("toolRect", "Rectangle"),
        ["ellipse"] = ("toolEllipse", "Ellipse"),
        ["path"] = ("toolPath", "Path"),
        ["text"] = ("toolText", "Text"),
        ["pen"] = ("toolPen", "Pen"),
        ["eraser"] = ("toolEraser", "Eraser"),
    };

    /// <summary>A tool's name in the person's language.</summary>
    public static string ToolWord(string tool) => ToolWords.TryGetValue(tool, out var w) ? Locale.Translate(w.Key, w.Written) : tool;

    /// <summary>The tools the toolbar shows: the canvas's, of its mode, in the mode's order.</summary>
    public IReadOnlyList<string> ToolsShown()
    {
        if (!ModeTools.TryGetValue(Mode, out var all))
            return [];
        var named = Tools.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        return named.Length == 0 ? all : [.. all.Where(named.Contains)];
    }

    /// <summary>The tool in hand: the one set, when the canvas has it, else its first.</summary>
    public string ToolInHand => ToolsShown() is { Count: > 0 } tools ? tools.Contains(Tool) ? Tool : tools[0] : "";

    StackPanel? toolbar;
    TextBox? typing;

    void ModeChanged(AvaloniaPropertyChangedEventArgs change)
    {
        if (change.Property == ModeProperty || change.Property == ToolsProperty)
            BuildToolbar();
        if (change.Property == ToolProperty)
            MarkTool();
        if (change.Property == PictureProperty || change.Property == FetchProperty)
            LoadPicture();
        if (change.Property == SvgProperty || change.Property == UnitsWidthProperty || change.Property == UnitsHeightProperty)
            vector = null;
        // A drawing drawn anew (after any handler, a timer's tick too) keeps a
        // drag going, on the same record's object in the new drawing.
        if (change.Property == DrawingProperty && dragging is { } held)
        {
            var box = SameObject(held.Box);
            shownObjects = ObjectRecords();
            dragging = box >= 0 ? held with { Box = box } : null;
        }
        if (change.Property == IsReadOnlyProperty && toolbar is not null)
            toolbar.IsEnabled = !IsReadOnly;
        if (change.Property == SvgProperty || change.Property == ChosenProperty || change.Property == InkProperty || change.Property == BrushProperty || change.Property == PictureProperty)
            InvalidateVisual();
    }

    void BuildToolbar()
    {
        if (toolbar is not null)
        {
            VisualChildren.Remove(toolbar);
            LogicalChildren.Remove(toolbar);
            toolbar = null;
        }
        var tools = ToolsShown();
        if (tools.Count == 0)
            return;
        toolbar = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 2, Margin = new Thickness(6), HorizontalAlignment = HorizontalAlignment.Left, VerticalAlignment = VerticalAlignment.Top, IsEnabled = !IsReadOnly };
        foreach (var tool in tools)
        {
            var button = new ToggleButton { Content = ToolWord(tool), Tag = tool, Padding = new Thickness(8, 2), Focusable = false };
            button.Click += (_, _) => Tool = tool;
            toolbar.Children.Add(button);
        }
        LogicalChildren.Add(toolbar);
        VisualChildren.Add(toolbar);
        MarkTool();
        InvalidateMeasure();
    }

    void MarkTool()
    {
        foreach (var button in toolbar?.Children.OfType<ToggleButton>() ?? [])
            button.IsChecked = (string?)button.Tag == ToolInHand;
    }

    /// <summary>The toolbar's buttons, for the person and for tests.</summary>
    public IReadOnlyList<ToggleButton> ToolButtons => [.. toolbar?.Children.OfType<ToggleButton>() ?? []];

    protected override Size MeasureOverride(Size availableSize)
    {
        toolbar?.Measure(Size.Infinity);
        typing?.Measure(Size.Infinity);
        return base.MeasureOverride(availableSize);
    }

    protected override Size ArrangeOverride(Size finalSize)
    {
        toolbar?.Arrange(new Rect(toolbar.DesiredSize));
        if (typing is not null && typingAt is { } unit)
        {
            var (scale, left, top) = Placement();
            typing.Arrange(new Rect(new Point(left + unit.X * scale, top + unit.Y * scale), new Size(Math.Max(160, typing.DesiredSize.Width), typing.DesiredSize.Height)));
        }
        return finalSize;
    }

    void Act(JsonObject action)
    {
        if (!IsReadOnly && ActionCommand?.CanExecute(action) == true)
            ActionCommand.Execute(action);
    }

    // -- objects ------------------------------------------------------------

    /// An object being dragged: the place of its box in the drawing, where it was pressed, how far it moved (units).
    (IPointer Pointer, int Box, Point At, Point From, Vector By, bool Moved)? dragging;

    /// <summary>A shape moved by <paramref name="dx"/>, <paramref name="dy"/> units.</summary>
    public static Shape Translated(Shape shape, double dx, double dy) => shape switch
    {
        Shape.Line s => s with { X1 = s.X1 + dx, Y1 = s.Y1 + dy, X2 = s.X2 + dx, Y2 = s.Y2 + dy },
        Shape.Rect s => s with { X = s.X + dx, Y = s.Y + dy },
        Shape.Circle s => s with { X = s.X + dx, Y = s.Y + dy },
        Shape.Ring s => s with { X = s.X + dx, Y = s.Y + dy },
        Shape.Words s => s with { X = s.X + dx, Y = s.Y + dy },
        Shape.Image s => s with { X = s.X + dx, Y = s.Y + dy },
        Shape.Object s => s with { X = s.X + dx, Y = s.Y + dy },
        Shape.Ellipse s => s with { X = s.X + dx, Y = s.Y + dy },
        Shape.Path s => s with { Points = [.. s.Points.Select((p, i) => p + (long)Math.Round(i % 2 == 0 ? dx : dy))] },
        Shape.Link s => s with { X1 = s.X1 + dx, Y1 = s.Y1 + dy, X2 = s.X2 + dx, Y2 = s.Y2 + dy, Via = [.. s.Via.Select((p, i) => p + (long)Math.Round(i % 2 == 0 ? dx : dy))] },
        _ => shape,
    };

    /// <summary>The drawing with the object at <paramref name="box"/> and what is drawn after it, until the next object, moved.</summary>
    public static IEnumerable<Shape> WithObjectMoved(IReadOnlyList<Shape> shapes, int box, Vector by)
    {
        var moving = false;
        for (var i = 0; i < shapes.Count; i++)
        {
            if (shapes[i] is Shape.Object)
                moving = i == box;
            yield return moving ? Translated(shapes[i], by.X, by.Y) : shapes[i];
        }
    }

    /// The place in the drawing now of the object that was at <paramref name="box"/> in the one before; -1 when it is gone.
    int SameObject(int box)
    {
        var record = box < shownObjects.Count ? shownObjects[box] : null;
        for (var i = 0; i < Drawing.Count; i++)
            if (Drawing[i] is Shape.Object o && Equals(o.Record, record))
                return i;
        return -1;
    }

    List<object?> ObjectRecords() => [.. Drawing.Select(shape => shape is Shape.Object x ? x.Record : null)];

    /// The record of each place of the drawing a drag started on, to find it again when the drawing is drawn anew.
    List<object?> shownObjects = [];

    /// The topmost object whose box holds a unit.
    int ObjectAt(Point p)
    {
        for (var i = Drawing.Count - 1; i >= 0; i--)
        {
            if (Drawing[i] is Shape.Object o && p.X >= o.X && p.X <= o.X + o.Width && p.Y >= o.Y && p.Y <= o.Y + o.Height)
                return i;
        }
        return -1;
    }

    // -- paint --------------------------------------------------------------

    /// The picture painted on, as the field holds it or as the last stroke left it.
    Bitmap? painted;
    /// The stroke being painted, in units.
    (IPointer Pointer, List<Point> Points)? stroke;
    int loads;

    async void LoadPicture()
    {
        var load = ++loads;
        try
        {
            var bytes = Picture switch
            {
                null => null,
                { Bytes: { } own } => own,
                _ when Fetch is { } fetch => await fetch(),
                _ => null,
            };
            if (load != loads)
                return;
            painted = bytes is null ? null : new Bitmap(new MemoryStream(bytes));
        }
        catch (Exception e)
        {
            // Fail safe: a picture that does not load leaves the canvas blank to paint on.
            Console.Error.WriteLine($"a canvas's picture did not load: {e.Message}");
            painted = null;
        }
        InvalidateVisual();
    }

    /// The size of the picture painted on: the picture's own, else the canvas's units.
    PixelSize PictureSize => painted?.PixelSize ?? new PixelSize(Math.Max(1, (int)Math.Round(UnitsWidth)), Math.Max(1, (int)Math.Round(UnitsHeight)));

    static StreamGeometry Polyline(IReadOnlyList<Point> points, Func<Point, Point> at)
    {
        var geometry = new StreamGeometry();
        using var g = geometry.Open();
        g.BeginFigure(at(points[0]), false);
        foreach (var p in points.Skip(1))
            g.LineTo(at(p));
        // A dot: the round caps of a line of no length.
        if (points.Count == 1)
            g.LineTo(at(points[0]));
        g.EndFigure(false);
        return geometry;
    }

    /// <summary>
    /// The picture after a stroke through <paramref name="points"/> (in the
    /// picture's pixels) with <paramref name="tool"/>, as a PNG: the pen paints
    /// with <paramref name="ink"/>, the eraser takes the paint away.
    /// </summary>
    public static byte[] Painted(Bitmap? before, PixelSize size, IReadOnlyList<Point> points, string tool, IBrush ink, double width)
    {
        using var after = new RenderTargetBitmap(size);
        var whole = new Rect(0, 0, size.Width, size.Height);
        var pen = new Pen(ink, Math.Max(1, width), lineCap: PenLineCap.Round, lineJoin: PenLineJoin.Round);
        using (var context = after.CreateDrawingContext())
        {
            if (before is not null)
                context.DrawImage(before, whole);
            if (tool == "eraser")
            {
                using var mask = new RenderTargetBitmap(size);
                using (var m = mask.CreateDrawingContext())
                    m.DrawGeometry(null, pen, Polyline(points, p => p));
                using (context.PushRenderOptions(new RenderOptions { BitmapBlendingMode = BitmapBlendingMode.DestinationOut }))
                    context.DrawImage(mask, whole);
            }
            else
                context.DrawGeometry(null, pen, Polyline(points, p => p));
        }
        using var png = new MemoryStream();
        after.Save(png, PngBitmapEncoderOptions.Default);
        return png.ToArray();
    }

    void EndStroke(IReadOnlyList<Point> points)
    {
        var size = PictureSize;
        // The units in the picture's pixels: the same when it is the canvas's size.
        var sx = size.Width / Math.Max(UnitsWidth, 1);
        var sy = size.Height / Math.Max(UnitsHeight, 1);
        var bytes = Painted(painted, size, [.. points.Select(p => new Point(p.X * sx, p.Y * sy))], ToolInHand, BrushOf(Ink), Brush * sx);
        // Shown as painted at once; the field's picture follows from the runtime.
        painted = new Bitmap(new MemoryStream(bytes));
        InvalidateVisual();
        Act(new JsonObject { ["stroke"] = new JsonObject { ["data"] = Convert.ToBase64String(bytes) } });
    }

    // -- vector -------------------------------------------------------------

    /// The drawing's shapes as the runtime reads its SVG, read once a change.
    List<JsonObject>? vector;

    /// <summary>The shapes of the bound drawing.</summary>
    public List<JsonObject> VectorShapes => vector ??= CanvasModes.DrawingShapes(Svg ?? "", [(int)UnitsWidth, (int)UnitsHeight]).Shapes;

    /// A shape being drawn or moved: the pointer, where it was pressed, where it is now (units).
    (IPointer Pointer, Point From, Point To, List<Point> Points, bool Moved)? drawing;

    int ChosenPlace => Chosen is long n ? (int)n : Chosen is int i ? i : -1;

    static double N(JsonObject shape, string name) => CanvasModes.Number(shape[name]) ?? 0;

    /// <summary>A vector shape as the canvas draws it; none for a line of SVG the runtime does not read.</summary>
    public static Shape? ShapeOf(JsonObject shape)
    {
        var color = (string?)shape["color"] ?? "text";
        return (string?)shape["op"] switch
        {
            "rect" => new Shape.Rect(N(shape, "x"), N(shape, "y"), N(shape, "width"), N(shape, "height"), color),
            "line" => new Shape.Line(N(shape, "x1"), N(shape, "y1"), N(shape, "x2"), N(shape, "y2"), color, N(shape, "width")),
            "ellipse" => new Shape.Ellipse(N(shape, "x"), N(shape, "y"), N(shape, "rx"), N(shape, "ry"), color),
            "path" => new Shape.Path([.. (shape["points"] as JsonArray ?? []).Select(p => (long)(CanvasModes.Number(p) ?? 0))], color, N(shape, "width")),
            "text" => new Shape.Words(N(shape, "x"), N(shape, "y"), Text.Plain((string?)shape["text"] ?? ""), color, N(shape, "size")),
            _ => null,
        };
    }

    /// <summary>The box a vector shape takes, in units, for choosing it.</summary>
    public static Rect? BoxOf(JsonObject shape)
    {
        switch ((string?)shape["op"])
        {
            case "rect":
                return new Rect(N(shape, "x"), N(shape, "y"), N(shape, "width"), N(shape, "height"));
            case "ellipse":
                return new Rect(N(shape, "x") - N(shape, "rx"), N(shape, "y") - N(shape, "ry"), 2 * N(shape, "rx"), 2 * N(shape, "ry"));
            case "line":
                var (x1, y1, x2, y2) = (N(shape, "x1"), N(shape, "y1"), N(shape, "x2"), N(shape, "y2"));
                return new Rect(new Point(Math.Min(x1, x2), Math.Min(y1, y2)), new Point(Math.Max(x1, x2), Math.Max(y1, y2))).Inflate(N(shape, "width") / 2);
            case "path":
                var points = (shape["points"] as JsonArray ?? []).Select(p => CanvasModes.Number(p) ?? 0).ToList();
                if (points.Count < 2)
                    return null;
                var xs = points.Where((_, i) => i % 2 == 0).ToList();
                var ys = points.Where((_, i) => i % 2 == 1).ToList();
                return new Rect(new Point(xs.Min(), ys.Min()), new Point(xs.Max(), ys.Max())).Inflate(N(shape, "width") / 2);
            case "text":
                var size = N(shape, "size");
                var length = ((string?)shape["text"] ?? "").Length;
                return new Rect(N(shape, "x") - size * length * 0.3, N(shape, "y") - size * 0.6, size * length * 0.6, size * 1.2);
        }
        return null;
    }

    /// <summary>A shape's first place, by which it moves.</summary>
    public static Point FirstPlace(JsonObject shape) => (string?)shape["op"] switch
    {
        "line" => new Point(N(shape, "x1"), N(shape, "y1")),
        "path" when shape["points"] is JsonArray { Count: >= 2 } p => new Point(CanvasModes.Number(p[0]) ?? 0, CanvasModes.Number(p[1]) ?? 0),
        _ => new Point(N(shape, "x"), N(shape, "y")),
    };

    /// The topmost shape whose box holds a unit, a few pixels around it too.
    int VectorShapeAt(Point p)
    {
        var near = 4 / Math.Max(Viewport.Scale, 1e-6);
        var shapes = VectorShapes;
        for (var i = shapes.Count - 1; i >= 0; i--)
        {
            if (BoxOf(shapes[i]) is { } box && box.Inflate(near).Contains(p))
                return i;
        }
        return -1;
    }

    /// <summary>The shape the tool in hand draws from <paramref name="from"/> to <paramref name="to"/> (units), in the canvas's ink and brush; none when it has no size.</summary>
    public JsonObject? Sketched(string tool, Point from, Point to, IReadOnlyList<Point> points)
    {
        var (left, top) = (Math.Min(from.X, to.X), Math.Min(from.Y, to.Y));
        var (w, h) = (Math.Abs(to.X - from.X), Math.Abs(to.Y - from.Y));
        if (tool != "path" && w == 0 && h == 0)
            return null;
        return tool switch
        {
            "line" => new JsonObject { ["op"] = "line", ["x1"] = from.X, ["y1"] = from.Y, ["x2"] = to.X, ["y2"] = to.Y, ["color"] = Ink, ["width"] = Brush },
            "rect" => new JsonObject { ["op"] = "rect", ["x"] = left, ["y"] = top, ["width"] = w, ["height"] = h, ["color"] = Ink },
            "ellipse" => new JsonObject { ["op"] = "ellipse", ["x"] = left + w / 2, ["y"] = top + h / 2, ["rx"] = w / 2, ["ry"] = h / 2, ["color"] = Ink },
            "path" when points.Count >= 2 => new JsonObject { ["op"] = "path", ["points"] = new JsonArray([.. points.SelectMany(p => new JsonNode[] { p.X, p.Y })]), ["color"] = Ink, ["width"] = Brush },
            _ => null,
        };
    }

    /// Where a text is being typed, in units.
    Point? typingAt;

    /// The text tool: an input where the person pressed, which Enter places.
    void StartTyping(Point unit)
    {
        EndTyping(false);
        typingAt = unit;
        typing = new TextBox { PlaceholderText = Locale.Translate("canvasText", "The text"), MinWidth = 160 };
        typing.KeyDown += (_, e) =>
        {
            if (e.Key is Key.Enter or Key.Escape)
            {
                e.Handled = true;
                EndTyping(e.Key == Key.Enter);
            }
        };
        typing.LostFocus += (_, _) => Dispatcher.UIThread.Post(() => EndTyping(true));
        LogicalChildren.Add(typing);
        VisualChildren.Add(typing);
        InvalidateMeasure();
        Dispatcher.UIThread.Post(() => typing?.Focus());
    }

    /// <summary>Places the text typed (<paramref name="place"/>) or drops it.</summary>
    public void EndTyping(bool place)
    {
        if (typing is not { } box || typingAt is not { } at)
            return;
        typing = null;
        typingAt = null;
        VisualChildren.Remove(box);
        LogicalChildren.Remove(box);
        if (place && !string.IsNullOrWhiteSpace(box.Text))
            Act(new JsonObject { ["add"] = new JsonObject { ["op"] = "text", ["x"] = at.X, ["y"] = at.Y, ["text"] = box.Text, ["color"] = Ink, ["size"] = Math.Max(10, Brush * 4) } });
    }

    /// <summary>The text input of the text tool, while the person types; for tests.</summary>
    public TextBox? Typing => typing;

    // -- the pointer and the keys ---------------------------------------------

    bool Acts => Mode is "objects" or "paint" or "vector" && !IsReadOnly && Runs is null;

    /// Whether a visual is <paramref name="part"/> or in it.
    static bool Within(Visual? v, Visual? part)
    {
        for (; v is not null && part is not null; v = v.GetVisualParent())
            if (v == part)
                return true;
        return false;
    }

    /// A press of a canvas with a mode; true when the mode took it.
    bool ModePressed(PointerPressedEventArgs e)
    {
        if (!Acts || !e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
            return false;
        if (e.Source is Visual v && (Within(v, toolbar) || Within(v, typing)))
            return false;
        Focusable = true;
        Focus(NavigationMethod.Pointer);
        var at = e.GetPosition(this);
        var unit = UnitAt(at);
        switch (Mode)
        {
            case "objects":
                var box = ObjectAt(unit);
                if (box < 0)
                    return false;
                var o = (Shape.Object)Drawing[box];
                shownObjects = ObjectRecords();
                dragging = (e.Pointer, box, at, new Point(o.X, o.Y), default, false);
                break;
            case "paint":
                stroke = (e.Pointer, [unit]);
                break;
            case "vector":
                var tool = ToolInHand;
                if (tool == "text")
                {
                    StartTyping(unit);
                    e.Handled = true;
                    return true;
                }
                if (tool == "select")
                {
                    var place = VectorShapeAt(unit);
                    Chosen = place < 0 ? null : (long)place;
                    if (place < 0)
                    {
                        InvalidateVisual();
                        e.Handled = true;
                        return true;
                    }
                }
                drawing = (e.Pointer, unit, unit, [unit], false);
                break;
            default:
                return false;
        }
        e.Pointer.Capture(this);
        e.Handled = true;
        InvalidateVisual();
        return true;
    }

    bool ModeMoved(PointerEventArgs e)
    {
        var at = e.GetPosition(this);
        if (dragging is { } d && d.Pointer == e.Pointer)
        {
            if (!d.Moved && Math.Sqrt(Math.Pow(at.X - d.At.X, 2) + Math.Pow(at.Y - d.At.Y, 2)) < Still)
                return true;
            var scale = Viewport.Scale;
            dragging = d with { By = new Vector((at.X - d.At.X) / scale, (at.Y - d.At.Y) / scale), Moved = true };
            InvalidateVisual();
            return true;
        }
        if (stroke is { } s && s.Pointer == e.Pointer)
        {
            s.Points.Add(UnitAt(at));
            InvalidateVisual();
            return true;
        }
        if (drawing is { } w && w.Pointer == e.Pointer)
        {
            var unit = UnitAt(at);
            w.Points.Add(unit);
            drawing = w with { To = unit, Moved = true };
            InvalidateVisual();
            return true;
        }
        return false;
    }

    bool ModeReleased(PointerReleasedEventArgs e)
    {
        if (dragging is { } d && d.Pointer == e.Pointer)
        {
            // Taken first: letting the pointer go loses its capture, which ends the gesture.
            dragging = null;
            e.Pointer.Capture(null);
            e.Handled = true;
            if (d.Moved && Drawing[d.Box] is Shape.Object o)
            {
                var id = o.Record is null ? null : System.Text.Json.JsonSerializer.SerializeToNode(o.Record, o.Record.GetType());
                Act(new JsonObject { ["move"] = new JsonObject { ["id"] = id, ["x"] = d.From.X + d.By.X, ["y"] = d.From.Y + d.By.Y } });
            }
            // What the runtime kept shows from its drawing; a refused drop puts the box back.
            dragging = null;
            InvalidateVisual();
            return true;
        }
        if (stroke is { } s && s.Pointer == e.Pointer)
        {
            stroke = null;
            e.Pointer.Capture(null);
            e.Handled = true;
            EndStroke(s.Points);
            return true;
        }
        if (drawing is { } w && w.Pointer == e.Pointer)
        {
            drawing = null;
            e.Pointer.Capture(null);
            e.Handled = true;
            var tool = ToolInHand;
            if (tool == "select")
            {
                var place = ChosenPlace;
                if (w.Moved && place >= 0 && place < VectorShapes.Count)
                {
                    var first = FirstPlace(VectorShapes[place]);
                    Act(new JsonObject { ["move"] = new JsonObject { ["index"] = place + 1, ["x"] = first.X + w.To.X - w.From.X, ["y"] = first.Y + w.To.Y - w.From.Y } });
                }
            }
            else if (w.Moved && Sketched(tool, w.From, w.To, w.Points) is { } shape)
                Act(new JsonObject { ["add"] = shape });
            InvalidateVisual();
            return true;
        }
        return false;
    }

    /// Delete deletes the chosen shape of a vector canvas.
    bool ModeKey(KeyEventArgs e)
    {
        if (!Acts || Mode != "vector" || e.Key is not (Key.Delete or Key.Back) || e.KeyModifiers != KeyModifiers.None || ChosenPlace < 0 || typing is not null)
            return false;
        e.Handled = true;
        Act(new JsonObject { ["delete"] = ChosenPlace + 1 });
        return true;
    }

    void ModeCaptureLost()
    {
        dragging = null;
        stroke = null;
        drawing = null;
        InvalidateVisual();
    }

    // -- drawing ----------------------------------------------------------------

    /// The drawing as it shows: an object being dragged moved with what is drawn after it.
    IEnumerable<Shape> ModeShapes(IEnumerable<Shape> shapes) =>
        dragging is { Moved: true } d ? WithObjectMoved([.. shapes], d.Box, d.By) : shapes;

    /// What a mode draws over the drawing: the painted picture and the stroke, or the vector shapes and the one being drawn.
    void RenderMode(DrawingContext context, double scale, Func<double, double, Point> at)
    {
        Point P(Point p) => at(p.X, p.Y);
        if (Mode == "paint")
        {
            if (painted is not null)
                context.DrawImage(painted, new Rect(at(0, 0), new Size(UnitsWidth * scale, UnitsHeight * scale)));
            if (stroke is { } s)
            {
                var ink = ToolInHand == "eraser" ? BrushOf(Paper != "" ? Paper : "panel") : BrushOf(Ink);
                context.DrawGeometry(null, new Pen(ink, Math.Max(1, Brush) * scale, lineCap: PenLineCap.Round, lineJoin: PenLineJoin.Round), Polyline(s.Points, P));
            }
            return;
        }
        if (Mode != "vector")
            return;
        var shapes = VectorShapes;
        var chosen = ChosenPlace;
        var by = drawing is { Moved: true } w && ToolInHand == "select" ? new Vector(w.To.X - w.From.X, w.To.Y - w.From.Y) : default;
        for (var i = 0; i < shapes.Count; i++)
        {
            if (ShapeOf(shapes[i]) is not { } shape)
                continue;
            DrawShape(context, i == chosen ? Translated(shape, by.X, by.Y) : shape, scale, at);
        }
        if (chosen >= 0 && chosen < shapes.Count && BoxOf(shapes[chosen]) is { } box)
        {
            var moved = box.Translate(by);
            context.DrawRectangle(null, new Pen(BrushOf("accent"), 1.5, new DashStyle([4, 3], 0)), new Rect(P(moved.TopLeft), P(moved.BottomRight)).Inflate(3));
        }
        if (drawing is { Moved: true } d && ToolInHand != "select" && Sketched(ToolInHand, d.From, d.To, d.Points) is { } sketch && ShapeOf(sketch) is { } preview)
            DrawShape(context, preview, scale, at);
    }
}
