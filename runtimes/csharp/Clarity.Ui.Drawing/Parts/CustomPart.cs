// A custom part (DESIGN 5.19, 7.9): a control of Clarity.UI that a library
// of the program's own language brings, made the way the built-in parts are
// made. It draws itself in its place, takes the pointer, the wheel, keys
// and the focus like any element, and speaks with its window in the part's
// declared words: the value it binds and its properties come in (Set), what
// code tells it comes in (Hear), the person's change of its value goes out
// (ChangeValue, which runs `on change`) and so do its own events (Tell, which
// runs the handler wired to the tell). On C# it is a class deriving from
// CustomPart, named by the part's declaration with [PartKind("Gauge")]:
// Clarity writes one half of it from the declaration, the developer's
// adapter the other, and the window's <Custom part="Gauge"> finds it in the
// program's assemblies.

using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui;

public abstract class CustomPart : Part
{
    object? shownValue;
    bool? shownEnabled;
    bool started;

    /// <summary>A property (<c>value</c> for what it binds, <c>enabled</c>, or a declared one) as the window or code sets it.</summary>
    protected abstract void Set(string name, object? value);

    /// <summary>What code told it, with its values: <c>gauge.reset(1)</c>.</summary>
    protected abstract void Hear(string hear, object?[] values);

    /// <summary>
    /// Starts it in its window: the values written on its line and set by
    /// code so far, then what it binds and whether it is enabled, then what
    /// code told it before the window showed.
    /// </summary>
    internal void Start()
    {
        started = true;
        if (State is { } state)
        {
            foreach (var (name, value) in state.Values)
                Set(name, value);
        }
        Sync();
        State?.Listen(Set, Hear);
        // What it binds and whether it is enabled reach it when they change,
        // as Avalonia's bound properties did, not at the next layout.
        Host.Changed += Sync;
        if (State is { } own)
            own.PropertyChanged += (_, e) =>
            {
                if (e.PropertyName is nameof(PartState.IsEnabled) or null or "")
                    Sync();
            };
    }

    /// <summary>The value it binds and whether it is enabled, given again when they changed (when the view model or its state changes, and as its window lays it out).</summary>
    void Sync()
    {
        if (!started)
            return;
        if (Bound is { } bound)
        {
            var value = Host.Read(bound);
            if (shownEnabled is null || !Equals(value, shownValue))
            {
                shownValue = value;
                Set("value", value);
            }
        }
        if (shownEnabled != Enabled)
        {
            shownEnabled = Enabled;
            Set("enabled", Enabled);
        }
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        base.Arrange(box);
        Sync();
    }

    /// <summary>The person changed its value: what it binds takes it (one edit, which Ctrl+Z takes back) and <c>on change</c> runs.</summary>
    protected void ChangeValue(object? value)
    {
        if (Bound is { } bound)
        {
            shownValue = value;
            Host.Write(Spec.Name ?? "", bound, value);
        }
        Fire("change");
    }

    /// <summary>The part told something: the tell and its values, wired to a handler or not, as Avalonia's DotNetPart.Told was.</summary>
    public event Action<string, JsonElement[]>? Told;

    /// <summary>It tells <paramref name="tell"/> with its values: <see cref="Told"/> hears it, and the handler wired to it runs with them.</summary>
    protected void Tell(string tell, params object?[] values)
    {
        var told = values.Select(v => JsonDocument.Parse(PartValues.Write(v)).RootElement.Clone()).ToArray();
        Told?.Invoke(tell, told);
        if (Spec.Name is { } name && Wired(tell))
            Host.Told(name, tell, told);
    }

    /// <summary>Whether a handler is wired to <paramref name="tell"/> on its line: <c>on reached: gaugeReached</c>.</summary>
    bool Wired(string tell) =>
        Spec.Json("tells") is JsonArray wired && wired.Any(w => w?["tell"]?.GetValue<string>() == tell);

    /// <summary>A value as type <typeparamref name="T"/>, however the window or code gave it.</summary>
    protected static T As<T>(object? value) =>
        value is T typed ? typed : JsonSerializer.Deserialize<T>(PartValues.Write(value), PartValues.Json)!;
}

/// <summary>
/// The place of a custom part in its window (<c>&lt;Custom part="Gauge"&gt;</c>):
/// it holds the part the program's library brings, or says in its place
/// that the program carries no such part.
/// </summary>
[PartKind("custom")]
public sealed class CustomPlace : Part
{
    /// <summary>The part it holds: the library's, or one that says why there is none.</summary>
    public Part Inner { get; private set; } = null!;

    /// <summary>It grows when its declaration says <c>grows</c>, which its markup carries.</summary>
    public override bool Grows => Spec.Flag("grows");

    protected internal override void Made()
    {
        var kind = Spec.Text("part") ?? "";
        Part inner;
        try
        {
            inner = (Part?)CustomParts.Make(kind) ?? new MissingPart { Why = $"{Locale.Translate("customPartMissing", "the program carries no part")} {kind}" };
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            inner = new MissingPart { Why = $"{kind}: {e.InnerException?.Message ?? e.Message}" };
        }
        inner.Spec = Spec;
        inner.Host = Host;
        inner.Made();
        if (inner is CustomPart custom)
            custom.Start();
        Inner = inner;
        AddChild(inner);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        Inner.Arrange(box);
    }
}

/// <summary>The custom parts the program's libraries bring, found by the name their declaration gives them.</summary>
public static class CustomParts
{
    static readonly Dictionary<string, Func<CustomPart>> added = [];

    /// <summary>Adds a custom part by hand, beside those found in the program's assemblies.</summary>
    public static void Add(string kind, Func<CustomPart> make)
    {
        lock (added)
            added[kind] = make;
    }

    /// <summary>A new part of <paramref name="kind"/>; none when no assembly of the program has it.</summary>
    public static CustomPart? Make(string kind)
    {
        lock (added)
            if (added.TryGetValue(kind, out var make))
                return make();
        var type = AppDomain.CurrentDomain.GetAssemblies()
            .Where(a => !a.IsDynamic)
            .SelectMany(Types)
            .FirstOrDefault(t => !t.IsAbstract && typeof(CustomPart).IsAssignableFrom(t)
                && t.GetCustomAttribute<PartKindAttribute>()?.Kinds.Contains(kind) == true);
        return type is null ? null : (CustomPart)Activator.CreateInstance(type)!;
    }

    /// <summary>The types of an assembly, those that load when some do not.</summary>
    static IEnumerable<Type> Types(Assembly assembly)
    {
        try
        {
            return assembly.GetTypes();
        }
        catch (ReflectionTypeLoadException e)
        {
            return e.Types.OfType<Type>();
        }
    }
}
