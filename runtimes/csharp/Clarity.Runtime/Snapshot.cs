// A window's state as JSON: what a window's undo keeps and takes back. The
// state is the window's own properties that hold something (those with a setter, and those set once by the constructor), and each part's
// state; computed ones, such as the title, are left to each side. A
// selected row crosses as a row and is matched back to the row of the
// window's own list, so a grid keeps it selected.

using System.Collections;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Runtime;

public static class Snapshot
{
    /// <summary>Values as the snapshot writes them: Clarity's JSON, with texts, shapes and values of any type.</summary>
    public static readonly JsonSerializerOptions Options = Build();

    static JsonSerializerOptions Build()
    {
        var options = new JsonSerializerOptions(JsonValues.Options);
        options.Converters.Add(new TextConverter());
        options.Converters.Add(new ShapeConverter());
        options.Converters.Add(new AnyConverter());
        return options;
    }

    /// <summary>
    /// The state of <paramref name="window"/>: <c>{"data": ..., "parts": ...}</c>.
    /// </summary>
    public static JsonObject Of(WindowViewModel window)
    {
        var data = new JsonObject();
        var parts = new JsonObject();
        foreach (var property in Members(window.GetType()))
        {
            var value = property.GetValue(window);
            if (value is PartState part)
                parts[property.Name] = Part(part);
            else
                data[property.Name] = JsonSerializer.SerializeToNode(value, property.PropertyType, Options);
        }
        return new JsonObject { ["data"] = data, ["parts"] = parts };
    }

    /// <summary>Gives <paramref name="window"/> the state <paramref name="state"/>: its data, then its parts.</summary>
    public static void Apply(WindowViewModel window, JsonObject state)
    {
        var members = Members(window.GetType()).ToDictionary(p => p.Name);
        if (state["data"] is JsonObject data)
        {
            foreach (var (name, node) in data)
            {
                if (!members.TryGetValue(name, out var property) || property.PropertyType == typeof(PartState))
                    continue;
                var value = node?.Deserialize(property.PropertyType, Options);
                if (property.GetSetMethod(true) is { } set)
                    set.Invoke(window, [value]);
                else
                    BackingField(property)?.SetValue(window, value);
            }
        }
        if (state["parts"] is JsonObject parts)
        {
            foreach (var (name, node) in parts)
            {
                if (members.TryGetValue(name, out var property) && property.GetValue(window) is PartState part && node is JsonObject sent)
                    ApplyPart(window, part, sent);
            }
        }
    }

    /// <summary>The window's parts, each by the name its state goes by.</summary>
    internal static IEnumerable<(string Name, PartState Part)> PartsOf(WindowViewModel window) =>
        Members(window.GetType()).Where(p => p.PropertyType == typeof(PartState)).Select(p => (p.Name, (PartState)p.GetValue(window)!));

    /// <summary>The window's own properties that hold its state.</summary>
    static IEnumerable<PropertyInfo> Members(Type type) =>
        type.GetProperties(BindingFlags.Instance | BindingFlags.Public)
            .Where(p => p.DeclaringType is { } d && d != typeof(WindowViewModel) && d.IsSubclassOf(typeof(WindowViewModel)))
            .Where(p => p.CanRead && p.GetIndexParameters().Length == 0)
            .Where(p => p.PropertyType == typeof(PartState) || p.GetSetMethod(true) is not null || BackingField(p) is not null);

    /// <summary>The field of a property set once, by its constructor.</summary>
    static FieldInfo? BackingField(PropertyInfo property) =>
        property.DeclaringType?.GetField($"<{property.Name}>k__BackingField", BindingFlags.Instance | BindingFlags.NonPublic);

    static JsonObject Part(PartState part) => new()
    {
        ["visible"] = part.IsVisible,
        ["enabled"] = part.IsEnabled,
        ["read_only"] = part.IsReadOnly,
        ["caption"] = JsonSerializer.SerializeToNode(part.CaptionText, Options),
        ["error"] = JsonSerializer.SerializeToNode(part.ErrorText, Options),
        ["text"] = part.Text,
        ["checked"] = part.IsChecked,
        ["folded"] = part.IsFolded,
        ["collapsed"] = part.IsCollapsed,
        ["maximized"] = part.IsMaximized,
        ["tab"] = part.Tab,
        ["step"] = part.Step,
        ["open_section"] = part.OpenSection,
        ["pointer"] = JsonSerializer.SerializeToNode(part.Pointer, Options),
        ["drawing"] = JsonSerializer.SerializeToNode(part.Drawing, Options),
        ["selected"] = JsonSerializer.SerializeToNode(part.SelectedItem, typeof(object), Options),
    };

    static void ApplyPart(WindowViewModel window, PartState part, JsonObject sent)
    {
        T Get<T>(string name, T otherwise) => sent[name] is { } node ? node.Deserialize<T>(Options) ?? otherwise : otherwise;
        part.IsVisible = Get("visible", part.IsVisible);
        part.IsEnabled = Get("enabled", part.IsEnabled);
        part.IsReadOnly = Get("read_only", part.IsReadOnly);
        part.CaptionText = Get("caption", part.CaptionText);
        part.ErrorText = Get("error", part.ErrorText);
        part.Text = Get("text", part.Text);
        part.IsChecked = Get("checked", part.IsChecked);
        part.IsFolded = Get("folded", part.IsFolded);
        part.IsCollapsed = Get("collapsed", part.IsCollapsed);
        part.IsMaximized = Get("maximized", part.IsMaximized);
        part.Tab = Get("tab", part.Tab);
        part.Step = Get("step", part.Step);
        part.OpenSection = Get("open_section", part.OpenSection);
        part.Pointer = sent["pointer"]?.Deserialize<Pointer>(Options);
        part.ShowDrawing(Get<List<Shape>>("drawing", [.. part.Drawing]));
        part.SelectedItem = Matched(window, sent["selected"]?.Deserialize<object>(Options));
    }

    /// <summary>The row of one of the window's lists that <paramref name="row"/> is: the same record, by its id.</summary>
    static object? Matched(WindowViewModel window, object? row)
    {
        if (row is null)
            return null;
        var id = row.GetType().GetProperty("Id");
        foreach (var property in Members(window.GetType()))
        {
            if (property.PropertyType == typeof(PartState) || property.PropertyType == typeof(string) || property.GetValue(window) is not IEnumerable list)
                continue;
            foreach (var item in list)
            {
                if (item is null || item.GetType() != row.GetType())
                    continue;
                if (id is not null ? Equals(id.GetValue(item), id.GetValue(row)) : Equals(item, row))
                    return item;
            }
        }
        return row;
    }

    /// <summary>A text: its template, its key and the values of its slots.</summary>
    sealed class TextConverter : JsonConverter<Text>
    {
        public override Text Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options)
        {
            var node = JsonNode.Parse(ref reader) as JsonObject ?? throw new JsonException("a text is an object");
            var text = new Text(node["template"]?.GetValue<string>() ?? "") { Key = node["key"]?.GetValue<string>() };
            if (node["values"] is JsonObject values)
            {
                foreach (var (path, slot) in values)
                {
                    if (slot is not JsonObject s)
                        continue;
                    var kind = Enum.Parse<SlotKind>(s["kind"]!.GetValue<string>());
                    object? value = s["value"] switch
                    {
                        null => null,
                        JsonValue v when v.TryGetValue<bool>(out var b) => b,
                        JsonValue v when v.TryGetValue<long>(out var n) => n,
                        JsonValue v => v.GetValue<string>(),
                        _ => null,
                    };
                    text.Values[path] = new Slot(kind, value, s["scale"]?.GetValue<int>(), s["format"]?.GetValue<string>(), s["enum_name"]?.GetValue<string>());
                }
            }
            return text;
        }

        public override void Write(Utf8JsonWriter writer, Text text, JsonSerializerOptions options)
        {
            var values = new JsonObject();
            foreach (var (path, slot) in text.Values)
            {
                values[path] = new JsonObject
                {
                    ["kind"] = slot.Kind.ToString(),
                    ["value"] = slot.Value switch { null => null, bool b => b, long n => n, var v => v.ToString() },
                    ["scale"] = slot.Scale,
                    ["format"] = slot.Format,
                    ["enum_name"] = slot.EnumName,
                };
            }
            new JsonObject { ["template"] = text.Template, ["key"] = text.Key, ["values"] = values }.WriteTo(writer);
        }
    }

    /// <summary>A shape, by its kind.</summary>
    sealed class ShapeConverter : JsonConverter<Shape>
    {
        public override bool CanConvert(Type type) => type == typeof(Shape);

        public override Shape Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options)
        {
            var node = JsonNode.Parse(ref reader) as JsonObject ?? throw new JsonException("a shape is an object");
            var kind = typeof(Shape).GetNestedType(node["kind"]?.GetValue<string>() ?? "") ?? throw new JsonException($"no shape {node["kind"]}");
            return (Shape)node["shape"]!.Deserialize(kind, options)!;
        }

        public override void Write(Utf8JsonWriter writer, Shape shape, JsonSerializerOptions options) =>
            new JsonObject { ["kind"] = shape.GetType().Name, ["shape"] = JsonSerializer.SerializeToNode(shape, shape.GetType(), options) }.WriteTo(writer);
    }

    /// <summary>A value of any type, such as a choice's or a selected row: with its type, so it comes back as it was.</summary>
    sealed class AnyConverter : JsonConverter<object>
    {
        public override object? Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options)
        {
            var node = JsonNode.Parse(ref reader) as JsonObject ?? throw new JsonException("a value is an object with its type");
            var of = Type.GetType(node["type"]?.GetValue<string>() ?? "") ?? throw new JsonException($"no type {node["type"]}");
            return node["value"]?.Deserialize(of, options);
        }

        public override void Write(Utf8JsonWriter writer, object value, JsonSerializerOptions options) =>
            new JsonObject { ["type"] = value.GetType().AssemblyQualifiedName, ["value"] = JsonSerializer.SerializeToNode(value, value.GetType(), options) }.WriteTo(writer);
    }
}
