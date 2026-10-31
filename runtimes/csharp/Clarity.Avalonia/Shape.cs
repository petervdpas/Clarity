// A shape a draw handler draws on its canvas, in the canvas's own units
// (conformance/drawing.json). CanvasView draws them.

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public abstract record Shape
{
    /// Fills the whole canvas.
    public sealed record Clear(string Color) : Shape;

    public sealed record Line(double X1, double Y1, double X2, double Y2, string Color, double Width = 1) : Shape;

    public sealed record Rect(double X, double Y, double Width, double Height, string Color) : Shape;

    public sealed record Circle(double X, double Y, double R, string Color) : Shape;

    /// A circle's outline.
    public sealed record Ring(double X, double Y, double R, string Color, double Width = 1) : Shape;

    /// Text centered on its point, shown in the person's format.
    public sealed record Words(double X, double Y, Text Text, string Color = "text", double Size = 10) : Shape;

    /// <summary>A picture of the resources folder by its name, stretched to fill its box.</summary>
    public sealed record Image(double X, double Y, double Width, double Height, string Picture) : Shape;

    /// <summary>A line people bend, named so code knows which: from X1, Y1 through its bend points
    /// (<paramref name="Via"/>, x, y, x, y, ...) to X2, Y2.</summary>
    public sealed record Link(string Name, double X1, double Y1, double X2, double Y2, IReadOnlyList<long> Via, string Color, double Width = 1) : Shape;

    /// <summary>The box of an object of a canvas in mode objects (DESIGN 5.31), by its record's id:
    /// it draws nothing itself; what is drawn after it, until the next object, moves with it while it is dragged.</summary>
    public sealed record Object(object? Record, double X, double Y, double Width, double Height) : Shape;

    /// <summary>A filled ellipse by its center and its radii.</summary>
    public sealed record Ellipse(double X, double Y, double Rx, double Ry, string Color) : Shape;

    /// <summary>An open line through its points (x, y, x, y, ...).</summary>
    public sealed record Path(IReadOnlyList<long> Points, string Color, double Width = 1) : Shape;

    /// <summary>A shape as a draw handler or an engine sends it (conformance/drawing.json).</summary>
    public static Shape Of(JsonNode sent)
    {
        // A number, or its text as an engine gives a decimal.
        double N(string name, double otherwise = double.NaN) =>
            sent[name] is JsonValue v && (v.TryGetValue<double>(out var n) || v.TryGetValue<string>(out var t) && double.TryParse(t, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out n))
                ? n
                : double.IsNaN(otherwise) ? throw new EngineException($"a shape's {name} is no number: {sent.ToJsonString()}") : otherwise;
        string T(string name, string otherwise) => sent[name] is JsonValue v && v.TryGetValue<string>(out var t) ? t : otherwise;
        return T("op", "") switch
        {
            "clear" => new Clear(T("color", "background")),
            "line" => new Line(N("x1"), N("y1"), N("x2"), N("y2"), T("color", "text"), N("width", 1)),
            "rect" => new Rect(N("x"), N("y"), N("width"), N("height"), T("color", "text")),
            "circle" => new Circle(N("x"), N("y"), N("r"), T("color", "text")),
            "ring" => new Ring(N("x"), N("y"), N("r"), T("color", "text"), N("width", 1)),
            "text" => new Words(N("x"), N("y"), Text.Plain(T("text", "")), T("color", "text"), N("size", 10)),
            "image" => new Image(N("x"), N("y"), N("width"), N("height"), T("picture", "")),
            "link" => new Link(T("name", ""), N("x1"), N("y1"), N("x2"), N("y2"),
                sent["via"] is JsonArray via ? [.. via.Select(v => v is JsonValue n && n.TryGetValue<double>(out var d) ? (long)Math.Round(d) : 0L)] : [],
                T("color", "text"), N("width", 1)),
            "object" => new Object(sent["record"] is JsonValue r ? (r.TryGetValue<long>(out var id) ? id : r.TryGetValue<double>(out var d) ? (long)d : r.ToString()) : null, N("x"), N("y"), N("width"), N("height")),
            "ellipse" => new Ellipse(N("x"), N("y"), N("rx"), N("ry"), T("color", "text")),
            "path" => new Path(sent["points"] is JsonArray points ? [.. points.Select(v => v is JsonValue n && n.TryGetValue<double>(out var d) ? (long)Math.Round(d) : 0L)] : [], T("color", "text"), N("width", 1)),
            var op => throw new EngineException($"'{op}' is no shape: {sent.ToJsonString()}"),
        };
    }

    /// <summary>The shapes of a frame, in order.</summary>
    public static IReadOnlyList<Shape> All(JsonArray sent) => [.. sent.Select(s => Of(s ?? throw new EngineException("a shape is null")))];
}

