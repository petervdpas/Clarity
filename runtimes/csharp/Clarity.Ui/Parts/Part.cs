// A part of a window (DESIGN 7.9): an element made from its spec in the
// window's markup, which reads and changes the window's view model through
// its host. Each kind of part is a class of its own with [PartKind], in
// Parts/; PartKinds finds it by its kind's name.

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>Marks a class as the part of one or more kinds of the catalog: <c>[PartKind("textbox", "memo")]</c>.</summary>
[AttributeUsage(AttributeTargets.Class)]
public sealed class PartKindAttribute(params string[] kinds) : Attribute
{
    public IReadOnlyList<string> Kinds { get; } = kinds;
}

/// <summary>
/// An input that takes what is typed in it only on leaving or Enter (a text
/// box, a number, a table's cell editor): a key that runs a command has it
/// take it first, as a click does by moving the focus.
/// </summary>
public interface ITakesTyped
{
    /// <summary>Takes what is typed: a change when it differs from the value.</summary>
    void TakeTyped();
}

/// <summary>Has the input the focus is in take what is typed in it.</summary>
public static class TakesTyped
{
    /// <summary>Walks up from <paramref name="focused"/> to the first input that takes typed text, and has it take it.</summary>
    public static void From(Element? focused)
    {
        for (var e = focused; e is not null; e = e.Parent)
            if (e is ITakesTyped input)
            {
                input.TakeTyped();
                return;
            }
    }
}

/// <summary>
/// What a part reads and changes: the window's view model, as the host makes
/// it reachable. A value is named by its path in the window's data, as the
/// markup binds it: <c>order.status</c>, <c>shown</c>.
/// </summary>
public interface IPartHost
{
    /// <summary>The value at <paramref name="path"/>; null when there is none.</summary>
    object? Read(string path);

    /// <summary>The records of a list at <paramref name="path"/>: a table's, a list's, a chart's rows.</summary>
    IReadOnlyList<object> Rows(string path);

    /// <summary>The person changed the value at <paramref name="path"/> in <paramref name="part"/>: one edit, which Ctrl+Z takes back.</summary>
    void Write(string part, string path, object? value);

    /// <summary>The state of the part named <paramref name="name"/>, which code sets; none for a part without a name.</summary>
    PartState? State(string? name);

    /// <summary>
    /// Runs a handler of <paramref name="part"/>, or opens the window a click
    /// targets; a change's handler runs as part of the person's edit. True
    /// when it ran and did not refuse (a wizard's Next stays on its step when
    /// it refused).
    /// </summary>
    bool Run(string? part, HandlerCall call);

    /// <summary>
    /// The person changed what an unbound part keeps in its own state (its
    /// text, its tick): one edit of <paramref name="part"/>, which Ctrl+Z takes back.
    /// </summary>
    void Edited(string part);

    /// <summary>A text as written in the project, in the language shown: <c>@saveCaption</c> by its key, else as it is.</summary>
    string Text(string? written);

    /// <summary>The type of the value at <paramref name="path"/>: an enum offers its values, a record its fields.</summary>
    Type? TypeOf(string path);

    /// <summary>What a dropdown or radio group bound to <paramref name="path"/> offers: an enum's values, or the records a reference points at.</summary>
    IReadOnlyList<Choice> Choices(string path);

    /// <summary>
    /// What the dropdown <paramref name="part"/> bound to <paramref name="path"/>
    /// offers: a typed one (<c>typed</c>) the values its field already has, a
    /// search one (<c>search</c> with <c>show</c>) the records it points at
    /// with the fields it shows; any other what <see cref="Choices"/> offers.
    /// </summary>
    IReadOnlyList<Choice> Offered(PartSpec part, string path) => Choices(path);

    /// <summary>
    /// What the application knows of the field at <paramref name="path"/>
    /// (<c>order.total</c>, <c>tasks[0].done</c>, <c>tasks.done</c>): its label,
    /// format, whether it is required, its range, what it refers to; none
    /// for a value that is no entity's field.
    /// </summary>
    FieldFacts? Field(string path);

    /// <summary>The person's formats for numbers and dates.</summary>
    Formats Formats { get; }

    /// <summary>Opens an address in the system's program for it: a web page in the browser, <c>mailto:</c> in the mail program.</summary>
    void OpenAddress(string address);

    /// <summary>A file of the application's resources folder by its name (<c>logo.png</c>); none when it carries none.</summary>
    byte[]? Resource(string name);

    /// <summary>The SVG path of one of the icons the application carries, by its name (<c>triangle-alert</c>); none when it carries none.</summary>
    string? IconPath(string name);

    /// <summary>The view model changed: parts read their values again.</summary>
    event Action? Changed;

    // -- Files ---------------------------------------------------------------

    /// <summary>The system's file dialogs and the program that opens a file, as the window's host gives them; one that picks nothing where it has none.</summary>
    IFilePicker Files { get; }

    /// <summary>
    /// The stored file of the record's field <paramref name="bind"/>
    /// (<c>task.attachment</c>): the one the record keeps when
    /// <paramref name="storePath"/> is none, else the file of the field's
    /// store at that path; none when there is none.
    /// </summary>
    Task<byte[]?> Fetch(string bind, string? storePath);

    /// <summary>Says something to the person among the window's messages: an info note that goes by itself, a warning or an error in its dialog.</summary>
    void Say(Level level, Clarity.Runtime.Text text);

    // -- Canvases and custom parts ---------------------------------------------

    /// <summary>
    /// Runs the handler of <paramref name="event"/> (click, drag, drop, bend)
    /// of canvas <paramref name="part"/> with where it was, as an edit Ctrl+Z
    /// takes back; null when the window has no command for it, and the
    /// canvas runs its handler itself.
    /// </summary>
    bool? Pointed(string part, string @event, Clarity.Runtime.Pointer at);

    /// <summary>Applies what the person did on a canvas with a mode (DESIGN 5.31) as one edit; null when the window has no command for it.</summary>
    bool? Acted(string part, JsonObject action);

    /// <summary>The engine beside the screen named <paramref name="name"/>, a new instance each call; none when the window has none.</summary>
    Func<IFrames>? Engine(string name);

    /// <summary>The engine of a canvas, or a custom part, <paramref name="part"/> told the window <paramref name="tell"/>: the handler wired to it runs with the values.</summary>
    void Told(string part, string tell, JsonElement[] values);

    // -- Keys ----------------------------------------------------------------

    /// <summary>
    /// The keys command <paramref name="command"/> answers (DESIGN 5.28): the
    /// person's, where they set their own, else <paramref name="designed"/>.
    /// A command of the app's frame is known by its name, a window's own by
    /// <c>Window.name</c>.
    /// </summary>
    IReadOnlyList<string> KeysOf(string command, string? designed);
}

public abstract class Part : Element
{
    public PartSpec Spec { get; internal set; } = null!;
    public IPartHost Host { get; internal set; } = null!;

    /// <summary>Whether its rows share the window's spare height (a table, a list); a container grows when a part in it does.</summary>
    public virtual bool Grows => false;

    /// <summary>Its state, which code sets by its name.</summary>
    public PartState? State => Host.State(Spec.Name);

    public override bool IsShown => State?.IsVisible ?? true;


    /// <summary>Whether the person may only look: marked read-only in its markup, or made read-only by code through its state.</summary>
    public bool ReadOnly => Spec.Flag("readonly") || (State?.IsReadOnly ?? false);

    /// <summary>Whether the person may use it: enabled, and its window not waiting.</summary>
    public bool Enabled => State?.IsEnabled ?? true;

    /// <summary>The path in the window's data it binds: <c>order.status</c>; none unbound.</summary>
    public string? Bound => Spec.Text("bind");

    /// <summary>
    /// Runs its handlers of <paramref name="event"/>; a click also opens the
    /// window its <c>opens</c> names (the <c>onClick</c> field). False when one refused.
    /// </summary>
    protected bool Fire(string @event)
    {
        var ran = true;
        foreach (var call in Spec.On(@event))
            ran &= Host.Run(Spec.Name, call);
        if (@event == "click" && Spec.Text("onClick") is { Length: > 0 } target)
            ran &= Host.Run(Spec.Name, new HandlerCall("click", target, []));
        return ran;
    }

    /// <summary>The person picked an item of its menu: the item's handler runs, or the window it opens opens.</summary>
    protected internal void Pick(CommandSpec item) => PartKinds.Pick(Host, Spec.Name, item);

    /// <summary>Made: its spec and host are set; it builds what it holds here.</summary>
    protected internal virtual void Made() { }

    /// <summary>The parts of a grid it holds (a panel's, a page's), placed in a grid of <paramref name="columns"/>.</summary>
    protected GridPanel GridOf(IReadOnlyList<PartSpec> parts, int columns, double gap = GridLayout.Gap)
    {
        var grid = new GridPanel(columns, gap);
        foreach (var spec in parts)
        {
            var part = PartKinds.Make(spec, Host);
            grid.Add(part, spec.Place(part.Grows));
        }
        return grid;
    }
}

/// <summary>Finds the part class of each kind, and makes parts.</summary>
public static class PartKinds
{
    static readonly Lazy<Dictionary<string, Type>> classes = new(() =>
        typeof(Part).Assembly.GetTypes()
            .Where(t => !t.IsAbstract && typeof(Part).IsAssignableFrom(t))
            .SelectMany(t => (t.GetCustomAttributes(typeof(PartKindAttribute), false).FirstOrDefault() as PartKindAttribute)?.Kinds.Select(k => (k, t)) ?? [])
            .ToDictionary(p => p.k, p => p.t));

    /// <summary>The kinds Clarity.UI draws so far.</summary>
    public static IReadOnlyCollection<string> Drawn => classes.Value.Keys;

    /// <summary>A menu's item picked: its handler runs, or the window its <c>opens</c> names (<c>onClick</c>) opens.</summary>
    public static void Pick(IPartHost host, string? part, CommandSpec item)
    {
        if (item.Text("handler") is { Length: > 0 } handler)
            host.Run(part, WindowMarkup.HandlerOf("click", handler));
        if (item.Text("onClick") is { Length: > 0 } target)
            host.Run(part, new HandlerCall("click", target, []));
    }

    /// <summary>
    /// The part of <paramref name="spec"/>: the variant that fits it (a typed
    /// or search dropdown, a picture field), else its kind's own part; a kind
    /// not drawn yet, or one that fails as it is made, shows that in its place.
    /// </summary>
    public static Part Make(PartSpec spec, IPartHost host)
    {
        Part part;
        try
        {
            var type = PartVariants.For(spec, host) ?? classes.Value.GetValueOrDefault(spec.Kind);
            part = type is null ? new MissingPart() : (Part)Activator.CreateInstance(type)!;
            part.Spec = spec;
            part.Host = host;
            part.Made();
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            part = new MissingPart { Why = e.InnerException?.Message ?? e.Message, Spec = spec, Host = host };
        }
        return part;
    }
}

/// <summary>A kind of part Clarity.UI does not draw yet, or one that failed: its kind and why, in its place (fail-safe).</summary>
public sealed class MissingPart : Part
{
    public string? Why { get; init; }

    public override void Draw(Painter painter)
    {
        var line = painter.Look.Color(Why is null ? "border" : "error");
        painter.Stroke(Bounds, line, 1, painter.Look.Radius("input"));
        painter.Text(Why is null ? Spec.Kind : $"{Spec.Kind}: {Why}", Bounds.Inset(6), painter.Look.Color(Why is null ? "muted" : "error"), painter.Look.Size("small"));
    }
}
