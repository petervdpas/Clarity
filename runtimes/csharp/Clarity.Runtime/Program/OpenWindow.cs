// One window the library shows, as the program keeps it: its view model (the
// truth of its values), what its markup binds, and what a handler asked of
// the screen while it ran (messages, windows to open, closing, questions,
// the clipboard, hears for custom parts). Its state goes to the library as
// the runtime's WindowState: the view model's values in Clarity's JSON, each
// part's state, and the chosen row of each list; what the library sends back
// is given to the view model where it differs from what it last had.

using System.Collections;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Reflection;
using System.Text.Json.Nodes;
using System.Windows.Input;

namespace Clarity.Runtime;

internal sealed class OpenWindow : IDisposable
{
    readonly Dictionary<string, PartState> states = [];
    readonly Dictionary<string, Dictionary<string, object?>> values = [];
    readonly List<JsonObject> hears = [];
    readonly List<Message> said = [];
    JsonObject lastData = [];

    public OpenWindow(WindowViewModel model, string name, WindowMarkup markup)
    {
        Model = model;
        Name = name;
        Markup = markup;
        foreach (var property in model.GetType().GetProperties(BindingFlags.Public | BindingFlags.Instance))
            if (property.PropertyType == typeof(PartState) && property.GetIndexParameters().Length == 0 && property.GetValue(model) is PartState state)
            {
                states.TryAdd(state.Name, state);
                var own = new Dictionary<string, object?>(state.Values);
                values[state.Name] = own;
                var part = state.Name;
                // What code sets on a custom part and tells it goes to the library with the next reply.
                state.Listen((name, value) => own[name] = value, (hear, given) => hears.Add(new JsonObject
                {
                    ["part"] = part,
                    ["hear"] = hear,
                    ["values"] = new JsonArray([.. given.Select(WireJson.Value)]),
                }));
            }
        model.Messages.CollectionChanged += Said;
        model.OpenRequested += Opened;
        model.CloseRequested += Closed;
        model.AskRequested += Asked;
        model.CopyRequested += Copied;
    }

    public WindowViewModel Model { get; }

    /// <summary>The window's name: MainViewModel is Main.</summary>
    public string Name { get; }

    public WindowMarkup Markup { get; }

    public long Instance => Model.Instance;

    /// <summary>When the library last asked about the window.</summary>
    public DateTime Used { get; set; }

    /// <summary>The windows a handler opened, with whether in a window of their own.</summary>
    public List<(WindowViewModel Window, bool NewWindow)> Opens { get; } = [];

    public bool Close { get; set; }

    public List<Question> Asks { get; } = [];

    public Text? Copy { get; set; }

    void Said(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (e.NewItems is { } added)
            said.AddRange(added.OfType<Message>());
    }

    void Opened(WindowViewModel other, bool newWindow) => Opens.Add((other, newWindow));

    void Closed() => Close = true;

    void Asked(Question question) => Asks.Add(question);

    void Copied(Text text) => Copy = text;

    /// <summary>What was said since the last reply, taken.</summary>
    public List<Message> TakeSaid()
    {
        var taken = said.ToList();
        said.Clear();
        // The window's own list keeps nothing the library shows already.
        Model.Messages.CollectionChanged -= Said;
        Model.Messages.Clear();
        Model.Messages.CollectionChanged += Said;
        return taken;
    }

    /// <summary>The hears given to custom parts since the last reply, taken.</summary>
    public JsonArray TakeHears()
    {
        var taken = new JsonArray([.. hears]);
        hears.Clear();
        return taken;
    }

    public PartState? State(string name) => states.GetValueOrDefault(name);

    // -- The state --------------------------------------------------------------

    /// <summary>The window's state as the library reads it (WindowState).</summary>
    public JsonObject State()
    {
        lastData = Data();
        var parts = new JsonObject();
        foreach (var (name, state) in states)
            parts[name] = WireJson.Part(state, values.GetValueOrDefault(name));
        var selected = new JsonObject();
        foreach (var (name, state) in states)
            if (Place(name, state.SelectedItem) is { } place)
                selected[name] = (long)place;
        return new JsonObject
        {
            ["window"] = Name,
            ["instance"] = Instance,
            ["data"] = lastData.DeepClone(),
            ["parts"] = parts,
            ["selected"] = selected,
        };
    }

    /// <summary>
    /// The view model's values by their Clarity names: every property its
    /// class declares that holds a value (a record, a list, a text, a
    /// number, the settings), not its parts' states, commands or choices.
    /// </summary>
    JsonObject Data()
    {
        var data = new JsonObject();
        foreach (var property in Shown(Model.GetType()))
        {
            object? value;
            try
            {
                value = property.GetValue(Model);
            }
            catch (TargetInvocationException)
            {
                continue;
            }
            data[Members.Named(property.Name)] = WireJson.Value(value);
        }
        return data;
    }

    static IEnumerable<PropertyInfo> Shown(Type type) =>
        type.GetProperties(BindingFlags.Public | BindingFlags.Instance)
            .Where(p => p.DeclaringType is { } d && d != typeof(WindowViewModel) && d.IsSubclassOf(typeof(WindowViewModel)))
            // What every window has (its title, size, whether it is a dialog) is no value of its data.
            .Where(p => p.CanRead && p.GetIndexParameters().Length == 0 && p.GetMethod?.GetBaseDefinition().DeclaringType is { } own && own != typeof(WindowViewModel))
            .Where(p => p.PropertyType != typeof(PartState) && p.PropertyType != typeof(Choice[]))
            .Where(p => !typeof(ICommand).IsAssignableFrom(p.PropertyType) && !typeof(Delegate).IsAssignableFrom(p.PropertyType));

    /// <summary>
    /// Gives the view model what the library sent: each value that differs
    /// from what it last had (at the deepest place it differs, so a record
    /// in a list stays the same record), each part's state, each chosen row.
    /// </summary>
    public void Apply(JsonObject? state)
    {
        if (state is null)
            return;
        if (state["data"] is JsonObject data)
        {
            foreach (var (name, node) in data)
            {
                if (JsonNode.DeepEquals(node, lastData[name]))
                    continue;
                ApplyAt(Model, name, node, lastData[name]);
            }
        }
        if (state["parts"] is JsonObject parts)
            foreach (var (name, node) in parts)
                if (node is JsonObject sent && states.TryGetValue(name, out var part))
                    WireJson.ApplyPart(part, sent);
        var chosen = state["selected"] as JsonObject;
        foreach (var (name, part) in states)
        {
            if (Markup.Bind(name) is not { } bind || Members.Read(Model, bind) is not IList list)
                continue;
            object? row = chosen?[name] is JsonValue v && v.TryGetValue<double>(out var place) && place >= 0 && place < list.Count ? list[(int)place] : null;
            if (!ReferenceEquals(row, part.SelectedItem))
                part.SelectedItem = row;
        }
        lastData = Data();
    }

    /// <summary>Gives <paramref name="owner"/>'s member <paramref name="name"/> the value <paramref name="node"/>, going into records and lists where they hold the same shape.</summary>
    static void ApplyAt(object owner, string name, JsonNode? node, JsonNode? was)
    {
        if (Members.Member(owner.GetType(), name) is not { } property)
            return;
        var current = property.GetValue(owner);
        if (Into(current, node, was))
            return;
        if (!property.CanWrite)
            return;
        try
        {
            var value = WireJson.As(node, property.PropertyType);
            if (!Equals(current, value))
                property.SetValue(owner, value);
        }
        catch (Exception e) when (e is System.Text.Json.JsonException or FormatException or InvalidCastException or NotSupportedException or ArgumentException or OverflowException)
        {
            Console.Error.WriteLine($"{name} cannot take {node?.ToJsonString()}: {e.Message}");
        }
    }

    /// <summary>Goes into a record or a list of the same length and gives each differing member its value; false when it is no such thing.</summary>
    static bool Into(object? current, JsonNode? node, JsonNode? was)
    {
        switch (current, node)
        {
            case (IList list, JsonArray rows) when list.Count == rows.Count && was is JsonArray before && before.Count == rows.Count:
                for (var i = 0; i < rows.Count; i++)
                    if (!JsonNode.DeepEquals(rows[i], before[i]) && !Into(list[i], rows[i], before[i]))
                        return false;
                return true;
            case (not null and not string and not IEnumerable, JsonObject fields) when !current.GetType().IsValueType:
                var old = was as JsonObject;
                foreach (var (field, value) in fields)
                    if (!JsonNode.DeepEquals(value, old?[field]))
                        ApplyAt(current, field, value, old?[field]);
                return true;
            default:
                return false;
        }
    }

    /// <summary>The place of <paramref name="row"/> in the list part <paramref name="name"/> binds; none when it is in none.</summary>
    int? Place(string name, object? row)
    {
        if (row is null || Markup.Bind(name) is not { } bind || Members.Read(Model, bind) is not IList list)
            return null;
        var place = list.IndexOf(row);
        return place >= 0 ? place : null;
    }

    public void Dispose()
    {
        Model.Messages.CollectionChanged -= Said;
        Model.OpenRequested -= Opened;
        Model.CloseRequested -= Closed;
        Model.AskRequested -= Asked;
        Model.CopyRequested -= Copied;
        foreach (var state in states.Values)
            state.StopListening();
    }
}
