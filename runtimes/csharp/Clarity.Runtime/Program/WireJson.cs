// The runtime's values as the library's JSON reads them (clarity_runtime::ui,
// the web server's wire, DESIGN 7.3): a text with its slots or its key, a
// message, a part's state, a question. Values of the window are Clarity's
// JSON, as JsonValues writes them: names in snake case, decimals as exact
// text, enums by their names.

using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Clarity.Runtime;

internal static class WireJson
{
    /// <summary>A value as Clarity's JSON; null for what cannot be written.</summary>
    public static JsonNode? Value(object? value)
    {
        if (value is null)
            return null;
        if (value is Text text)
            return JsonValue.Create(Locale.Show(text));
        if (value is Choice choice)
            return Value(choice.Value);
        try
        {
            return JsonSerializer.SerializeToNode(value, value.GetType(), JsonValues.Options);
        }
        catch (Exception e) when (e is NotSupportedException or JsonException or InvalidOperationException or ArgumentException)
        {
            return null;
        }
    }

    /// <summary>JSON the library sent, as a value of <paramref name="type"/>.</summary>
    public static object? As(JsonNode? node, Type type)
    {
        var target = Nullable.GetUnderlyingType(type) ?? type;
        if (node is null)
            return null;
        if (target == typeof(object))
            return Plain(node);
        // A number the library keeps as a double goes to a whole number when the field is one.
        if (node is JsonValue v && v.TryGetValue<double>(out var d))
        {
            if (target == typeof(long))
                return (long)Math.Round(d);
            if (target == typeof(int))
                return (int)Math.Round(d);
            if (target == typeof(decimal))
                return (decimal)d;
            if (target == typeof(string))
                return d.ToString(CultureInfo.InvariantCulture);
        }
        // An empty text is no value for a field that may have none (a date not chosen).
        if (node is JsonValue s && s.TryGetValue<string>(out var str) && str.Length == 0 && target != typeof(string) && (Nullable.GetUnderlyingType(type) is not null || !type.IsValueType))
            return null;
        return node.Deserialize(type, JsonValues.Options);
    }

    /// <summary>JSON as plain values: text, a long or a double, a bool, lists and maps.</summary>
    public static object? Plain(JsonNode? node) => node switch
    {
        null => null,
        JsonValue v when v.TryGetValue<bool>(out var b) => b,
        JsonValue v when v.TryGetValue<long>(out var n) => n,
        JsonValue v when v.TryGetValue<double>(out var d) => d,
        JsonValue v when v.TryGetValue<string>(out var s) => s,
        JsonArray a => a.Select(Plain).ToList(),
        JsonObject o => o.ToDictionary(p => p.Key, p => Plain(p.Value)),
        _ => node.ToJsonString(),
    };

    // -- Texts ---------------------------------------------------------------------

    /// <summary>A text as the library reads it: a key with its slots, a template with slots, or plain text.</summary>
    public static JsonNode? Text(Text? text)
    {
        if (text is null)
            return null;
        if (text.Key is { Length: > 0 } key)
        {
            var keyed = new JsonObject { ["key"] = key };
            // Clarity's own texts carry their English: the library shows it where the project has no translation.
            if (text.Template.Length > 0 && text.Template != key)
                keyed["text"] = text.Template;
            if (text.Values.Count > 0)
                keyed["values"] = Slots(text);
            return keyed;
        }
        if (text.Values.Count == 0)
            return JsonValue.Create(text.Show());
        return new JsonObject { ["text"] = text.Template, ["values"] = Slots(text) };
    }

    static JsonObject Slots(Text text)
    {
        var slots = new JsonObject();
        foreach (var (path, slot) in text.Values)
        {
            var one = new JsonObject { ["kind"] = JsonNamingPolicy.CamelCase.ConvertName(slot.Kind.ToString()), ["value"] = SlotValue(slot.Value) };
            if (slot.Scale is { } scale)
                one["scale"] = scale;
            if (slot.Format is { } format)
                one["format"] = format;
            if (slot.EnumName is { } name)
                one["enum"] = name;
            slots[path] = one;
        }
        return slots;
    }

    static JsonNode? SlotValue(object? value) => value switch
    {
        null => null,
        bool b => b,
        long n => n,
        int n => n,
        decimal m => m.ToString(CultureInfo.InvariantCulture),
        double d => d.ToString(CultureInfo.InvariantCulture),
        DateOnly day => day.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture),
        DateTime moment => moment.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture),
        _ => Convert.ToString(value, CultureInfo.InvariantCulture),
    };

    /// <summary>A text the library sent: plain, keyed, or a template with slots.</summary>
    public static Text ReadText(JsonNode? node)
    {
        switch (node)
        {
            case JsonValue v when v.TryGetValue<string>(out var plain):
                return Clarity.Runtime.Text.Plain(plain);
            case JsonObject o when o["key"]?.GetValue<string>() is { } key:
                return Clarity.Runtime.Text.Of(key);
            case JsonObject o:
                return new Text(o["text"]?.GetValue<string>() ?? "");
            default:
                return Clarity.Runtime.Text.Plain("");
        }
    }

    /// <summary>What a handler said, as the library shows it.</summary>
    public static JsonObject Message(Message message) => new()
    {
        ["level"] = message.Level.ToString().ToLowerInvariant(),
        ["text"] = Text(message.Text),
    };

    /// <summary>An error the library shows: its text.</summary>
    public static JsonObject Error(Text text) => new() { ["error"] = Text(text) };

    /// <summary>An error as plain text.</summary>
    public static JsonObject Error(string text) => Error(Clarity.Runtime.Text.Plain(text));

    // -- Part states -----------------------------------------------------------------

    /// <summary>
    /// A part's state as the library reads it: what code may have set. A
    /// caption without text and a tab, step or section at 0 are left out,
    /// so the design's show.
    /// </summary>
    public static JsonObject Part(PartState part, IReadOnlyDictionary<string, object?>? values)
    {
        var state = new JsonObject
        {
            ["visible"] = part.IsVisible,
            ["enabled"] = part.IsEnabled,
            ["readonly"] = part.IsReadOnly,
            ["error"] = Text(part.ErrorText),
            ["folded"] = part.IsFolded,
            ["collapsed"] = part.IsCollapsed,
            ["maximized"] = part.IsMaximized,
            ["pinned"] = part.IsPinned,
            ["text"] = part.Text,
            ["checked"] = part.IsChecked,
        };
        if (part.CaptionText.Key is { Length: > 0 } || part.CaptionText.Template.Length > 0)
            state["caption"] = Text(part.CaptionText);
        if (part.Tab > 0)
            state["tab"] = part.Tab;
        if (part.Step > 0)
            state["step"] = part.Step;
        if (part.OpenSection > 0)
            state["open"] = part.OpenSection;
        if (part.Grid != 0)
            state["grid"] = part.Grid;
        if (part.Snap)
            state["snap"] = true;
        if (part.Connecting)
            state["connecting"] = true;
        if (!part.Resizable)
            state["resizable"] = false;
        if (part.Background.Length > 0)
            state["background"] = part.Background;
        if (part.DotSize != 2)
            state["dotSize"] = part.DotSize;
        if (part.DotColor.Length > 0)
            state["dotColor"] = part.DotColor;
        if (part.Tool.Length > 0)
            state["tool"] = part.Tool;
        if (part.Brush != 2)
            state["brush"] = part.Brush;
        if (part.Ink != "text")
            state["ink"] = part.Ink;
        if (part.Drawing.Count > 0)
            state["drawing"] = new JsonArray([.. part.Drawing.Select(Shape)]);
        if (part.Pointer is { } pointer)
            state["pointer"] = new JsonObject
            {
                ["x"] = pointer.X,
                ["y"] = pointer.Y,
                ["fromX"] = pointer.FromX,
                ["fromY"] = pointer.FromY,
                ["bent"] = pointer.Bent,
                ["via"] = new JsonArray([.. pointer.Via.Select(v => (JsonNode)v)]),
            };
        if (values is { Count: > 0 })
            state["values"] = new JsonObject(values.Select(kv => KeyValuePair.Create(kv.Key, Value(kv.Value))));
        return state;
    }

    /// <summary>A shape of a canvas's drawing, as conformance/drawing.json writes it: its op beside its fields.</summary>
    static JsonNode? Shape(Shape shape)
    {
        // A text's words go as a text the library shows in the person's language and format.
        if (shape is Shape.Words words)
            return new JsonObject
            {
                ["op"] = "text",
                ["x"] = words.X,
                ["y"] = words.Y,
                ["text"] = Text(words.Text),
                ["color"] = words.Color,
                ["size"] = words.Size,
                ["align"] = words.Align,
                ["style"] = words.Style,
            };
        // A link's label goes as a text too.
        if (shape is Shape.Link link)
            return new JsonObject
            {
                ["op"] = "link",
                ["name"] = link.Name,
                ["x1"] = link.X1,
                ["y1"] = link.Y1,
                ["x2"] = link.X2,
                ["y2"] = link.Y2,
                ["via"] = new JsonArray([.. link.Via.Select(v => (JsonNode)v)]),
                ["color"] = link.Color,
                ["width"] = link.Width,
                ["start"] = link.Start,
                ["end"] = link.End,
                ["label"] = link.Label is { } label ? Text(label) : "",
                ["style"] = link.Style,
            };
        if (Value(shape) is not JsonObject fields)
            return null;
        fields["op"] = JsonNamingPolicy.SnakeCaseLower.ConvertName(shape.GetType().Name);
        return fields;
    }

    /// <summary>
    /// What the screen changed of a part's state (an unbound input's text,
    /// a tick, a fold, a tab, a step, a section, a canvas's tool, a popover
    /// or slide-out the person closed or pinned) given back to the view
    /// model's state; what code alone sets (caption, error, enabled) stays
    /// the view model's.
    /// </summary>
    public static void ApplyPart(PartState part, JsonObject sent)
    {
        if (sent["visible"] is JsonValue shown && shown.TryGetValue<bool>(out var v) && v != part.IsVisible)
            part.IsVisible = v;
        if (sent["text"] is JsonValue text && text.TryGetValue<string>(out var t) && t != part.Text)
            part.Text = t;
        if (sent["checked"] is JsonValue ticked && ticked.TryGetValue<bool>(out var c) && c != part.IsChecked)
            part.IsChecked = c;
        if (sent["folded"] is JsonValue folded && folded.TryGetValue<bool>(out var f) && f != part.IsFolded)
            part.IsFolded = f;
        if (sent["collapsed"] is JsonValue collapsed && collapsed.TryGetValue<bool>(out var k) && k != part.IsCollapsed)
            part.IsCollapsed = k;
        if (sent["maximized"] is JsonValue maximized && maximized.TryGetValue<bool>(out var m) && m != part.IsMaximized)
            part.IsMaximized = m;
        if (sent["pinned"] is JsonValue pinned && pinned.TryGetValue<bool>(out var p) && p != part.IsPinned)
            part.IsPinned = p;
        if (Whole(sent["tab"]) is { } tab && tab != part.Tab)
            part.Tab = tab;
        if (Whole(sent["step"]) is { } step && step != part.Step)
            part.Step = step;
        if (Whole(sent["open"]) is { } open && open != part.OpenSection)
            part.OpenSection = open;
        if (sent["tool"] is JsonValue tool && tool.TryGetValue<string>(out var to) && to != part.Tool)
            part.Tool = to;
        if (Whole(sent["brush"]) is { } brush && brush != part.Brush)
            part.Brush = brush;
        if (sent["ink"] is JsonValue ink && ink.TryGetValue<string>(out var i) && i != part.Ink)
            part.Ink = i;
        // The pointer as the library names its fields (conformance/drawing.json): x, y, fromX, fromY, bent, via.
        if (sent["pointer"] is JsonObject pointer)
            part.Pointer = new Pointer(Whole(pointer["x"]) ?? 0, Whole(pointer["y"]) ?? 0)
            {
                FromX = Whole(pointer["fromX"]) ?? 0,
                FromY = Whole(pointer["fromY"]) ?? 0,
                Bent = pointer["bent"] is JsonValue bent && bent.TryGetValue<string>(out var name) ? name : "",
                Via = [.. (pointer["via"] as JsonArray ?? []).Select(v => Whole(v) ?? 0)],
            };
    }

    static long? Whole(JsonNode? node) => node is JsonValue v && v.TryGetValue<double>(out var d) ? (long)Math.Round(d) : null;
}
