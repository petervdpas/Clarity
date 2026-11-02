// A canvas with a mode (DESIGN 5.31, conformance/canvas-modes.json): what
// the person does on it. The canvas shows the gesture and sends what was
// done as one action (a JSON object); the runtime decides what it does,
// snaps it and keeps it (WindowViewModel.CanvasEvent), and the canvas shows
// the data that comes back.
//   objects: an `object` shape of the drawing marks a record's box. A press
//     in it and a drag moves the box and the shapes drawn after it, until the
//     next object, as a preview; the drop sends {"move": {id, x, y}}.
//   paint: strokes of the brush, its brush's pixels wide in its ink, on the
//     picture of the bound image field; a stroke's end sends
//     {"stroke": {"data": PNG}}.
//   vector: the shapes of the bound text field's SVG, with a toolbar of the
//     mode's tools. Drawing a shape adds it, select and drag moves the chosen
//     one, Delete deletes it, and the text tool takes its text where it was
//     pressed until Enter places it.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

public sealed partial class CanvasPart
{
    /// <summary>What the person does on it: objects, paint or vector; empty: its draw handler draws it.</summary>
    public string Mode => Spec.Json("mode") is JsonValue v && v.TryGetValue<string>(out var mode) ? mode : "";

    /// <summary>Whether the person acts on it: it has a mode, may be used and changed, and no engine draws it.</summary>
    bool Acts => Mode is "objects" or "paint" or "vector" && !ReadOnly && Enabled && Engine is null;

    /// <summary>The tools of each mode, in the order its toolbar shows them.</summary>
    public static readonly IReadOnlyDictionary<string, string[]> ModeTools = new Dictionary<string, string[]>
    {
        ["paint"] = ["pen", "eraser"],
        ["vector"] = ["select", "line", "rect", "ellipse", "path", "text"],
    };

    /// <summary>A tool's name in the person's language.</summary>
    public static string ToolWord(string tool) => tool switch
    {
        "select" => Locale.Translate("toolSelect", "Select"),
        "line" => Locale.Translate("toolLine", "Line"),
        "rect" => Locale.Translate("toolRect", "Rectangle"),
        "ellipse" => Locale.Translate("toolEllipse", "Ellipse"),
        "path" => Locale.Translate("toolPath", "Path"),
        "text" => Locale.Translate("toolText", "Text"),
        "pen" => Locale.Translate("toolPen", "Pen"),
        "eraser" => Locale.Translate("toolEraser", "Eraser"),
        _ => tool,
    };

    /// <summary>The tools its toolbar shows: the canvas's own, of its mode, in the mode's order; all of them when it names none.</summary>
    public IReadOnlyList<string> ToolsShown()
    {
        if (!ModeTools.TryGetValue(Mode, out var all))
            return [];
        var named = Spec.Names("tools");
        return named.Count == 0 ? all : [.. all.Where(named.Contains)];
    }

    /// <summary>The tool in hand: the one its state names, when the canvas has it, else its first.</summary>
    public string ToolInHand => ToolsShown() is { Count: > 0 } tools ? tools.Contains(State?.Tool ?? "") ? State!.Tool : tools[0] : "";

    /// <summary>How wide it paints or draws, in its units.</summary>
    public double Brush => State is { } s ? s.Brush : Spec.Number("brush") ?? 2;

    /// <summary>The color it paints or draws with: the theme's by name, or written out.</summary>
    public string Ink => State is { Ink.Length: > 0 } s ? s.Ink : Spec.Text("ink") ?? "text";

    /// <summary>The shape chosen, its place in the drawing from 0; -1 for none.</summary>
    int ChosenPlace => State?.SelectedItem switch
    {
        long n => (int)n,
        int i => i,
        _ => -1,
    };

    void Act(JsonObject action)
    {
        if (Acts && Spec.Name is { } name)
            Host.Acted(name, action);
    }

    // -- The toolbar -------------------------------------------------------------

    const double ToolHeight = 24;

    /// <summary>Its toolbar's buttons on the screen, in the order of its tools.</summary>
    public IReadOnlyList<(string Tool, Box Box)> ToolButtons()
    {
        var buttons = new List<(string, Box)>();
        var x = Bounds.X + 6;
        foreach (var tool in ToolsShown())
        {
            var width = FontsNow.Measure(ToolWord(tool), "text", LookNow.Size("small"), false) + 16;
            buttons.Add((tool, new Box(x, Bounds.Y + 6, width, ToolHeight)));
            x += width + 2;
        }
        return buttons;
    }

    void DrawToolbar(Painter painter)
    {
        var look = painter.Look;
        var inHand = ToolInHand;
        foreach (var (tool, box) in ToolButtons())
        {
            var chosen = tool == inHand;
            painter.Fill(box, look.Color(chosen ? "accent" : "panel"), look.Radius("button"));
            painter.Stroke(box, look.Color("border"), look.Border, look.Radius("button"));
            painter.Text(ToolWord(tool), box, look.Color(chosen ? "on_accent" : "text"), look.Size("small"), align: TextAlign.Center);
        }
    }

    // -- objects -----------------------------------------------------------------

    /// <summary>An object being dragged: the place of its box in the drawing, where it was pressed, where its box was and how far it moved (units).</summary>
    (int Box, double AtX, double AtY, double FromX, double FromY, double ByX, double ByY, bool Moved)? dragging;

    /// <summary>The record of each place of the drawing a drag started on, to find it again when the drawing is drawn anew.</summary>
    List<object?> shownObjects = [];

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
    public static IEnumerable<Shape> WithObjectMoved(IReadOnlyList<Shape> shapes, int box, double dx, double dy)
    {
        var moving = false;
        for (var i = 0; i < shapes.Count; i++)
        {
            if (shapes[i] is Shape.Object)
                moving = i == box;
            yield return moving ? Translated(shapes[i], dx, dy) : shapes[i];
        }
    }

    List<object?> ObjectRecords() => [.. Drawing.Select(shape => shape is Shape.Object x ? x.Record : null)];

    /// <summary>The topmost object whose box holds a unit; -1 for none.</summary>
    int ObjectAt((double X, double Y) p)
    {
        for (var i = Drawing.Count - 1; i >= 0; i--)
            if (Drawing[i] is Shape.Object o && p.X >= o.X && p.X <= o.X + o.Width && p.Y >= o.Y && p.Y <= o.Y + o.Height)
                return i;
        return -1;
    }

    /// <summary>A drawing drawn anew (after any handler, a timer's tick too) keeps a drag going, on the same record's object in it.</summary>
    void DrawingChanged()
    {
        if (dragging is not { } held)
            return;
        var record = held.Box < shownObjects.Count ? shownObjects[held.Box] : null;
        var box = -1;
        for (var i = 0; i < Drawing.Count; i++)
            if (Drawing[i] is Shape.Object o && Equals(o.Record, record))
            {
                box = i;
                break;
            }
        shownObjects = ObjectRecords();
        dragging = box >= 0 ? held with { Box = box } : null;
    }

    // -- paint -------------------------------------------------------------------

    /// <summary>The picture painted on: as the field holds it, or as the last stroke left it.</summary>
    SKBitmap? painted;

    /// <summary>The field's picture <see cref="painted"/> was read from.</summary>
    object? paintedFrom;

    /// <summary>The stroke being painted, in units.</summary>
    List<(double X, double Y)>? stroke;

    /// <summary>The picture of the bound image field, read again when the field holds another.</summary>
    SKBitmap? Painted()
    {
        var value = Bound is { } path ? Host.Read(path) : null;
        if (!ReferenceEquals(value, paintedFrom))
        {
            paintedFrom = value;
            painted?.Dispose();
            painted = null;
            // Fail safe: a picture that does not load leaves the canvas blank to paint on.
            if (value is Picture { Bytes: { } bytes })
                try
                {
                    painted = SKBitmap.Decode(bytes);
                }
                catch (Exception e) when (e is ArgumentException or InvalidOperationException or IOException)
                {
                    Console.Error.WriteLine($"a canvas's picture did not load: {e.Message}");
                }
        }
        return painted;
    }

    /// <summary>The size of the picture painted on: the picture's own, else the canvas's units.</summary>
    (int Width, int Height) PictureSize => Painted() is { } p ? (p.Width, p.Height) : (Math.Max(1, (int)Math.Round(UnitsWidth)), Math.Max(1, (int)Math.Round(UnitsHeight)));

    /// <summary>
    /// The picture after a stroke through <paramref name="points"/> (in the
    /// picture's pixels) with <paramref name="tool"/>, as a PNG: the pen paints
    /// with <paramref name="ink"/>, the eraser takes the paint away.
    /// </summary>
    public static byte[] PaintedWith(SKBitmap? before, int width, int height, IReadOnlyList<(double X, double Y)> points, string tool, SKColor ink, double brush)
    {
        using var after = new SKBitmap(new SKImageInfo(width, height, SKColorType.Rgba8888, SKAlphaType.Premul));
        using (var canvas = new SKCanvas(after))
        {
            canvas.Clear(SKColors.Transparent);
            if (before is not null)
            {
                using var picture = SKImage.FromBitmap(before);
                canvas.DrawImage(picture, new SKRect(0, 0, width, height), new SKSamplingOptions(SKFilterMode.Linear, SKMipmapMode.None));
            }
            using var pen = new SKPaint
            {
                Color = ink,
                IsAntialias = true,
                Style = SKPaintStyle.Stroke,
                StrokeWidth = (float)Math.Max(1, brush),
                StrokeCap = SKStrokeCap.Round,
                StrokeJoin = SKStrokeJoin.Round,
                BlendMode = tool == "eraser" ? SKBlendMode.Clear : SKBlendMode.SrcOver,
            };
            Polyline(canvas, [.. points.Select(p => new SKPoint((float)p.X, (float)p.Y))], pen);
        }
        using var image = SKImage.FromBitmap(after);
        using var png = image.Encode(SKEncodedImageFormat.Png, 100);
        return png.ToArray();
    }

    void EndStroke(IReadOnlyList<(double X, double Y)> points)
    {
        var (width, height) = PictureSize;
        // The units in the picture's pixels: the same when it is the canvas's size.
        var sx = width / Math.Max(UnitsWidth, 1);
        var sy = height / Math.Max(UnitsHeight, 1);
        var bytes = PaintedWith(Painted(), width, height, [.. points.Select(p => (p.X * sx, p.Y * sy))], ToolInHand, ColorOf(LookNow, Ink), Brush * sx);
        // Shown as painted at once; the field's picture follows from the runtime.
        painted?.Dispose();
        painted = SKBitmap.Decode(bytes);
        Invalidate();
        Act(new JsonObject { ["stroke"] = new JsonObject { ["data"] = Convert.ToBase64String(bytes) } });
    }

    // -- vector ------------------------------------------------------------------

    /// <summary>The drawing's shapes as the runtime reads its SVG, read once for each text.</summary>
    (string? Svg, List<JsonObject> Shapes)? vector;

    /// <summary>The shapes of the bound drawing.</summary>
    public List<JsonObject> VectorShapes
    {
        get
        {
            var svg = Bound is { } path ? Host.Read(path) as string : null;
            if (vector is not { } kept || kept.Svg != svg)
                vector = kept = (svg, CanvasModes.DrawingShapes(svg ?? "", [(int)UnitsWidth, (int)UnitsHeight]).Shapes);
            return kept.Shapes;
        }
    }

    /// <summary>A shape being drawn or moved: where it was pressed, where it is now (units), the points it went through, whether it moved.</summary>
    (double FromX, double FromY, double ToX, double ToY, List<(double X, double Y)> Points, bool Moved)? sketching;

    static double N(JsonObject shape, string name) => CanvasModes.Number(shape[name]) ?? 0;

    static string S(JsonObject shape, string name, string otherwise) => shape[name] is JsonValue v && v.TryGetValue<string>(out var s) ? s : otherwise;

    /// <summary>A vector shape as the canvas draws it; none for a line of SVG the runtime does not read.</summary>
    public static Shape? ShapeOf(JsonObject shape)
    {
        var color = S(shape, "color", "text");
        return S(shape, "op", "") switch
        {
            "rect" => new Shape.Rect(N(shape, "x"), N(shape, "y"), N(shape, "width"), N(shape, "height"), color),
            "line" => new Shape.Line(N(shape, "x1"), N(shape, "y1"), N(shape, "x2"), N(shape, "y2"), color, N(shape, "width")),
            "ellipse" => new Shape.Ellipse(N(shape, "x"), N(shape, "y"), N(shape, "rx"), N(shape, "ry"), color),
            "path" => new Shape.Path([.. (shape["points"] as JsonArray ?? []).Select(p => (long)(CanvasModes.Number(p) ?? 0))], color, N(shape, "width")),
            "text" => new Shape.Words(N(shape, "x"), N(shape, "y"), Text.Plain(S(shape, "text", "")), color, N(shape, "size")),
            _ => null,
        };
    }

    /// <summary>The box a vector shape takes, in units, for choosing it.</summary>
    public static Box? BoxOf(JsonObject shape)
    {
        static Box Between(double x1, double y1, double x2, double y2, double grow) =>
            new(Math.Min(x1, x2) - grow, Math.Min(y1, y2) - grow, Math.Abs(x2 - x1) + 2 * grow, Math.Abs(y2 - y1) + 2 * grow);
        switch (S(shape, "op", ""))
        {
            case "rect":
                return new Box(N(shape, "x"), N(shape, "y"), N(shape, "width"), N(shape, "height"));
            case "ellipse":
                return new Box(N(shape, "x") - N(shape, "rx"), N(shape, "y") - N(shape, "ry"), 2 * N(shape, "rx"), 2 * N(shape, "ry"));
            case "line":
                return Between(N(shape, "x1"), N(shape, "y1"), N(shape, "x2"), N(shape, "y2"), N(shape, "width") / 2);
            case "path":
                var points = (shape["points"] as JsonArray ?? []).Select(p => CanvasModes.Number(p) ?? 0).ToList();
                if (points.Count < 2)
                    return null;
                var xs = points.Where((_, i) => i % 2 == 0).ToList();
                var ys = points.Where((_, i) => i % 2 == 1).ToList();
                return Between(xs.Min(), ys.Min(), xs.Max(), ys.Max(), N(shape, "width") / 2);
            case "text":
                var size = N(shape, "size");
                var length = S(shape, "text", "").Length;
                return new Box(N(shape, "x") - size * length * 0.3, N(shape, "y") - size * 0.6, size * length * 0.6, size * 1.2);
        }
        return null;
    }

    /// <summary>A shape's first place, by which it moves.</summary>
    public static (double X, double Y) FirstPlace(JsonObject shape) => S(shape, "op", "") switch
    {
        "line" => (N(shape, "x1"), N(shape, "y1")),
        "path" when shape["points"] is JsonArray { Count: >= 2 } p => (CanvasModes.Number(p[0]) ?? 0, CanvasModes.Number(p[1]) ?? 0),
        _ => (N(shape, "x"), N(shape, "y")),
    };

    /// <summary>The topmost shape whose box holds a unit, a few pixels around it too; -1 for none.</summary>
    int VectorShapeAt((double X, double Y) p)
    {
        var near = 4 / Math.Max(Viewport.Scale, 1e-6);
        var shapes = VectorShapes;
        for (var i = shapes.Count - 1; i >= 0; i--)
            if (BoxOf(shapes[i]) is { } box && p.X >= box.X - near && p.X <= box.Right + near && p.Y >= box.Y - near && p.Y <= box.Bottom + near)
                return i;
        return -1;
    }

    /// <summary>The shape <paramref name="tool"/> draws from one unit to another, in the canvas's ink and brush; none when it has no size.</summary>
    public JsonObject? Sketched(string tool, double fromX, double fromY, double toX, double toY, IReadOnlyList<(double X, double Y)> points)
    {
        var (left, top) = (Math.Min(fromX, toX), Math.Min(fromY, toY));
        var (w, h) = (Math.Abs(toX - fromX), Math.Abs(toY - fromY));
        if (tool != "path" && w == 0 && h == 0)
            return null;
        return tool switch
        {
            "line" => new JsonObject { ["op"] = "line", ["x1"] = fromX, ["y1"] = fromY, ["x2"] = toX, ["y2"] = toY, ["color"] = Ink, ["width"] = Brush },
            "rect" => new JsonObject { ["op"] = "rect", ["x"] = left, ["y"] = top, ["width"] = w, ["height"] = h, ["color"] = Ink },
            "ellipse" => new JsonObject { ["op"] = "ellipse", ["x"] = left + w / 2, ["y"] = top + h / 2, ["rx"] = w / 2, ["ry"] = h / 2, ["color"] = Ink },
            "path" when points.Count >= 2 => new JsonObject { ["op"] = "path", ["points"] = new JsonArray([.. points.SelectMany(p => new JsonNode[] { p.X, p.Y })]), ["color"] = Ink, ["width"] = Brush },
            _ => null,
        };
    }

    /// <summary>Where a text is being typed, in units; none while no text is.</summary>
    (double X, double Y)? typingAt;

    /// <summary>The text typed with the text tool so far; none while no text is being typed.</summary>
    public string? Typing { get; private set; }

    void StartTyping((double X, double Y) unit)
    {
        EndTyping(true);
        typingAt = unit;
        Typing = "";
        Invalidate();
    }

    /// <summary>Places the text typed (<paramref name="place"/>) or drops it.</summary>
    public void EndTyping(bool place)
    {
        if (Typing is not { } text || typingAt is not { } at)
            return;
        Typing = null;
        typingAt = null;
        Invalidate();
        if (place && !string.IsNullOrWhiteSpace(text))
            Act(new JsonObject { ["add"] = new JsonObject { ["op"] = "text", ["x"] = at.X, ["y"] = at.Y, ["text"] = text, ["color"] = Ink, ["size"] = Math.Max(10, Brush * 4) } });
    }

    public override void OnText(string text)
    {
        if (Typing is null)
            return;
        Typing += text;
        Invalidate();
    }

    public override void OnFocusChanged(bool focused)
    {
        // A text being typed is placed when the canvas loses the keyboard.
        if (!focused)
            EndTyping(true);
    }

    public override bool OnKey(KeyInput key)
    {
        if (Typing is { } typed)
        {
            switch (key.Key)
            {
                case Key.Enter or Key.Escape:
                    EndTyping(key.Key == Key.Enter);
                    return true;
                case Key.Backspace:
                    if (typed.Length > 0)
                        Typing = typed[..^1];
                    Invalidate();
                    return true;
            }
            return false;
        }
        // Delete deletes the chosen shape of a vector canvas.
        if (!Acts || Mode != "vector" || key.Key is not (Key.Delete or Key.Backspace) || key.Modifiers != Modifiers.None || ChosenPlace < 0)
            return false;
        Act(new JsonObject { ["delete"] = ChosenPlace + 1 });
        return true;
    }

    // -- the pointer ----------------------------------------------------------------

    /// <summary>A press of a canvas with a mode; true when the mode took it.</summary>
    bool ModePressed(PointerInput at)
    {
        if (!Acts)
            return false;
        foreach (var (tool, box) in ToolButtons())
            if (box.Contains(at.ScreenX, at.ScreenY))
            {
                if (State is { } state)
                    state.Tool = tool;
                Invalidate();
                return true;
            }
        var unit = UnitAt(at.X, at.Y);
        switch (Mode)
        {
            case "objects":
                var box = ObjectAt(unit);
                if (box < 0)
                    return false;
                var o = (Shape.Object)Drawing[box];
                shownObjects = ObjectRecords();
                dragging = (box, at.X, at.Y, o.X, o.Y, 0, 0, false);
                break;
            case "paint":
                stroke = [unit];
                break;
            case "vector":
                var tool = ToolInHand;
                if (tool == "text")
                {
                    StartTyping(unit);
                    return true;
                }
                EndTyping(true);
                if (tool == "select")
                {
                    var place = VectorShapeAt(unit);
                    if (State is { } state)
                        state.SelectedItem = place < 0 ? null : (long)place;
                    if (place < 0)
                    {
                        Invalidate();
                        return true;
                    }
                }
                sketching = (unit.X, unit.Y, unit.X, unit.Y, [unit], false);
                break;
            default:
                return false;
        }
        Invalidate();
        return true;
    }

    bool ModeMoved(PointerInput at)
    {
        if (dragging is { } d)
        {
            if (!d.Moved && Math.Sqrt(Math.Pow(at.X - d.AtX, 2) + Math.Pow(at.Y - d.AtY, 2)) < Still)
                return true;
            var scale = Viewport.Scale;
            dragging = d with { ByX = (at.X - d.AtX) / scale, ByY = (at.Y - d.AtY) / scale, Moved = true };
            Invalidate();
            return true;
        }
        if (stroke is { } s)
        {
            s.Add(UnitAt(at.X, at.Y));
            Invalidate();
            return true;
        }
        if (sketching is { } w)
        {
            var unit = UnitAt(at.X, at.Y);
            w.Points.Add(unit);
            sketching = w with { ToX = unit.X, ToY = unit.Y, Moved = true };
            Invalidate();
            return true;
        }
        return false;
    }

    bool ModeReleased(PointerInput at)
    {
        if (dragging is { } d)
        {
            dragging = null;
            if (d.Moved && d.Box < Drawing.Count && Drawing[d.Box] is Shape.Object o)
            {
                var id = o.Record is null ? null : System.Text.Json.JsonSerializer.SerializeToNode(o.Record, o.Record.GetType());
                Act(new JsonObject { ["move"] = new JsonObject { ["id"] = id, ["x"] = d.FromX + d.ByX, ["y"] = d.FromY + d.ByY } });
            }
            // What the runtime kept shows from its drawing; a refused drop puts the box back.
            Invalidate();
            return true;
        }
        if (stroke is { } s)
        {
            stroke = null;
            EndStroke(s);
            return true;
        }
        if (sketching is { } w)
        {
            sketching = null;
            var tool = ToolInHand;
            if (tool == "select")
            {
                var place = ChosenPlace;
                if (w.Moved && place >= 0 && place < VectorShapes.Count)
                {
                    var first = FirstPlace(VectorShapes[place]);
                    Act(new JsonObject { ["move"] = new JsonObject { ["index"] = place + 1, ["x"] = first.X + w.ToX - w.FromX, ["y"] = first.Y + w.ToY - w.FromY } });
                }
            }
            else if (w.Moved && Sketched(tool, w.FromX, w.FromY, w.ToX, w.ToY, w.Points) is { } shape)
                Act(new JsonObject { ["add"] = shape });
            Invalidate();
            return true;
        }
        return false;
    }

    // -- drawing ----------------------------------------------------------------------

    /// <summary>The drawing as it shows: an object being dragged moved with what is drawn after it.</summary>
    IEnumerable<Shape> ModeShapes(IReadOnlyList<Shape> shapes) =>
        dragging is { Moved: true } d ? WithObjectMoved(shapes, d.Box, d.ByX, d.ByY) : shapes;

    /// <summary>What a mode draws over the drawing: the painted picture and the stroke, or the vector shapes, the chosen one's frame and the one being drawn; and its toolbar.</summary>
    void DrawMode(Painter painter, CanvasViewport v)
    {
        SKPoint P(double x, double y) => At(v, x, y);
        var canvas = painter.Canvas;
        if (Mode == "paint")
        {
            if (Painted() is { } picture)
            {
                var start = P(0, 0);
                using var image = SKImage.FromBitmap(picture);
                canvas.DrawImage(image, new SKRect(start.X, start.Y, start.X + (float)(UnitsWidth * v.Scale), start.Y + (float)(UnitsHeight * v.Scale)), new SKSamplingOptions(SKFilterMode.Linear, SKMipmapMode.Linear));
            }
            if (stroke is { Count: > 0 } s)
            {
                var ink = ToolInHand == "eraser" ? ColorOf(painter.Look, Paper != "" ? Paper : "panel") : ColorOf(painter.Look, Ink);
                using var pen = new SKPaint { Color = ink, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = (float)(Math.Max(1, Brush) * v.Scale), StrokeCap = SKStrokeCap.Round, StrokeJoin = SKStrokeJoin.Round };
                Polyline(canvas, [.. s.Select(p => P(p.X, p.Y))], pen);
            }
        }
        else if (Mode == "vector")
        {
            var shapes = VectorShapes;
            var chosen = ChosenPlace;
            var (byX, byY) = sketching is { Moved: true } w && ToolInHand == "select" ? (w.ToX - w.FromX, w.ToY - w.FromY) : (0.0, 0.0);
            for (var i = 0; i < shapes.Count; i++)
                if (ShapeOf(shapes[i]) is { } shape)
                    DrawShape(painter, i == chosen ? Translated(shape, byX, byY) : shape, v.Scale, P, Picture);
            if (chosen >= 0 && chosen < shapes.Count && BoxOf(shapes[chosen]) is { } box)
            {
                var (a, b) = (P(box.X + byX, box.Y + byY), P(box.Right + byX, box.Bottom + byY));
                using var frame = new SKPaint { Color = painter.Look.Color("accent"), IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.5f, PathEffect = SKPathEffect.CreateDash([4, 3], 0) };
                canvas.DrawRect(new SKRect(a.X - 3, a.Y - 3, b.X + 3, b.Y + 3), frame);
            }
            if (sketching is { Moved: true } d && ToolInHand != "select" && Sketched(ToolInHand, d.FromX, d.FromY, d.ToX, d.ToY, d.Points) is { } sketch && ShapeOf(sketch) is { } preview)
                DrawShape(painter, preview, v.Scale, P, Picture);
            if (Typing is { } typed && typingAt is { } at)
            {
                // The text being typed, where it goes, with its caret.
                var p = P(at.X, at.Y);
                var size = painter.Look.Size("text");
                var width = Math.Max(160, painter.Fonts.Measure(typed, "text", size, false) + 12);
                var field = new Box(p.X, p.Y - size, width, size * 2);
                painter.Fill(field, painter.Look.Color("input"), painter.Look.Radius("input"));
                painter.Stroke(field, painter.Look.Color("accent"), painter.Look.Border, painter.Look.Radius("input"));
                painter.Text(typed.Length == 0 ? Locale.Translate("canvasText", "The text") : typed + "|", field.Inset(4), painter.Look.Color(typed.Length == 0 ? "muted" : "text"), size);
            }
        }
        if (ToolsShown().Count > 0)
            DrawToolbar(painter);
    }
}
