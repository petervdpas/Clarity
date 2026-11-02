// A window's markup read as the parts Clarity.UI draws (DESIGN 7.8, 7.9):
// the file Generate writes for each window (`Windows/Main.cxaml`) and for
// each app its frame (`Apps/Shop.cxaml`). Each element is a part with what
// differs from its kind's defaults (the catalog, part-defaults.json), each
// value read by its field's type (part-fields.json), as the shared screen
// reads it (runtimes/web/src/screen/windowMarkup.ts).

using System.Text.Json.Nodes;
using System.Xml.Linq;

namespace Clarity.Ui;

/// <summary>A window's own frame (its regions as written, for the frame's parts to read), its parts, and its name (<c>Main</c>) by which the person's keys know its commands.</summary>
public sealed record WindowSpec(IReadOnlyList<XElement> Frame, IReadOnlyList<PartSpec> Parts, string? Name = null);

public static class WindowMarkup
{
    /// <summary>The words Clarity writes for a field of a part, where they differ from its name.</summary>
    static readonly Dictionary<string, string> PartWords = new() { ["key"] = "shortcut", ["taborder"] = "tabOrder", ["childList"] = "children", ["opens"] = "onClick" };

    static readonly Dictionary<string, string> CommandWords = new() { ["key"] = "shortcut", ["opens"] = "onClick", ["onclick"] = "handler" };

    static readonly string[] PageKinds = ["Tab", "Step", "Section", "Pane"];

    static readonly string[] Regions = ["Menu", "Toolbar", "Ribbon", "Sidebar", "Footer"];

    static readonly string[] Events = ["click", "change", "enter", "leave", "select", "open", "pointer", "key", "close", "return", "start", "submit", "draw", "drag", "drop", "bend", "next", "finish", "reorder", "tick", "unconnected"];

    static readonly Lazy<JsonObject> PartDefaults = new(() => Carried("part-defaults.json"));
    static readonly Lazy<JsonObject> PartFields = new(() => Carried("part-fields.json"));
    static readonly Lazy<JsonObject> CommandDefaults = new(() => Carried("command-defaults.json"));
    static readonly Lazy<JsonObject> CommandFields = new(() => Carried("command-fields.json"));
    static readonly Lazy<JsonObject> SidebarDefaultsBySide = new(() => Carried("sidebar-defaults.json"));
    static readonly Lazy<JsonObject> SidebarFields = new(() => Carried("sidebar-fields.json"));

    /// <summary>A new sidebar's fields on <paramref name="side"/> (<c>left</c>, <c>right</c>), as the catalog gives them.</summary>
    public static JsonObject SidebarDefaults(string side) =>
        (SidebarDefaultsBySide.Value[side] ?? SidebarDefaultsBySide.Value["left"])!.DeepClone().AsObject();

    /// <summary>A sidebar's fields: the catalog's for its side, with what its element writes, each read as the catalog says.</summary>
    public static JsonObject SidebarOf(XElement sidebar)
    {
        var props = SidebarDefaults((string?)sidebar.Attribute("side") ?? "left");
        foreach (var attribute in sidebar.Attributes())
        {
            var name = attribute.Name.LocalName;
            if (attribute.Value == "null")
                props[name] = null;
            else if (SidebarFields.Value[name] is JsonValue kind && ValueOf(kind.GetValue<string>(), attribute.Value) is { } value)
                props[name] = value;
        }
        return props;
    }

    static JsonObject Carried(string name)
    {
        using var stream = typeof(WindowMarkup).Assembly.GetManifestResourceStream($"Clarity.Ui.Catalog.{name}")
            ?? throw new InvalidOperationException($"Clarity.Ui carries no {name}");
        return JsonNode.Parse(stream)!.AsObject();
    }

    /// <summary>The kinds of part the catalog has.</summary>
    public static IEnumerable<string> Kinds => PartDefaults.Value.Select(p => p.Key);

    /// <summary>A window as its markup writes it; a file that is no well-formed XML is refused.</summary>
    public static WindowSpec Read(string markup)
    {
        var root = XDocument.Parse(markup).Root ?? throw new FormatException("the window's markup is empty");
        var elements = root.Elements().ToList();
        return new WindowSpec([.. elements.Where(e => Regions.Contains(e.Name.LocalName))], [.. elements.Where(e => !Regions.Contains(e.Name.LocalName)).Select(PartOf)], (string?)root.Attribute("name"));
    }

    /// <summary>An app's frame as its markup writes it: its regions.</summary>
    public static IReadOnlyList<XElement> ReadApp(string markup) =>
        [.. (XDocument.Parse(markup).Root ?? throw new FormatException("the app's markup is empty")).Elements()];

    /// <summary>A part as its element writes it.</summary>
    public static PartSpec PartOf(XElement element)
    {
        var tag = element.Name.LocalName;
        var kind = char.ToLowerInvariant(tag[0]) + tag[1..];
        if (PartDefaults.Value[kind] is not JsonObject defaults)
            throw new FormatException($"a window's markup has no part <{tag}>");
        var props = defaults.DeepClone().AsObject();
        // A part one row high writes no `rows`.
        props["height"] = 1;
        var handlers = new List<HandlerCall>();
        foreach (var attribute in element.Attributes())
        {
            var word = attribute.Name.LocalName;
            var value = attribute.Value;
            if (word == "at")
            {
                var at = value.Split(',');
                props["col"] = int.Parse(at[0]);
                props["row"] = int.Parse(at[1]);
            }
            else if (word == "span")
                props["width"] = int.Parse(value);
            else if (word == "rows")
                props["height"] = int.Parse(value);
            else if (word.StartsWith("on") && Events.Contains(word[2..]))
                handlers.Add(HandlerOf(word[2..], value));
            else if (word == "none")
                foreach (var empty in value.Split(' ', StringSplitOptions.RemoveEmptyEntries))
                    props[PartWords.GetValueOrDefault(empty, empty)] = null;
            else
            {
                var field = PartWords.GetValueOrDefault(word, word);
                props[field] = ValueOf(PartFields.Value[field]?.GetValue<string>(), value);
            }
        }
        var children = new List<PartSpec>();
        var pages = new List<PageSpec>();
        MenuSpec? menu = null;
        foreach (var child in element.Elements())
        {
            var name = child.Name.LocalName;
            if (name == "Menu")
                menu = MenuOf(child);
            else if (PageKinds.Contains(name))
                pages.Add(new PageSpec(name.ToLowerInvariant(), (string?)child.Attribute("title"), (int?)child.Attribute("span"), [.. child.Elements().Select(PartOf)]));
            else
                children.Add(PartOf(child));
        }
        return new PartSpec(kind, props, handlers, children, pages, menu);
    }

    public static MenuSpec MenuOf(XElement element) => new((string?)element.Attribute("name"), [.. element.Elements().Select(CommandOf)]);

    public static CommandSpec CommandOf(XElement element)
    {
        var props = CommandDefaults.Value.DeepClone().AsObject();
        foreach (var attribute in element.Attributes())
        {
            var field = CommandWords.GetValueOrDefault(attribute.Name.LocalName, attribute.Name.LocalName);
            props[field] = ValueOf(CommandFields.Value[field]?.GetValue<string>(), attribute.Value);
        }
        return new CommandSpec(props, [.. element.Elements().Select(CommandOf)]);
    }

    /// <summary>A value of a field, by its type: text as it is, a number, true or false, names, or JSON.</summary>
    static JsonNode? ValueOf(string? kind, string text)
    {
        switch (kind)
        {
            case "text":
                return text;
            case "number":
                return double.TryParse(text, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var n) ? n : null;
            case "bool":
                return text == "true";
            case "texts":
                return new JsonArray([.. text.Split(',').Select(t => t.Trim()).Where(t => t.Length > 0).Select(t => (JsonNode)t)]);
        }
        try
        {
            return JsonNode.Parse(text);
        }
        catch (System.Text.Json.JsonException)
        {
            return text;
        }
    }

    /// <summary><c>digit(7)</c> as the handler and the values it runs with, as Clarity writes a handler call.</summary>
    public static HandlerCall HandlerOf(string @event, string call)
    {
        var open = call.IndexOf('(');
        if (open < 0)
            return new HandlerCall(@event, call.Trim(), []);
        var inner = call[(open + 1)..call.LastIndexOf(')')];
        return new HandlerCall(@event, call[..open].Trim(), SplitValues(inner));
    }

    static List<string> SplitValues(string text)
    {
        var values = new List<string>();
        var current = new System.Text.StringBuilder();
        var quoted = false;
        for (var i = 0; i < text.Length; i++)
        {
            var c = text[i];
            if (c == '\\' && quoted && i + 1 < text.Length)
            {
                current.Append(c).Append(text[++i]);
            }
            else if (c == '"')
            {
                quoted = !quoted;
                current.Append(c);
            }
            else if (c == ',' && !quoted)
            {
                values.Add(current.ToString().Trim());
                current.Clear();
            }
            else
                current.Append(c);
        }
        if (current.ToString().Trim().Length > 0)
            values.Add(current.ToString().Trim());
        return values;
    }
}
