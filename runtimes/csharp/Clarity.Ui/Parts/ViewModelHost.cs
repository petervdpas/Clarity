// The host of a generated window (DESIGN 7.9): its parts read and change the
// window's view model (a WindowViewModel the C# generator wrote) through it.
// A path of the markup (`order.status`) is the view model's properties in
// Pascal case (Order.Status); a handler (`approveClicked`) is the command
// the generator wrote for it (ApproveClickedCommand); a part's state is its
// PartState property. What each view model type has is found once, by
// reflection, and kept per type. What the application carries beside its
// windows (its fields' facts in the generated Fields class, its icons in
// Icons, its resources folder) is read from the view model's assembly.

using System.Collections;
using System.Collections.Concurrent;
using System.ComponentModel;
using System.Globalization;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;
using System.Windows.Input;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed partial class ViewModelHost : IPartHost, IDisposable
{
    readonly Dictionary<string, PartState> states = [];
    readonly Dictionary<string, PartSpec> parts = [];
    readonly List<INotifyPropertyChanged> heard = [];
    readonly Dictionary<(string Handler, string Args), string> calls = [];
    readonly Shape shape;

    public ViewModelHost(WindowViewModel model, WindowSpec? spec = null)
    {
        Model = model;
        shape = Shape.Of(model.GetType());
        foreach (var property in shape.States)
            if (property.GetValue(model) is PartState state)
            {
                states.TryAdd(state.Name, state);
                Hear(state);
            }
        Hear(model);
        // The person's settings change in every window: a part bound to one shows it.
        foreach (var property in shape.Settings)
            if (property.GetValue(model) is INotifyPropertyChanged settings)
                Hear(settings);
        if (spec is not null)
        {
            foreach (var part in All(spec.Parts))
                if (part.Name is { } name)
                    parts.TryAdd(name, part);
            NameCalls(spec.Parts);
        }
        Locale.Changed += Shout;
    }

    /// <summary>The view model the window shows.</summary>
    public WindowViewModel Model { get; }

    public event Action? Changed;

    void Hear(INotifyPropertyChanged source)
    {
        source.PropertyChanged += Heard;
        heard.Add(source);
    }

    void Heard(object? sender, PropertyChangedEventArgs e) => Shout();

    void Shout() => Changed?.Invoke();

    /// <summary>The window is gone: it hears its view model no more.</summary>
    public void Dispose()
    {
        foreach (var source in heard)
            source.PropertyChanged -= Heard;
        heard.Clear();
        Locale.Changed -= Shout;
    }

    static IEnumerable<PartSpec> All(IEnumerable<PartSpec> specs)
    {
        foreach (var spec in specs)
        {
            yield return spec;
            foreach (var inner in All(spec.Children.Concat(spec.Pages.SelectMany(p => p.Children))))
                yield return inner;
        }
    }

    // -- Values ----------------------------------------------------------------

    public object? Read(string path)
    {
        object? at = Model;
        foreach (var step in path.Split('.'))
        {
            // A step into a list names the record at its place: `tasks[1]` in `tasks[1].done`.
            var (name, place) = Step(step);
            if (at is null || Shape.Member(at.GetType(), name) is not { } property)
                return null;
            at = property.GetValue(at);
            if (place is { } i)
                at = at is IList list && i >= 0 && i < list.Count ? list[i] : null;
        }
        return at;
    }

    /// <summary>A step of a path: its name, and the place of a record when it has one (<c>tasks[1]</c>).</summary>
    static (string Name, int? Place) Step(string step)
    {
        var open = step.IndexOf('[');
        if (open < 0 || !step.EndsWith(']'))
            return (step, null);
        return int.TryParse(step.AsSpan(open + 1, step.Length - open - 2), NumberStyles.None, CultureInfo.InvariantCulture, out var place)
            ? (step[..open], place)
            : (step, null);
    }

    public IReadOnlyList<object> Rows(string path) =>
        Read(path) is IEnumerable list and not string ? [.. list.Cast<object>()] : [];

    public Type? TypeOf(string path)
    {
        Type? type = Model.GetType();
        foreach (var name in path.Split('.'))
        {
            if (type is null)
                return null;
            // A path through a list goes on in its rows: `orders.note` is each order's note.
            type = Shape.Member(Shape.RowType(type) ?? type, name)?.PropertyType;
        }
        return type;
    }

    public void Write(string part, string path, object? value)
    {
        var dot = path.LastIndexOf('.');
        var owner = dot < 0 ? Model : Read(path[..dot]);
        if (owner is null || Shape.Member(owner.GetType(), path[(dot + 1)..]) is not { CanWrite: true } property)
        {
            Console.Error.WriteLine($"{part} writes {path}, which the window has not");
            return;
        }
        object? converted;
        try
        {
            converted = Converted(value, property.PropertyType);
        }
        catch (Exception e) when (e is FormatException or InvalidCastException or OverflowException or ArgumentException)
        {
            Console.Error.WriteLine($"{part} writes {path} with {value}, which is no {property.PropertyType.Name}");
            return;
        }
        if (Equals(property.GetValue(owner), converted))
            return;
        void Set() => property.SetValue(owner, converted);
        if (Scope(part) is { } scope)
            Model.EditOf(part, scope, ends: true, Set);
        else
            Set();
        Shout();
    }

    /// <summary>Where the edits of <paramref name="part"/> are undone: its <c>undoScope</c>, else its own name; none with <c>no undo</c>.</summary>
    public string? Scope(string? part)
    {
        if (part is null)
            return null;
        if (!parts.TryGetValue(part, out var spec))
            return part;
        if (spec.Flag("noUndo"))
            return null;
        return spec.Text("undoScope") ?? part;
    }

    /// <summary>A value as the type of the property it goes to: a number from a part's double, an enum from its name.</summary>
    static object? Converted(object? value, Type type)
    {
        var target = Nullable.GetUnderlyingType(type) ?? type;
        if (value is null)
            return null;
        if (target.IsInstanceOfType(value))
            return value;
        if (value is Choice choice)
            return Converted(choice.Value, type);
        if (target.IsEnum)
            return value is string name ? Enum.Parse(target, name, ignoreCase: true) : Enum.ToObject(target, value);
        if (value is string text && Nullable.GetUnderlyingType(type) is not null && text.Length == 0)
            return null;
        if (target == typeof(string))
            return Convert.ToString(value, CultureInfo.InvariantCulture);
        if (target == typeof(DateOnly) && value is DateTime moment)
            return DateOnly.FromDateTime(moment);
        if (target == typeof(long) && value is double d)
            return (long)Math.Round(d);
        return Convert.ChangeType(value, target, CultureInfo.InvariantCulture);
    }

    // -- Handlers --------------------------------------------------------------

    public bool Run(string? part, HandlerCall call)
    {
        if (Command(call) is not { } command)
        {
            Console.Error.WriteLine($"{part ?? "a part"} runs {call.Written}, which the window has no command for");
            Model.Say(Level.Error, Clarity.Runtime.Text.Plain(Locale.Translate("handlerMissing", "This does nothing yet: the window has no handler for it.")));
            Shout();
            return false;
        }
        // A handler runs through the view model's Handle, which says whether it
        // refused: one without values by its On<Handler>, a call with values by
        // the Run_ the generator writes beside its command. A command cannot say.
        var ran = true;
        var handler = call.Args.Count == 0 ? Handler(call.Handler) : null;
        var withValues = call.Args.Count > 0 ? RunOf(call) : null;
        void Execute()
        {
            if (handler is not null)
                ran = Model.Handle(handler, null, Pascal(call.Handler));
            else if (withValues is not null)
                ran = withValues();
            else
                command.Execute(null);
        }
        // A change (or a reorder) the person made is an edit: what its handler changes is taken back with it.
        if (call.Event is "change" or "reorder" && Scope(part) is { } scope)
            Model.EditOf(part!, scope, ends: true, Execute);
        else
            Execute();
        Shout();
        return ran;
    }

    /// <summary>The view model's own On&lt;Handler&gt; of a handler without values, which its command runs; none for a target or a call.</summary>
    Action? Handler(string handler)
    {
        var name = Pascal(handler);
        if (!shape.Handlers.Contains(name) || Named(name) is null)
            return null;
        var method = Model.GetType().GetMethod("On" + name, BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Instance, Type.EmptyTypes);
        return method is { ReturnType.Name: "Void" } ? method.CreateDelegate<Action>(Model) : null;
    }

    /// <summary>An unbound part's own text or tick changed: one edit of the part, in its undo scope.</summary>
    public void Edited(string part)
    {
        if (Scope(part) is { } scope)
            Model.Edited(part, scope);
        Shout();
    }

    /// <summary>The command of a handler call: its handler's, its call's with values, or a window it opens.</summary>
    ICommand? Command(HandlerCall call)
    {
        if (call.Args.Count == 0)
            return Named(Pascal(call.Handler)) ?? Target(call.Handler);
        return CommandOf(call) is { } name ? Named(name) : null;
    }

    /// <summary>The name of the command of a call with values, as the generator wrote it; none when the view model has none.</summary>
    string? CommandOf(HandlerCall call)
    {
        var named = calls.GetValueOrDefault((call.Handler, string.Join(",", call.Args)));
        if (named is not null && Named(named) is not null)
            return named;
        // The generator numbers a name that is taken: Digit7, then Digit72.
        var name = CallCommand(call);
        for (var n = 1; n < 10; n++)
        {
            var numbered = n == 1 ? name : $"{name}{n}";
            if (Named(numbered) is not null)
                return numbered;
        }
        return null;
    }

    /// <summary>The Run_ of a call with values (Run_Digit7), which runs it and says whether its handler refused; none from an older generator.</summary>
    Func<bool>? RunOf(HandlerCall call)
    {
        if (CommandOf(call) is not { } name)
            return null;
        var method = Model.GetType().GetMethod("Run_" + name, BindingFlags.Public | BindingFlags.Instance, Type.EmptyTypes);
        return method is { ReturnType.Name: "Boolean" } ? method.CreateDelegate<Func<bool>>(Model) : null;
    }

    ICommand? Named(string command) => shape.Commands.GetValueOrDefault(command)?.GetValue(Model) as ICommand;

    /// <summary>What a click opens: a window (in place or in a window of its own), or Clarity's own targets.</summary>
    ICommand? Target(string target) => target switch
    {
        "quit" => Named("Quit_"),
        "close" => Named("CloseWindow_"),
        "keys" => Named("Keys_"),
        "undo" => Named("Undo_"),
        "redo" => Named("Redo_"),
        _ => Named($"Open_{target}") ?? Named($"OpenNew_{target}") ?? Named($"Show_{target}"),
    };

    /// <summary>
    /// The command of each call with values its window's parts wire, named as
    /// the generator names them: in the order of the parts, a name taken by a
    /// handler or an earlier call numbered from 2.
    /// </summary>
    void NameCalls(IEnumerable<PartSpec> specs)
    {
        var taken = new HashSet<string>(shape.Handlers);
        foreach (var spec in All(specs))
            foreach (var call in spec.Handlers.Where(h => h.Args.Count > 0))
            {
                var key = (call.Handler, string.Join(",", call.Args));
                if (calls.ContainsKey(key))
                    continue;
                var name = CallCommand(call);
                var numbered = name;
                for (var n = 2; !taken.Add(numbered); n++)
                    numbered = $"{name}{n}";
                calls[key] = numbered;
            }
    }

    /// <summary>
    /// The command the generator names for a call with values, as
    /// clarity-gen's call_command does: <c>digit(7)</c> is Digit7,
    /// <c>scale(0.5, "half")</c> Scale0_5Half.
    /// </summary>
    public static string CallCommand(HandlerCall call)
    {
        var name = new StringBuilder(Pascal(call.Handler));
        foreach (var value in call.Args)
        {
            if (value.Length >= 2 && value[0] == '"' && value[^1] == '"')
            {
                var words = new string([.. value[1..^1].Select(c => char.IsAsciiLetterOrDigit(c) ? c : '_')]);
                name.Append(Pascal(words));
            }
            else if (value is "true" or "false")
                name.Append(value == "true" ? "True" : "False");
            else if (double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out _))
                name.Append(value.Replace("-", "Minus").Replace(".", "_"));
            else
                name.Append(Pascal(value));
        }
        return name.ToString();
    }

    /// <summary>A Clarity name in Pascal case, as clarity-gen's pascal: <c>finished_by</c> is FinishedBy; nothing, or a digit first, starts with _.</summary>
    public static string Pascal(string name)
    {
        var pascal = new StringBuilder();
        var start = true;
        foreach (var c in name)
        {
            if (!char.IsLetterOrDigit(c))
            {
                start = true;
                continue;
            }
            pascal.Append(start ? char.ToUpperInvariant(c) : c);
            start = false;
        }
        if (pascal.Length == 0 || char.IsAsciiDigit(pascal[0]))
            pascal.Insert(0, '_');
        return pascal.ToString();
    }

    // -- What the application carries --------------------------------------------

    /// <summary>
    /// The facts of the field at <paramref name="path"/>, by the type of the
    /// record it is in and its name: <c>order.total</c> and <c>orders[0].total</c>
    /// and <c>orders.total</c> are all <c>Order.total</c> of the generated Fields.
    /// </summary>
    public FieldFacts? Field(string path)
    {
        if (Carried.Of(Model.GetType()).Fields is not { } fields)
            return null;
        var plain = Places().Replace(path, "");
        var dot = plain.LastIndexOf('.');
        var owner = dot < 0 ? Model.GetType() : TypeOf(plain[..dot]);
        if (owner is null)
            return null;
        owner = Shape.RowType(owner) ?? owner;
        owner = Nullable.GetUnderlyingType(owner) ?? owner;
        // The entity is an open set of the application's, so its key is made here.
        return fields.GetValueOrDefault($"{owner.Name}.{plain[(dot + 1)..]}");
    }

    [GeneratedRegex(@"\[\d+\]")]
    private static partial Regex Places();

    public Formats Formats => Locale.Formats;

    /// <summary>How the host opens an address in the system's program; none, and it is only said (a test, a page).</summary>
    public Action<string>? Opener { get; init; }

    public void OpenAddress(string address)
    {
        if (Opener is { } open)
            open(address);
        else
            Console.Error.WriteLine($"this host opens no address: {address}");
    }

    public byte[]? Resource(string name) => Carried.Resource(Model.GetType().Assembly, name);

    public string? IconPath(string name) => Carried.Of(Model.GetType()).Icons.GetValueOrDefault(IconPart.Member(name));

    // -- States, texts and choices ---------------------------------------------

    public PartState? State(string? name) => name is not null && states.TryGetValue(name, out var state) ? state : null;

    public string Text(string? written) =>
        written is null ? "" : written.StartsWith('@') ? Locale.Translate(written[1..]) : written;

    public IReadOnlyList<Choice> Choices(string path)
    {
        if (TypeOf(path) is not { } declared)
            return [];
        var type = Nullable.GetUnderlyingType(declared) ?? declared;
        var field = path[(path.LastIndexOf('.') + 1)..];
        if (type.IsEnum)
            return EnumChoices(type);
        // The settings `language` and `theme` offer the project's languages and themes.
        var dot = path.LastIndexOf('.');
        if (dot > 0 && TypeOf(path[..dot]) is { } owner && typeof(ISettings).IsAssignableFrom(owner) && type == typeof(string))
        {
            if (field == "language")
                return Choice.LanguagesOffered;
            if (field == "theme")
                return Choice.ThemesOffered;
        }
        if (type == typeof(long))
            return Field(path)?.Refers is { } refers && Named(shape.Choices, refers) is { } referred ? referred : ReferenceChoices(field);
        return [];
    }

    static readonly ConcurrentDictionary<Type, Choice[]> enumChoices = new();

    /// <summary>An enum's values with their labels: the generated <c>Choices.&lt;Enum&gt;</c>, else its names.</summary>
    static IReadOnlyList<Choice> EnumChoices(Type type)
    {
        var generated = type.Assembly.GetType($"{type.Namespace}.Choices")?.GetProperty(type.Name, BindingFlags.Public | BindingFlags.Static);
        // Made each time: the labels follow the language shown.
        if (generated?.GetValue(null) is Choice[] choices)
            return choices;
        return enumChoices.GetOrAdd(type, t => [.. Enum.GetValues(t).Cast<object>().Select(v => new Choice(v, v.ToString()!))]);
    }

    /// <summary>
    /// The records a reference picks from: the view model's <c>&lt;Entity&gt;Choices</c>.
    /// The markup does not say which entity a field refers to, so it is the
    /// one named after the field, else the only entity with choices.
    /// </summary>
    IReadOnlyList<Choice> ReferenceChoices(string field)
    {
        if (Named(shape.Choices, Pascal(field)) is { } byName)
            return byName;
        var entities = shape.Choices.Keys.Where(e => Model.GetType().Assembly.GetType($"{Model.GetType().Namespace}.{e}") is not null).ToList();
        return entities.Count == 1 ? Named(shape.Choices, entities[0]) ?? [] : [];
    }

    Choice[]? Named(Dictionary<string, PropertyInfo> choices, string entity) =>
        choices.TryGetValue(entity, out var property) ? property.GetValue(Model) as Choice[] : null;

    /// <summary>What a view model type has, found once: its members by name, part states, commands, handlers and choices.</summary>
    sealed class Shape
    {
        static readonly ConcurrentDictionary<Type, Shape> shapes = new();
        static readonly ConcurrentDictionary<(Type, string), PropertyInfo?> members = new();

        public required PropertyInfo[] States { get; init; }
        public required PropertyInfo[] Settings { get; init; }
        public required Dictionary<string, PropertyInfo> Commands { get; init; }
        public required HashSet<string> Handlers { get; init; }
        public required Dictionary<string, PropertyInfo> Choices { get; init; }

        public static Shape Of(Type type) => shapes.GetOrAdd(type, Read);

        static Shape Read(Type type)
        {
            var properties = type.GetProperties(BindingFlags.Public | BindingFlags.Instance);
            // The handlers of the window: each has its On<Handler>, which a command or a call runs.
            var handlers = type.GetMethods(BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly)
                .Where(m => m.Name.StartsWith("On", StringComparison.Ordinal) && m.Name.Length > 2 && m.GetBaseDefinition() == m)
                .Select(m => m.Name[2..]);
            return new Shape
            {
                States = [.. properties.Where(p => p.PropertyType == typeof(PartState) && p.GetIndexParameters().Length == 0)],
                Settings = [.. properties.Where(p => typeof(ISettings).IsAssignableFrom(p.PropertyType) && typeof(INotifyPropertyChanged).IsAssignableFrom(p.PropertyType))],
                Commands = properties
                    .Where(p => typeof(ICommand).IsAssignableFrom(p.PropertyType) && p.Name.EndsWith("Command", StringComparison.Ordinal))
                    .ToDictionary(p => p.Name[..^"Command".Length]),
                Handlers = [.. handlers],
                Choices = properties
                    .Where(p => p.PropertyType == typeof(Choice[]) && p.Name.EndsWith("Choices", StringComparison.Ordinal) && p.Name.Length > "Choices".Length)
                    .ToDictionary(p => p.Name[..^"Choices".Length]),
            };
        }

        /// <summary>A property by its Clarity name: <c>finished_by</c> is FinishedBy, or FinishedBy_ where the name is taken.</summary>
        public static PropertyInfo? Member(Type type, string name) => members.GetOrAdd((type, name), key =>
        {
            var pascal = Pascal(key.Item2);
            return key.Item1.GetProperty(pascal, BindingFlags.Public | BindingFlags.Instance)
                ?? key.Item1.GetProperty(pascal + "_", BindingFlags.Public | BindingFlags.Instance);
        });

        /// <summary>The type of a list's rows; none for what is no list.</summary>
        public static Type? RowType(Type type)
        {
            if (type == typeof(string))
                return null;
            if (type.IsArray)
                return type.GetElementType();
            return type.GetInterfaces().Append(type)
                .FirstOrDefault(i => i.IsGenericType && i.GetGenericTypeDefinition() == typeof(IEnumerable<>))?
                .GetGenericArguments()[0];
        }
    }

    /// <summary>What an application's assembly carries beside its windows, found once: its fields' facts and its icons.</summary>
    sealed class Carried
    {
        static readonly ConcurrentDictionary<Type, Carried> carried = new();

        public IReadOnlyDictionary<string, FieldFacts>? Fields { get; private init; }
        public required Dictionary<string, string> Icons { get; init; }

        public static Carried Of(Type model) => carried.GetOrAdd(model, Read);

        static Carried Read(Type model)
        {
            Type[] types;
            try
            {
                types = model.Assembly.GetTypes();
            }
            catch (ReflectionTypeLoadException e)
            {
                types = [.. e.Types.OfType<Type>()];
            }
            // The application's own first: the one in the view model's namespace.
            var ordered = types.OrderBy(t => t.Namespace == model.Namespace ? 0 : 1).ToList();
            var fields = ordered.Where(t => t.Name == "Fields")
                .Select(t => (t.GetProperty("All", BindingFlags.Public | BindingFlags.Static)?.GetValue(null) ?? t.GetField("All", BindingFlags.Public | BindingFlags.Static)?.GetValue(null)) as IReadOnlyDictionary<string, FieldFacts>)
                .FirstOrDefault(d => d is not null);
            var icons = new Dictionary<string, string>();
            foreach (var type in ordered.Where(t => t.Name == "Icons" && t.IsAbstract && t.IsSealed))
                foreach (var field in type.GetFields(BindingFlags.Public | BindingFlags.Static).Where(f => f.IsLiteral && f.FieldType == typeof(string)))
                    icons.TryAdd(field.Name, (string)field.GetRawConstantValue()!);
            return new Carried { Fields = fields, Icons = icons };
        }

        /// <summary>A file of the resources folder, as a generated C# project embeds it (<c>resources/logo.png</c>); none when it carries none.</summary>
        public static byte[]? Resource(Assembly app, string name)
        {
            using var stream = app.GetManifestResourceStream($"resources/{name}");
            if (stream is null)
                return null;
            using var bytes = new MemoryStream();
            stream.CopyTo(bytes);
            return bytes.ToArray();
        }
    }
}
