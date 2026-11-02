// A part as its window's markup gives it (DESIGN 7.8): its kind, its fields
// (its kind's defaults with what the markup changes), its handlers, its
// parts, pages and menu. Read by WindowMarkup; a part module reads what it
// shows from here.

using System.Text.Json.Nodes;

namespace Clarity.Ui;

/// <summary>A handler a part runs on an event: <c>onclick="digit(7)"</c> is click, digit and the value 7, as written.</summary>
public sealed record HandlerCall(string Event, string Handler, IReadOnlyList<string> Args)
{
    /// <summary>How Clarity writes it: <c>digit(7)</c>, or <c>approve</c> without values.</summary>
    public string Written => Args.Count == 0 ? Handler : $"{Handler}({string.Join(", ", Args)})";
}

/// <summary>A page of a container: a tab, a step, a section, a pane (with its columns when it has a span).</summary>
public sealed record PageSpec(string Kind, string? Title, int? Columns, IReadOnlyList<PartSpec> Children);

/// <summary>An item of a menu: its fields (caption, name, key, handler or window it opens) and its items.</summary>
public sealed record CommandSpec(JsonObject Props, IReadOnlyList<CommandSpec> Items)
{
    public string? Text(string field) => Props[field] is JsonValue v && v.TryGetValue<string>(out var s) ? s : null;
    public bool Flag(string field) => Props[field] is JsonValue v && v.TryGetValue<bool>(out var b) && b;
}

public sealed record MenuSpec(string? Name, IReadOnlyList<CommandSpec> Items);

public sealed class PartSpec(string kind, JsonObject props, IReadOnlyList<HandlerCall> handlers, IReadOnlyList<PartSpec> children, IReadOnlyList<PageSpec> pages, MenuSpec? menu)
{
    /// <summary>Its kind, as the catalog names it: <c>button</c>, <c>textbox</c>, <c>treeTable</c>.</summary>
    public string Kind { get; } = kind;

    /// <summary>Every field of the part, its kind's defaults with what the markup changes.</summary>
    public JsonObject Props { get; } = props;

    public IReadOnlyList<HandlerCall> Handlers { get; } = handlers;
    public IReadOnlyList<PartSpec> Children { get; } = children;
    public IReadOnlyList<PageSpec> Pages { get; } = pages;
    public MenuSpec? Menu { get; } = menu;

    /// <summary>Its name, by which code and its part state know it; none for a part without one.</summary>
    public string? Name => Text("name");

    /// <summary>Where it is in its grid: column and row from 1, columns wide and rows high.</summary>
    public GridItem Place(bool grows) => new((int)(Number("col") ?? 1), (int)(Number("row") ?? 1), (int)(Number("width") ?? 1), (int)(Number("height") ?? 1), grows);

    /// <summary>A text field; null when it has none.</summary>
    public string? Text(string field) => Props[field] is JsonValue v && v.TryGetValue<string>(out var s) ? s : null;

    /// <summary>A number field; null when it has none.</summary>
    public double? Number(string field) =>
        Props[field] is JsonValue v && v.GetValueKind() == System.Text.Json.JsonValueKind.Number
            ? double.Parse(v.ToJsonString(), System.Globalization.CultureInfo.InvariantCulture)
            : null;

    /// <summary>A yes or no field; false when it has none.</summary>
    public bool Flag(string field) => Props[field] is JsonValue v && v.TryGetValue<bool>(out var b) && b;

    /// <summary>A field of names: <c>show="title, kind"</c>.</summary>
    public IReadOnlyList<string> Names(string field) =>
        Props[field] is JsonArray a ? [.. a.Select(n => n is JsonValue v && v.TryGetValue<string>(out var s) ? s : n?.ToJsonString() ?? "")] : [];

    /// <summary>A field as JSON: a list, a record, or a name of the model's.</summary>
    public JsonNode? Json(string field) => Props[field];

    /// <summary>The handlers it runs on <paramref name="event"/> (click, change, select...).</summary>
    public IEnumerable<HandlerCall> On(string @event) => Handlers.Where(h => h.Event == @event);
}
