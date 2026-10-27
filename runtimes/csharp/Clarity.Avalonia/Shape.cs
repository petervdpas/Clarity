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
            var op => throw new EngineException($"'{op}' is no shape: {sent.ToJsonString()}"),
        };
    }

    /// <summary>The shapes of a frame, in order.</summary>
    public static IReadOnlyList<Shape> All(JsonArray sent) => [.. sent.Select(s => Of(s ?? throw new EngineException("a shape is null")))];
}

