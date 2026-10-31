// Canvas modes (DESIGN 5.31), decided here in the runtime, as every runtime
// decides them (conformance/canvas-modes.json): the screen says what the
// person did on a canvas (an object dropped, a stroke painted, a shape added,
// changed, moved or deleted) and the runtime changes the window's data,
// snapped to the canvas's grid, and chooses what it touched (its place from 0, as a
// table's chosen row). A vector
// drawing is kept as SVG text in one plain form, which every runtime writes
// and reads alike. The window applies the same decisions to its own records
// (WindowViewModel.CanvasEvent).

using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Avalonia;

/// <summary>A canvas's mode and the mode's parameters, as the action on it needs them.</summary>
/// <param name="Mode">objects, paint or vector.</param>
public sealed record CanvasArgs(string Mode)
{
    /// <summary>The data path of the list of objects (objects).</summary>
    public string List { get; init; } = "";

    /// <summary>The fields of an object's place, x and y (objects).</summary>
    public string[] At { get; init; } = ["x", "y"];

    /// <summary>The data path of the image field (paint) or the text field (vector).</summary>
    public string Path { get; init; } = "";

    /// <summary>A drawing's units, width and height; none: 1000 x 1000.</summary>
    public int[]? Units { get; init; }

    public long Grid { get; init; }

    public bool Snap { get; init; }

    public bool Readonly { get; init; }

    /// <summary>Where a place <paramref name="v"/> goes: to the nearest multiple of the grid with snap, else a whole number, half away from zero.</summary>
    public long Place(double v) =>
        Snap && Grid > 0 ? (long)Math.Round(v / Grid, MidpointRounding.AwayFromZero) * Grid : (long)Math.Round(v, MidpointRounding.AwayFromZero);
}

/// <summary>A vector canvas's drawing after an action: its SVG text, and the place of the shape it touched from 1 (0: none).</summary>
public sealed record VectorDrawn(string Text, int Selected);

public static class CanvasModes
{
    /// <summary>A drawing's units when the canvas names none.</summary>
    public static readonly int[] DefaultUnits = [1000, 1000];

    static string Head(int[] units) => $"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {units[0]} {units[1]}\">";

    /// <summary>A JSON number as a double; null for anything else.</summary>
    public static double? Number(JsonNode? node) =>
        node is JsonValue v && v.GetValueKind() == JsonValueKind.Number ? double.Parse(v.ToJsonString(), CultureInfo.InvariantCulture) : null;

    static string? Str(JsonNode? node) => node is JsonValue v && v.GetValueKind() == JsonValueKind.String ? v.GetValue<string>() : null;

    /// <summary>A part of an action, or a refusal to read it.</summary>
    static T Read<T>(JsonObject action, string name, Func<JsonNode?, T?> read) where T : struct =>
        read(action[name]) ?? throw new InvalidValueException($"a canvas's {name} does not fit: {action[name]?.ToJsonString()}");

    static int? Index(JsonNode? node) => Number(node) is { } n && n == Math.Floor(n) ? (int)n : null;

    /// <summary>
    /// Applies the person's action on a canvas to <paramref name="data"/> (the
    /// window's data as JSON) and says whether it changed anything; what it
    /// touched goes in <paramref name="selected"/> under <paramref name="part"/>.
    /// An action that changes nothing, or names no object or shape, is refused
    /// (false), as every action on a readonly canvas.
    /// </summary>
    public static bool Event(JsonObject data, JsonObject selected, string part, CanvasArgs a, JsonObject action)
    {
        if (a.Readonly)
            return false;
        switch (a.Mode)
        {
            case "objects":
                {
                    var (id, x, y) = Move(action);
                    if (ValueAt(data, a.List) is not JsonArray list)
                        throw new InvalidValueException($"the window has no list {a.List}");
                    for (var place = 0; place < list.Count; place++)
                    {
                        if (list[place] is not JsonObject record || !SameId(record["id"], id))
                            continue;
                        if (Placed(a, Number(record[a.At[0]]), Number(record[a.At[1]]), x, y) is not var (px, py))
                            return false;
                        record[a.At[0]] = px;
                        record[a.At[1]] = py;
                        selected[part] = place;
                        return true;
                    }
                    return false;
                }
            case "paint":
                {
                    var picture = Stroke(action);
                    if (JsonNode.DeepEquals(ValueAt(data, a.Path), picture))
                        return false;
                    // A stroke that is no picture is refused: the record never holds it.
                    if (!IsPicture(action))
                        return false;
                    SetValueAt(data, a.Path, picture.DeepClone());
                    selected.Remove(part);
                    return true;
                }
            case "vector":
                {
                    if (Vector(Str(ValueAt(data, a.Path)) ?? "", a, action) is not { } drawn)
                        return false;
                    SetValueAt(data, a.Path, drawn.Text);
                    if (drawn.Selected > 0)
                        selected[part] = drawn.Selected - 1;
                    else
                        selected.Remove(part);
                    return true;
                }
        }
        throw new InvalidValueException($"a canvas's mode is objects, paint or vector, not \"{a.Mode}\"");
    }

    /// <summary>An object's move: the id it names and where it was dropped.</summary>
    public static (JsonNode? Id, double X, double Y) Move(JsonObject action)
    {
        if (action["move"] is not JsonObject move)
            throw new InvalidValueException("an object is moved");
        return (move["id"], Read(move, "x", Number), Read(move, "y", Number));
    }

    /// <summary>An object's place after a drop at <paramref name="x"/>, <paramref name="y"/>; null when it is there already.</summary>
    public static (long X, long Y)? Placed(CanvasArgs a, double? wasX, double? wasY, double x, double y)
    {
        var (px, py) = (a.Place(x), a.Place(y));
        return wasX == px && wasY == py ? null : (px, py);
    }

    /// <summary>The picture a stroke leaves, as an image field holds a new one: <c>{"data": base64 of a PNG}</c> (conformance/pictures.json).</summary>
    public static JsonObject Stroke(JsonObject action) =>
        action.ContainsKey("stroke") ? action["stroke"] as JsonObject ?? throw new InvalidValueException($"a canvas's stroke does not fit: {action["stroke"]?.ToJsonString()}") : throw new InvalidValueException("a painted canvas is stroked");

    /// <summary>The PNG a stroke leaves, from its picture.</summary>
    /// <summary>Whether a stroke's data is a picture's bytes (conformance/pictures.json).</summary>
    public static bool IsPicture(JsonObject action)
    {
        try
        {
            return Pictures.KindOf(StrokeBytes(action)) is not null;
        }
        catch (InvalidValueException)
        {
            return false;
        }
    }

    public static byte[] StrokeBytes(JsonObject action)
    {
        try
        {
            return Convert.FromBase64String(Str(Stroke(action)["data"]) ?? throw new InvalidValueException("a stroke's picture has its data"));
        }
        catch (FormatException)
        {
            throw new InvalidValueException("a stroke's picture is base64");
        }
    }

    /// <summary>Whether an object's id is the id an action names: numbers by their value, texts as they are.</summary>
    public static bool SameId(JsonNode? id, JsonNode? named) =>
        (Number(id), Number(named)) is ({ } a, { } b) ? a == b : Str(id) is { } s && s == Str(named);

    /// <summary>A vector canvas's drawing <paramref name="text"/> after <paramref name="action"/>; null when it is refused.</summary>
    public static VectorDrawn? Vector(string text, CanvasArgs a, JsonObject action)
    {
        var (units, shapes) = DrawingShapes(text, a.Units ?? DefaultUnits);
        var selected = 0;
        if (action.ContainsKey("add"))
        {
            shapes.Add(Fit(action["add"] as JsonObject ?? throw new InvalidValueException("a canvas adds a shape"), a));
            selected = shapes.Count;
        }
        else if (action["change"] is JsonObject change)
        {
            var index = Read(change, "index", Index);
            if (index < 1 || index > shapes.Count)
                return null;
            shapes[index - 1] = Fit(change["shape"] as JsonObject ?? throw new InvalidValueException("a canvas changes a shape into a shape"), a);
            selected = index;
        }
        else if (action["move"] is JsonObject move)
        {
            var index = Read(move, "index", Index);
            if (index < 1 || index > shapes.Count || !Moved(shapes[index - 1], a.Place(Read(move, "x", Number)), a.Place(Read(move, "y", Number))))
                return null;
            selected = index;
        }
        else if (action.ContainsKey("delete"))
        {
            var index = Read(action, "delete", Index);
            if (index < 1 || index > shapes.Count)
                return null;
            shapes.RemoveAt(index - 1);
        }
        else
            throw new InvalidValueException("a vector canvas adds, changes, moves or deletes a shape");
        var drawn = DrawingText(units, shapes);
        return drawn == text ? null : new VectorDrawn(drawn, selected);
    }

    /// <summary>The numbers of each shape a vector canvas keeps, and which of them are places.</summary>
    static readonly Dictionary<string, (string[] Numbers, string[] Places)> ShapeFields = new()
    {
        ["rect"] = (["x", "y", "width", "height"], ["x", "y"]),
        ["line"] = (["x1", "y1", "x2", "y2", "width"], ["x1", "y1", "x2", "y2"]),
        ["ellipse"] = (["x", "y", "rx", "ry"], ["x", "y"]),
        ["path"] = (["width"], []),
        ["text"] = (["x", "y", "size"], ["x", "y"]),
    };

    /// <summary>What a shape's left-out numbers are (conformance/drawing.json).</summary>
    static readonly Dictionary<string, double> ShapeDefaults = new() { ["line.width"] = 1, ["path.width"] = 1, ["text.size"] = 10 };

    /// <summary>A shape as a vector canvas keeps it: whole numbers, places snapped.</summary>
    public static JsonObject Fit(JsonObject shape, CanvasArgs a)
    {
        var op = Str(shape["op"]) ?? "";
        if (!ShapeFields.TryGetValue(op, out var fields))
            throw new InvalidValueException($"a vector canvas keeps rect, line, ellipse, path and text, not \"{op}\"");
        var fitted = new JsonObject { ["op"] = op };
        foreach (var name in fields.Numbers)
        {
            var n = Number(shape[name]) ?? ShapeDefaults.GetValueOrDefault($"{op}.{name}");
            fitted[name] = fields.Places.Contains(name) ? a.Place(n) : (long)Math.Round(n, MidpointRounding.AwayFromZero);
        }
        fitted["color"] = Str(shape["color"]) ?? "";
        if (op == "path")
            fitted["points"] = new JsonArray([.. (shape["points"] as JsonArray ?? []).Select(p => (JsonNode)a.Place(Number(p) ?? 0))]);
        if (op == "text")
            fitted["text"] = Str(shape["text"]) ?? "";
        return fitted;
    }

    static long Whole(JsonNode? n) => (long)(Number(n) ?? 0);

    /// <summary>Moves a shape so its first place is at <paramref name="x"/>, <paramref name="y"/>, the rest with it;
    /// false when it is there already or is a line the runtime does not read.</summary>
    public static bool Moved(JsonObject shape, long x, long y)
    {
        long dx, dy;
        switch (Str(shape["op"]))
        {
            case "rect" or "ellipse" or "text":
                (dx, dy) = (x - Whole(shape["x"]), y - Whole(shape["y"]));
                (shape["x"], shape["y"]) = (x, y);
                break;
            case "line":
                (dx, dy) = (x - Whole(shape["x1"]), y - Whole(shape["y1"]));
                (shape["x1"], shape["y1"], shape["x2"], shape["y2"]) = (x, y, Whole(shape["x2"]) + dx, Whole(shape["y2"]) + dy);
                break;
            case "path":
                if (shape["points"] is not JsonArray points || points.Count < 2)
                    return false;
                (dx, dy) = (x - Whole(points[0]), y - Whole(points[1]));
                for (var i = 0; i < points.Count; i++)
                    points[i] = Whole(points[i]) + (i % 2 == 0 ? dx : dy);
                break;
            default:
                return false;
        }
        return dx != 0 || dy != 0;
    }

    static string Escaped(string text) => text.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;").Replace("\"", "&quot;");

    static readonly (string Entity, char Is)[] Entities = [("&lt;", '<'), ("&gt;", '>'), ("&quot;", '"'), ("&amp;", '&')];

    /// <summary>Entities as their characters, in one pass, so <c>&amp;amp;lt;</c> is <c>&amp;lt;</c>.</summary>
    static string Unescaped(string text)
    {
        var out_ = new StringBuilder();
        for (var i = 0; i < text.Length;)
        {
            var found = Entities.FirstOrDefault(e => string.CompareOrdinal(text, i, e.Entity, 0, e.Entity.Length) == 0);
            if (found.Entity is null)
                out_.Append(text[i++]);
            else
            {
                out_.Append(found.Is);
                i += found.Entity.Length;
            }
        }
        return out_.ToString();
    }

    /// <summary>A drawing's SVG text: its units and its shapes, one element on a line.</summary>
    public static string DrawingText(int[] units, IEnumerable<JsonObject> shapes)
    {
        var out_ = new StringBuilder(Head(units)).Append('\n');
        foreach (var shape in shapes)
            out_.Append(ShapeLine(shape)).Append('\n');
        return out_.Append("</svg>\n").ToString();
    }

    static string ShapeLine(JsonObject shape)
    {
        string N(string name) => ((long)Math.Round(Number(shape[name]) ?? 0, MidpointRounding.AwayFromZero)).ToString(CultureInfo.InvariantCulture);
        string T(string name) => Escaped(Str(shape[name]) ?? "");
        switch (Str(shape["op"]))
        {
            case "rect":
                return $"<rect x=\"{N("x")}\" y=\"{N("y")}\" width=\"{N("width")}\" height=\"{N("height")}\" fill=\"{T("color")}\"/>";
            case "line":
                return $"<line x1=\"{N("x1")}\" y1=\"{N("y1")}\" x2=\"{N("x2")}\" y2=\"{N("y2")}\" stroke=\"{T("color")}\" stroke-width=\"{N("width")}\"/>";
            case "ellipse":
                return $"<ellipse cx=\"{N("x")}\" cy=\"{N("y")}\" rx=\"{N("rx")}\" ry=\"{N("ry")}\" fill=\"{T("color")}\"/>";
            case "path":
                var points = shape["points"] as JsonArray ?? [];
                var pairs = new List<string>();
                for (var i = 0; i + 1 < points.Count; i += 2)
                    pairs.Add(FormattableString.Invariant($"{(long)Math.Round(Number(points[i]) ?? 0, MidpointRounding.AwayFromZero)},{(long)Math.Round(Number(points[i + 1]) ?? 0, MidpointRounding.AwayFromZero)}"));
                return $"<polyline points=\"{string.Join(' ', pairs)}\" fill=\"none\" stroke=\"{T("color")}\" stroke-width=\"{N("width")}\"/>";
            case "text":
                return $"<text x=\"{N("x")}\" y=\"{N("y")}\" fill=\"{T("color")}\" font-size=\"{N("size")}\">{T("text")}</text>";
        }
        return Str(shape["text"]) ?? "";
    }

    /// <summary>
    /// Reads a drawing's SVG text: its units (<paramref name="units"/> when it
    /// names none) and its shapes. A line it does not read as a shape is kept as it is.
    /// </summary>
    public static (int[] Units, List<JsonObject> Shapes) DrawingShapes(string text, int[] units)
    {
        var shapes = new List<JsonObject>();
        foreach (var line in text.Split('\n'))
        {
            if (line.Trim() == "" || line == "</svg>")
                continue;
            if (line.StartsWith("<svg", StringComparison.Ordinal))
            {
                if (ViewBox(line) is { } box)
                    units = box;
                continue;
            }
            shapes.Add(ReadShape(line));
        }
        return (units, shapes);
    }

    /// <summary>The units of the svg line, when it is the line every runtime writes.</summary>
    static int[]? ViewBox(string line)
    {
        const string start = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 ";
        if (!line.StartsWith(start, StringComparison.Ordinal) || !line.EndsWith("\">", StringComparison.Ordinal))
            return null;
        var sizes = line[start.Length..^2].Split(' ');
        return sizes.Length == 2 && int.TryParse(sizes[0], CultureInfo.InvariantCulture, out var w) && int.TryParse(sizes[1], CultureInfo.InvariantCulture, out var h) ? [w, h] : null;
    }

    /// <summary>The shape of one line of a drawing: written back, it must be the line again, or the line is kept as it is.</summary>
    static JsonObject ReadShape(string line)
    {
        var kept = new JsonObject { ["op"] = "svg", ["text"] = line };
        if (SvgElement(line) is not var (name, attrs, content))
            return kept;
        var shape = new JsonObject();
        bool Pick(params string[] pairs)
        {
            for (var i = 0; i < pairs.Length; i += 2)
            {
                if (!long.TryParse(attrs.GetValueOrDefault(pairs[i]), NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n))
                    return false;
                shape[pairs[i + 1]] = n;
            }
            return true;
        }
        var fits = false;
        switch (name)
        {
            case "rect":
                (shape["op"], shape["color"]) = ("rect", attrs.GetValueOrDefault("fill", ""));
                fits = Pick("x", "x", "y", "y", "width", "width", "height", "height");
                break;
            case "line":
                (shape["op"], shape["color"]) = ("line", attrs.GetValueOrDefault("stroke", ""));
                fits = Pick("x1", "x1", "y1", "y1", "x2", "x2", "y2", "y2", "stroke-width", "width");
                break;
            case "ellipse":
                (shape["op"], shape["color"]) = ("ellipse", attrs.GetValueOrDefault("fill", ""));
                fits = Pick("cx", "x", "cy", "y", "rx", "rx", "ry", "ry");
                break;
            case "polyline":
                (shape["op"], shape["color"]) = ("path", attrs.GetValueOrDefault("stroke", ""));
                fits = Pick("stroke-width", "width");
                var points = new JsonArray();
                foreach (var pair in attrs.GetValueOrDefault("points", "").Split(' ', StringSplitOptions.RemoveEmptyEntries))
                {
                    var xy = pair.Split(',', 2);
                    if (xy.Length < 2 || !long.TryParse(xy[0], NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var x) || !long.TryParse(xy[1], NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var y))
                    {
                        fits = false;
                        break;
                    }
                    points.Add(x);
                    points.Add(y);
                }
                shape["points"] = points;
                break;
            case "text":
                (shape["op"], shape["color"], shape["text"]) = ("text", attrs.GetValueOrDefault("fill", ""), Unescaped(content));
                fits = Pick("x", "x", "y", "y", "font-size", "size");
                break;
        }
        return fits && ShapeLine(shape) == line ? shape : kept;
    }

    /// <summary>Reads <c>&lt;name a="1" b="2"/&gt;</c> or <c>&lt;name a="1"&gt;content&lt;/name&gt;</c>.</summary>
    static (string Name, Dictionary<string, string> Attrs, string Content)? SvgElement(string line)
    {
        if (!line.StartsWith('<'))
            return null;
        var rest = line[1..];
        var end = rest.IndexOfAny([' ', '/', '>']);
        if (end <= 0)
            return null;
        var name = rest[..end];
        rest = rest[end..];
        var attrs = new Dictionary<string, string>();
        while (true)
        {
            rest = rest.TrimStart(' ');
            if (rest.StartsWith("/>", StringComparison.Ordinal))
                return rest == "/>" ? (name, attrs, "") : null;
            if (rest.StartsWith('>'))
            {
                var close = $"</{name}>";
                return rest.EndsWith(close, StringComparison.Ordinal) && rest.Length > close.Length ? (name, attrs, rest[1..^close.Length]) : rest == ">" + close ? (name, attrs, "") : null;
            }
            var eq = rest.IndexOf("=\"", StringComparison.Ordinal);
            if (eq <= 0)
                return null;
            var quote = rest.IndexOf('"', eq + 2);
            if (quote < 0)
                return null;
            attrs[rest[..eq]] = Unescaped(rest[(eq + 2)..quote]);
            rest = rest[(quote + 1)..];
        }
    }

    /// <summary>The value at a data path (<c>note.picture</c>) of the window's data as JSON.</summary>
    public static JsonNode? ValueAt(JsonObject data, string path)
    {
        JsonNode? at = data;
        foreach (var name in path.Split('.'))
            at = at is JsonObject o ? o[name] : null;
        return at;
    }

    static void SetValueAt(JsonObject data, string path, JsonNode? value)
    {
        var names = path.Split('.');
        JsonNode? at = data;
        foreach (var name in names[..^1])
            at = at is JsonObject o ? o[name] : null;
        if (at is not JsonObject owner)
            throw new InvalidValueException($"the window has no {path}");
        owner[names[^1]] = value;
    }
}
