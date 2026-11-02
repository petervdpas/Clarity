// A window's view model as the parts' tests give it: values by path, lists,
// part states, choices and texts in dictionaries, and a log of what the
// parts did (each write and each handler run; apart, each edit of a part's own
// state and each address opened). Its fields' facts, resources and icons are
// dictionaries too. Shared by every part's tests; a test that needs more
// makes a subclass of its own.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class TestHost : IPartHost
{
    public readonly Dictionary<string, object?> Values = [];
    public readonly Dictionary<string, List<object>> Lists = [];
    public readonly Dictionary<string, PartState> States = [];
    public readonly Dictionary<string, List<Choice>> Offered = [];
    public readonly Dictionary<string, string> Texts = [];
    public readonly Dictionary<string, FieldFacts> Fields = [];
    public readonly Dictionary<string, byte[]> Resources = [];
    public readonly Dictionary<string, string> Icons = [];
    public readonly HashSet<string> Refusing = [];
    public readonly List<string> Log = [];
    public readonly List<string> Edits = [];
    public readonly List<string> Opened = [];

    public virtual object? Read(string path) => Values.GetValueOrDefault(path);

    public virtual IReadOnlyList<object> Rows(string path) => Lists.GetValueOrDefault(path) ?? [];

    public virtual void Write(string part, string path, object? value)
    {
        Values[path] = value;
        Log.Add($"write {part} {path} {value}");
        Changed?.Invoke();
    }

    /// <summary>A named part gets a state the first time it is asked for, as a generated view model has one per named part.</summary>
    public virtual PartState? State(string? name)
    {
        if (name is null)
            return null;
        if (!States.TryGetValue(name, out var state))
            States[name] = state = new PartState(name);
        return state;
    }

    /// <summary>Logs the run; a handler named in <see cref="Refusing"/> refuses.</summary>
    public virtual bool Run(string? part, HandlerCall call)
    {
        Log.Add($"run {part} {call.Event} {call.Written}");
        return !Refusing.Contains(call.Handler);
    }

    public virtual void Edited(string part) => Edits.Add(part);

    /// <summary>The facts of a field by its path as written, else by the path without a row's place (<c>tasks[0].done</c> as <c>tasks.done</c>).</summary>
    public virtual FieldFacts? Field(string path) =>
        Fields.GetValueOrDefault(path) ?? Fields.GetValueOrDefault(System.Text.RegularExpressions.Regex.Replace(path, @"\[\d+\]", ""));

    public Formats Formats { get; set; } = Locale.Formats;

    public virtual void OpenAddress(string address) => Opened.Add(address);

    public virtual byte[]? Resource(string name) => Resources.GetValueOrDefault(name);

    public virtual string? IconPath(string name) => Icons.GetValueOrDefault(name);

    public virtual string Text(string? written) =>
        written is null ? "" : written.StartsWith('@') ? Texts.GetValueOrDefault(written[1..], written[1..]) : written;

    public virtual Type? TypeOf(string path) => Values.GetValueOrDefault(path)?.GetType();

    public virtual IReadOnlyList<Choice> Choices(string path) => Offered.GetValueOrDefault(path) ?? [];

    public event Action? Changed;

    /// <summary>The view model changed, as a handler changes it.</summary>
    public void Change() => Changed?.Invoke();
}

/// <summary>Parts made from markup, on a screen of their own.</summary>
public static class Made
{
    /// <summary>The window's parts of <paramref name="markup"/> (the inside of a <c>Window</c> element) in a grid of 12, on a screen <paramref name="width"/> by <paramref name="height"/>.</summary>
    public static (Screen Screen, GridPanel Grid, List<Part> Parts) Window(string markup, IPartHost host, double width = 800, double height = 400)
    {
        var spec = WindowMarkup.Read($"<Window name=\"Test\">{markup}</Window>");
        var grid = new GridPanel(columns: 12);
        var parts = new List<Part>();
        foreach (var p in spec.Parts)
        {
            var part = PartKinds.Make(p, host);
            parts.Add(part);
            grid.Add(part, p.Place(part.Grows));
        }
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(width, height, 1);
        return (screen, grid, parts);
    }
}
