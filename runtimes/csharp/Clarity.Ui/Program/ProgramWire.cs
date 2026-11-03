// A generated C# application as the library's program (DESIGN 7.9): the
// library asks by command (clarity_ui.h, the web server's wire) and this
// answers from the application's view models, which stay the truth of each
// window. A window's state goes out as the runtime's WindowState, and what
// the library sends back with an event is given to its view model before
// the event runs. What a handler asks of the screen (a window to open, a
// question, closing, the clipboard, messages, hears) comes back in the
// reply. A view model a handler opened is handed over by a token in the
// `with` of its open, a question by a token as its handler. When an engine
// says how far a long call is, the library runs the handlers on its worker
// thread and asks how far the call is from the screen's thread: the windows
// share one LongCall, which answers it and stops the call.

using System.Collections.ObjectModel;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Windows.Input;
using Clarity.Runtime;

namespace Clarity.Ui;

internal sealed class ProgramWire : IDesktop
{
    /// <summary>The key of an open's <c>with</c> that hands over a view model the application made.</summary>
    public const string WindowToken = "$window";

    /// <summary>What a question's handler starts with.</summary>
    const string AskPrefix = "$ask:";

    /// <summary>The handler of the app's <c>on unconnected</c>.</summary>
    const string UnconnectedHandler = "$unconnected";

    readonly Lock gate = new();
    readonly Assembly app;
    readonly Dictionary<long, OpenWindow> windows = [];
    readonly Dictionary<string, WindowViewModel> handed = [];
    readonly Dictionary<string, (OpenWindow Window, Question Question)> questions = [];
    readonly Dictionary<ulong, (IFrames Frames, Lock Turn)> engines = [];
    readonly Func<string, string?> markupOf;
    /// <summary>The long engine call of the program's windows (DESIGN 5.12): the library shows its bar with Stop.</summary>
    readonly LongCall calls;
    readonly bool watches;
    readonly Lock watchGate = new();
    /// <summary>The watch whose handler runs now, the last that ended, and one stopped before it started.</summary>
    ulong? watching;
    ulong lastEnded;
    ulong? stoppedEarly;
    WindowViewModel? main;
    Db? db;
    int next;
    ulong nextEngine;

    // While the databases did not open (DESIGN 5.27).
    readonly Unconnected? unconnected;
    readonly Text? problem;
    Db? memory;
    Db? connected;

    public ProgramWire(Assembly app, WindowViewModel? main, Func<string, string?> markupOf, Unconnected? unconnected = null, Text? problem = null, bool? watches = null)
    {
        this.app = app;
        this.watches = watches ?? Watches(app);
        calls = new LongCall { Waiter = WaitOutside };
        this.main = main;
        db = main?.IsMirror == false ? main.Db : null;
        this.markupOf = markupOf;
        this.unconnected = unconnected;
        this.problem = problem;
    }

    /// <summary>The databases that opened while the program ran unconnected; the caller closes them at its end.</summary>
    public Db? Connected => connected;

    /// <summary>Code quit the application (`quit`) or opened the keys window (`keys`): said in the next reply.</summary>
    bool quitting, showingKeys;

    /// <summary>The built-in <c>quit</c> from code: the next reply says so, and the library closes every window, windows of their own too.</summary>
    public void Quit() => quitting = true;

    /// <summary>The built-in <c>keys</c> from code: the next reply says so, and the library opens Clarity's keys window.</summary>
    public void ShowKeys() => showingKeys = true;

    /// <summary>The long call the windows share, for tests.</summary>
    internal LongCall Calls => calls;

    /// <summary>Whether an engine of the application says how far a long call is: the generated <c>Engines.Watches</c>.</summary>
    static bool Watches(Assembly app)
    {
        Type[] types;
        try
        {
            types = app.GetTypes();
        }
        catch (ReflectionTypeLoadException e)
        {
            types = [.. e.Types.OfType<Type>()];
        }
        return types.Where(t => t.Name == "Engines").Any(t => t.GetField("Watches", BindingFlags.Public | BindingFlags.Static)?.GetValue(null) is true);
    }

    /// <summary>Answers a command of the library: <c>{"value": ...}</c>, <c>{"error": text}</c>, or none for the program's default.</summary>
    public JsonObject? Answer(string command, string args)
    {
        JsonObject given;
        try
        {
            given = JsonNode.Parse(args.Length == 0 ? "{}" : args) as JsonObject ?? [];
        }
        catch (JsonException e)
        {
            return WireJson.Error($"the library sent {command} with JSON that does not read: {e.Message}");
        }
        try
        {
            // Engine frames run on threads of their own, an instance at a time; everything else one at a time.
            if (command is "engine_frame" or "engine_hears")
                return Engine(command, given);
            // A long call's watch is asked from the screen's thread while its handler holds the program.
            if (command is "watches" or "start_watch" or "watch_progress" or "stop_watch" or "end_watch")
                return Watch(command, given["id"]?.GetValue<ulong>() ?? 0);
            lock (gate)
                return Command(command, given);
        }
        catch (Exception e)
        {
            // Fail safe: what a handler threw shows as an error, and the program goes on.
            var inner = e is TargetInvocationException { InnerException: { } cause } ? cause : e;
            Console.Error.WriteLine($"{command}: {inner}");
            return WireJson.Error(inner.Message);
        }
    }

    static JsonObject Value(JsonNode? value) => new() { ["value"] = value };

    JsonObject? Command(string command, JsonObject given)
    {
        switch (command)
        {
            case "started":
                return Value(Started());
            case "start_app":
                return Value(EmptyReply());
            case "connect":
                return Connect();
            case "open_window":
                return Open(given["window"]?.GetValue<string>() ?? "", given["with"] as JsonObject);
            case "close_window":
                return OnWindow(given, w =>
                {
                    w.Model.Closing();
                    var reply = Reply(w);
                    windows.Remove(w.Instance);
                    w.Dispose();
                    return reply;
                });
            case "return_to":
                return OnWindow(given, w =>
                {
                    w.Apply(given["state"] as JsonObject);
                    w.Model.Returned();
                    return Reply(w);
                });
            case "dispatch_event":
                return OnWindow(given, w =>
                {
                    w.Apply(given["state"] as JsonObject);
                    var handler = given["handler"]?.GetValue<string>() ?? "";
                    if (!Pointed(w, handler, null))
                        Handlers.Run(w.Model, handler);
                    return Reply(w);
                });
            case "edit_event":
                return OnWindow(given, w => Edit(w, given));
            case "undo_edit":
                return OnWindow(given, w =>
                {
                    w.Apply(given["state"] as JsonObject);
                    w.Model.UndoEdit(given["scope"]?.GetValue<string>() ?? "", given["back"]?.GetValue<bool>() ?? true);
                    return Reply(w);
                });
            case "canvas_event":
                return OnWindow(given, w => Canvas(w, given));
            case "answer_event":
                return AnswerEvent(given);
            case "ref_options":
                return Value(RefOptions(given["entity"]?.GetValue<string>() ?? ""));
            case "field_values":
                return Value(new JsonArray([.. FieldValues(given["entity"]?.GetValue<string>() ?? "", given["field"]?.GetValue<string>() ?? "").Select(v => (JsonNode?)v)]));
            case "find_options":
                return Value(FindOptions(given));
            case "field_bytes":
                return Value(FieldBytes(given["entity"]?.GetValue<string>() ?? "", given["id"]?.GetValue<long>() ?? 0, given["field"]?.GetValue<string>() ?? "") is { } bytes ? Convert.ToBase64String(bytes) : null);
            case "store_file":
                return StoreFile(given["path"]?.GetValue<string>() ?? "");
            case "read_settings":
                return Value(SettingsNow());
            case "keep_settings":
                KeepSettings(given["settings"] as JsonObject);
                return Value(null);
            case "set_language":
                if (given["language"]?.GetValue<string>() is { Length: > 0 } language && language != Locale.Current)
                    Locale.Choose(language);
                return Value(null);
            case "facts":
                return Facts() is { } facts ? Value(facts) : null;
            case "engine_open":
                return EngineOpen(given["name"]?.GetValue<string>() ?? "");
            case "engine_close":
                EngineClose(given["id"]?.GetValue<ulong>() ?? 0);
                return Value(null);
            default:
                // Everything else takes the program's default (no facts, no watch).
                return null;
        }
    }

    // -- Windows ---------------------------------------------------------------------

    JsonObject Started()
    {
        if (unconnected is null || problem is null)
            return new JsonObject { ["main"] = main is null ? "" : Handlers.WindowName(main), ["unconnected"] = null };
        var mainName = MainType() is { } type ? Handlers.WindowName(type) : "Main";
        memory ??= unconnected.Empty();
        string? window = null;
        if (unconnected.Window is { } keeps)
        {
            var first = keeps(memory);
            window = Handlers.WindowName(first);
            handed[$"unconnected:{window}"] = first;
        }
        return new JsonObject
        {
            ["main"] = mainName,
            ["unconnected"] = new JsonObject
            {
                ["handler"] = unconnected.Handler is null ? null : UnconnectedHandler,
                ["window"] = window,
                ["problem"] = WireJson.Text(problem),
            },
        };
    }

    /// <summary>The type of the main window, which the application gives as what it opens once connected.</summary>
    Type? MainType() => unconnected?.Main.Method.ReturnType is { } type && type != typeof(WindowViewModel) ? type : app.GetTypes().FirstOrDefault(t => t.Name == "MainViewModel" && t.IsSubclassOf(typeof(WindowViewModel)));

    JsonObject Connect()
    {
        if (unconnected is null)
            return Value(null);
        try
        {
            connected = unconnected.Connect();
        }
        catch (Exception e)
        {
            return WireJson.Error(Unconnected.Reason(e));
        }
        db = connected;
        main = unconnected.Main(connected);
        return Value(null);
    }

    /// <summary>Opens window <paramref name="name"/>: the view model handed over by its token, the main window, or a new one of its type on the application's databases.</summary>
    JsonObject Open(string name, JsonObject? with)
    {
        WindowViewModel? model = null;
        if (with?[WindowToken]?.GetValue<string>() is { } token && handed.Remove(token, out var given))
            model = given;
        else if (handed.Remove($"unconnected:{name}", out var keeps))
            model = keeps;
        else if (main is not null && Handlers.WindowName(main) == name && !windows.Values.Any(w => w.Model == main))
            model = main;
        else
            model = Make(name);
        if (model is null)
            return WireJson.Error(Text.Plain($"the program has no window {name}"));
        // The windows share one long call: while it runs, none of them runs another handler.
        model.LongCall = calls;
        var window = new OpenWindow(model, name, new WindowMarkup(markupOf(name)));
        windows[window.Instance] = window;
        model.Opened();
        return Value(Reply(window));
    }

    /// <summary>A new view model of window <paramref name="name"/>, as its Open_ command makes one: on the application's databases.</summary>
    WindowViewModel? Make(string name)
    {
        var type = app.GetTypes().FirstOrDefault(t => t.Name == name + "ViewModel" && t.IsSubclassOf(typeof(WindowViewModel)));
        var on = db ?? memory;
        if (type?.GetConstructor([typeof(Db)]) is { } make)
            return (WindowViewModel)make.Invoke([on]);
        return null;
    }

    JsonObject OnWindow(JsonObject given, Func<OpenWindow, JsonObject> act)
    {
        var instance = given["state"]?["instance"]?.GetValue<long>() ?? 0;
        if (!windows.TryGetValue(instance, out var window))
            return WireJson.Error(Text.Plain($"the program has no open window {instance}"));
        return Value(act(window));
    }

    /// <summary>The person ended an edit: the value the library sent is given to the view model as one edit of its undo scope, its handler with it.</summary>
    JsonObject Edit(OpenWindow window, JsonObject given)
    {
        var state = given["state"] as JsonObject;
        var part = given["part"]?.GetValue<string>() ?? "";
        var scope = given["scope"]?.GetValue<string>() ?? part;
        var handler = given["handler"]?.GetValue<string>();
        // A readonly part's change is taken back: the reply shows the value the window still has.
        if (given["readonly"]?.GetValue<bool>() == true)
            return Reply(window);
        // A canvas's drag, drop or bend goes through its command, which makes it the edit the generator wrote (a drag one that goes on).
        if (handler is { Length: > 0 } && window.Markup.PointerOf(handler, part) is not null)
        {
            window.Apply(state);
            if (Pointed(window, handler, part))
                return Reply(window);
        }
        window.Model.EditOf(part, scope, given["ends"]?.GetValue<bool>() ?? true, () =>
        {
            window.Apply(state);
            if (handler is { Length: > 0 })
                Handlers.Run(window.Model, handler);
        });
        window.Model.Changed();
        return Reply(window);
    }

    /// <summary>The generator's prefix of each pointer event's command.</summary>
    static readonly Dictionary<string, string> PointerCommands = new()
    {
        ["click"] = "ClickAt_",
        ["drag"] = "DragAt_",
        ["drop"] = "DropAt_",
        ["bend"] = "BendAt_",
    };

    /// <summary>
    /// Runs a canvas's pointer handler through the command the generator
    /// writes for it (<c>DragAt_Board</c>), with where the pointer was, as
    /// ViewModelHost.Pointed did: false when <paramref name="handler"/> is no
    /// canvas's pointer handler or the view model has no such command.
    /// </summary>
    static bool Pointed(OpenWindow window, string handler, string? part)
    {
        if (window.Markup.PointerOf(handler, part) is not { } pointed || window.State(pointed.Part)?.Pointer is not { } at)
            return false;
        foreach (var member in new[] { Members.Pascal(pointed.Part), Members.Pascal(pointed.Part) + "_" })
            if (window.Model.GetType().GetProperty(PointerCommands[pointed.Event] + member + "Command", BindingFlags.Public | BindingFlags.Instance)?.GetValue(window.Model) is ICommand command)
            {
                if (command.CanExecute(at))
                    command.Execute(at);
                return true;
            }
        return false;
    }

    /// <summary>What the person did on a canvas with a mode: the view model's CanvasAt_ command applies it as one edit.</summary>
    JsonObject Canvas(OpenWindow window, JsonObject given)
    {
        window.Apply(given["state"] as JsonObject);
        var part = given["part"]?.GetValue<string>() ?? "";
        var action = given["action"] as JsonObject ?? [];
        foreach (var member in new[] { Members.Pascal(part), Members.Pascal(part) + "_" })
            if (window.Model.GetType().GetProperty("CanvasAt_" + member + "Command", BindingFlags.Public | BindingFlags.Instance)?.GetValue(window.Model) is ICommand command)
            {
                if (command.CanExecute(action))
                    command.Execute(action);
                break;
            }
        return Reply(window);
    }

    /// <summary>The answer to a question, the app's <c>on unconnected</c>, or a handler a custom part or an engine told, with its values.</summary>
    JsonObject AnswerEvent(JsonObject given)
    {
        var handler = given["handler"]?.GetValue<string>() ?? "";
        var values = given["values"] as JsonArray ?? [];
        if (handler == UnconnectedHandler)
            return Value(Unconnecting());
        if (handler.StartsWith(AskPrefix, StringComparison.Ordinal) && questions.Remove(handler, out var asked))
        {
            asked.Window.Apply(given["state"] as JsonObject);
            asked.Window.Model.Answer(asked.Question, AnswerOf(asked.Question, values.LastOrDefault(), values.Count));
            return Value(Reply(asked.Window));
        }
        return OnWindow(given, w =>
        {
            w.Apply(given["state"] as JsonObject);
            RunWithValues(w.Model, handler, values);
            return Reply(w);
        });
    }

    /// <summary>The app's <c>on unconnected</c> on a window that is not shown, with why: what it opens shows first, what it says shows there.</summary>
    JsonObject Unconnecting()
    {
        memory ??= unconnected!.Empty();
        var said = new ObservableCollection<Message>();
        var opens = new JsonArray();
        var unconnecting = new Unconnecting(Locale.Show(problem!), said, opened =>
        {
            var token = $"w{++next}";
            handed[token] = opened;
            opens.Add(new JsonObject { ["window"] = Handlers.WindowName(opened), ["with"] = new JsonObject { [WindowToken] = token } });
        });
        unconnected!.Handler!(memory, unconnecting);
        var reply = EmptyReply();
        reply["messages"] = new JsonArray([.. said.Select(WireJson.Message)]);
        reply["open"] = opens;
        return reply;
    }

    /// <summary>Runs the view model's On&lt;Handler&gt; that takes the values, as one step; a handler without values when none came.</summary>
    static void RunWithValues(WindowViewModel model, string handler, JsonArray values)
    {
        var name = "On" + Members.Pascal(handler);
        var method = model.GetType().GetMethods(BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Instance)
            .FirstOrDefault(m => (m.Name == name || m.Name == name + "_") && m.GetParameters().Length == values.Count);
        if (method is null)
        {
            Handlers.Run(model, handler);
            return;
        }
        var args = method.GetParameters().Select((p, i) => WireJson.As(values[i], p.ParameterType)).ToArray();
        model.Handle(() => method.Invoke(model, args), args, Members.Pascal(handler));
    }

    /// <summary>A question's answer as its handler takes it: none for a yes, the chosen value, else the value of its kind.</summary>
    static object? AnswerOf(Question question, JsonNode? answer, int count)
    {
        switch (question.Form)
        {
            case ConfirmForm:
                return null;
            case ValueForm { Choices: { } choices }:
                return answer is JsonValue v && v.TryGetValue<double>(out var place) && place >= 0 && place < choices.Count ? choices[(int)place].Value : null;
            case ValueForm { Format: { } format } when count > 0:
                var type = format.Kind switch
                {
                    FormatKind.Integer or FormatKind.Duration => typeof(long?),
                    FormatKind.Decimal or FormatKind.Money => typeof(decimal?),
                    FormatKind.Date => typeof(DateOnly?),
                    FormatKind.DateTime => typeof(DateTime?),
                    _ => typeof(string),
                };
                if (answer is JsonValue text && text.TryGetValue<string>(out var s) && type == typeof(decimal?))
                    return decimal.TryParse(s, System.Globalization.NumberStyles.Number, System.Globalization.CultureInfo.InvariantCulture, out var d) ? d : null;
                return WireJson.As(answer, type);
            default:
                return answer is null ? "" : WireJson.Plain(answer)?.ToString() ?? "";
        }
    }

    // -- Replies ---------------------------------------------------------------------

    static JsonObject EmptyReply() => new()
    {
        ["state"] = new JsonObject { ["window"] = "", ["instance"] = 0, ["data"] = new JsonObject() },
        ["messages"] = new JsonArray(),
        ["open"] = new JsonArray(),
        ["close"] = false,
    };

    /// <summary>The window's state after a call, and what its handler asked of the screen.</summary>
    JsonObject Reply(OpenWindow window)
    {
        var opens = new JsonArray();
        foreach (var (other, newWindow) in window.Opens)
        {
            var token = $"w{++next}";
            handed[token] = other;
            var open = new JsonObject { ["window"] = Handlers.WindowName(other), ["with"] = new JsonObject { [WindowToken] = token } };
            // `open ... in new window`: a desktop window of its own.
            if (newWindow)
                open["new_window"] = true;
            opens.Add(open);
        }
        window.Opens.Clear();
        var asks = new JsonArray();
        foreach (var question in window.Asks)
        {
            var token = $"{AskPrefix}{++next}";
            questions[token] = (window, question);
            asks.Add(new JsonObject
            {
                ["question"] = WireJson.Text(question.Text),
                ["form"] = Form(question.Form, token),
                ["handler"] = token,
                ["values"] = new JsonArray(),
            });
        }
        window.Asks.Clear();
        var reply = new JsonObject
        {
            ["state"] = window.State(),
            ["messages"] = new JsonArray([.. window.TakeSaid().Select(WireJson.Message)]),
            ["open"] = opens,
            ["close"] = window.Close,
            ["asks"] = asks,
            ["hears"] = window.TakeHears(),
        };
        window.Close = false;
        if (quitting)
            reply["quit"] = true;
        if (showingKeys)
            reply["keys"] = true;
        quitting = showingKeys = false;
        if (window.Copy is { } copy)
        {
            reply["copy"] = WireJson.Text(copy);
            window.Copy = null;
        }
        return reply;
    }

    /// <summary>
    /// What a question asks for, as the library's ask dialog reads it: yes or
    /// no, or a value of a kind. Choices go as the records of an entity named
    /// by the question's token, which ref_options answers by their places.
    /// </summary>
    static JsonObject Form(AskForm form, string token) => form switch
    {
        ConfirmForm confirm => new JsonObject { ["kind"] = "confirm", ["yes"] = WireJson.Text(confirm.Yes), ["no"] = WireJson.Text(confirm.No) },
        ValueForm { Choices: { } choices } value => new JsonObject
        {
            ["kind"] = "value",
            ["type"] = "record",
            ["of"] = token,
            ["only"] = null,
            ["default"] = choices.Select((c, i) => (c, i)).FirstOrDefault(p => Equals(p.c.Value, value.Default)) is var (_, at) && value.Default is not null ? at : null,
        },
        ValueForm value => new JsonObject
        {
            ["kind"] = "value",
            ["type"] = value.Format?.Kind switch
            {
                FormatKind.Integer => "integer",
                FormatKind.Decimal => "decimal",
                FormatKind.Money => "money",
                FormatKind.Date => "date",
                FormatKind.DateTime => "datetime",
                FormatKind.Duration => "duration",
                _ => "text",
            },
            ["of"] = null,
            ["only"] = null,
            ["default"] = WireJson.Value(value.Default),
        },
        _ => new JsonObject { ["kind"] = "confirm", ["yes"] = null, ["no"] = null },
    };

    // -- What parts read beside the values ------------------------------------------------

    /// <summary>The open windows, the one on top first: what a reference offers is the last window's that has it.</summary>
    IEnumerable<OpenWindow> Newest() => windows.Values.OrderByDescending(w => w.Instance);

    /// <summary>The records a reference of <paramref name="entity"/> points at: the view model's <c>&lt;Entity&gt;Choices</c>, or a question's choices by their places.</summary>
    JsonArray RefOptions(string entity)
    {
        if (questions.TryGetValue(entity, out var asked) && asked.Question.Form is ValueForm { Choices: { } offered })
            return new JsonArray([.. offered.Select((c, i) => (JsonNode)new JsonObject { ["id"] = i, ["text"] = c.Label })]);
        foreach (var window in Newest())
            if (window.Model.GetType().GetProperty(Members.Pascal(entity) + "Choices", BindingFlags.Public | BindingFlags.Instance)?.GetValue(window.Model) is Choice[] choices)
                return Options(choices);
        return [];
    }

    static JsonArray Options(IEnumerable<Choice> choices) =>
        new([.. choices.Where(c => c.Value is long).Select(c =>
        {
            var option = new JsonObject { ["id"] = (long)c.Value!, ["text"] = c.Label };
            if (c.Cells is { Count: > 0 } cells)
                option["fields"] = new JsonObject(cells.Select((cell, i) => KeyValuePair.Create($"{i}", (JsonNode?)cell)));
            return (JsonNode)option;
        })]);

    /// <summary>The values a text field already has, which a typed dropdown offers: the view model's <c>&lt;Entity&gt;&lt;Field&gt;Values</c>.</summary>
    IEnumerable<string> FieldValues(string entity, string field)
    {
        foreach (var window in Newest())
            if (window.Model.GetType().GetProperty($"{Members.Pascal(entity)}{Members.Pascal(field)}Values", BindingFlags.Public | BindingFlags.Instance)?.GetValue(window.Model) is IEnumerable<string> values)
                return values;
        return [];
    }

    /// <summary>What a typed or search dropdown offers for what was typed (conformance/dropdown-options.json).</summary>
    JsonNode FindOptions(JsonObject given)
    {
        var kind = given["kind"]?.GetValue<string>();
        var entity = given["entity"]?.GetValue<string>() ?? "";
        var typed = given["typed"]?.GetValue<string>() ?? "";
        if (kind == "typed")
            return new JsonArray([.. DropdownOptions.Suggestions(FieldValues(entity, given["field"]?.GetValue<string>() ?? ""), typed).Select(v => (JsonNode?)v)]);
        var show = (given["show"] as JsonArray)?.Select(s => Members.Pascal(s?.GetValue<string>() ?? "")) ?? [];
        foreach (var window in Newest())
            if (window.Model.GetType().GetProperty(Members.Pascal(entity) + string.Concat(show) + "Choices", BindingFlags.Public | BindingFlags.Instance)?.GetValue(window.Model) is Choice[] choices)
                return Options(DropdownOptions.Searched(choices, typed));
        return Options([]);
    }

    /// <summary>
    /// The bytes of a record's image or file field, through the member the
    /// generator writes for it (<c>TaskAttachmentPicture</c>): the open
    /// window whose data holds that record.
    /// </summary>
    byte[]? FieldBytes(string entity, long id, string field)
    {
        foreach (var window in Newest())
            foreach (var property in window.Model.GetType().GetProperties(BindingFlags.Public | BindingFlags.Instance))
            {
                if (property.PropertyType.Name != entity || property.GetIndexParameters().Length > 0)
                    continue;
                var record = property.GetValue(window.Model);
                if (record?.GetType().GetProperty("Id")?.GetValue(record) is not long has || has != id)
                    continue;
                var fetcher = window.Model.GetType().GetProperty(property.Name + Members.Pascal(field) + "Picture", BindingFlags.Public | BindingFlags.Instance);
                if (fetcher?.GetValue(window.Model) is Func<Task<byte[]?>> fetch)
                    return fetch().GetAwaiter().GetResult();
            }
        return null;
    }

    /// <summary>A file of a store by its path, through the window's <c>&lt;Data&gt;&lt;Field&gt;File</c>.</summary>
    JsonObject StoreFile(string path)
    {
        foreach (var window in Newest())
            foreach (var property in window.Model.GetType().GetProperties(BindingFlags.Public | BindingFlags.Instance))
                if (property.Name.EndsWith("File", StringComparison.Ordinal) && property.GetValue(window.Model) is Func<string, Task<byte[]?>> file && file(path).GetAwaiter().GetResult() is { } bytes)
                    return Value(Convert.ToBase64String(bytes));
        return WireJson.Error(Text.Plain($"there is no file {path}"));
    }

    /// <summary>
    /// The facts of the application's fields, from the generated Fields.All
    /// (keyed <c>Entity.field</c>), as fields::Facts::to_json writes them:
    /// label, format, whether required, range, what it refers to. None when
    /// the application carries none: the library reads its model instead.
    /// </summary>
    JsonObject? Facts()
    {
        Type[] types;
        try
        {
            types = app.GetTypes();
        }
        catch (ReflectionTypeLoadException e)
        {
            types = [.. e.Types.OfType<Type>()];
        }
        var all = types.Where(t => t.Name == "Fields")
            .Select(t => (t.GetProperty("All", BindingFlags.Public | BindingFlags.Static)?.GetValue(null) ?? t.GetField("All", BindingFlags.Public | BindingFlags.Static)?.GetValue(null)) as IReadOnlyDictionary<string, FieldFacts>)
            .FirstOrDefault(d => d is not null);
        if (all is null)
            return null;
        var fields = new JsonObject();
        foreach (var (name, facts) in all)
            fields[name] = new JsonObject
            {
                ["label"] = facts.Label,
                ["format"] = facts.Format is { } format ? new JsonObject
                {
                    ["kind"] = JsonNamingPolicy.CamelCase.ConvertName(format.Kind.ToString()),
                    ["scale"] = format.Scale,
                    ["format"] = format.Format,
                } : null,
                ["required"] = facts.Required,
                ["min"] = facts.Min,
                ["max"] = facts.Max,
                ["refers"] = facts.Refers,
                ["parent"] = facts.Parent,
                ["color"] = facts.Color,
            };
        return new JsonObject { ["fields"] = fields };
    }

    // -- Settings ----------------------------------------------------------------------

    /// <summary>The person's settings, the settings object a window shows.</summary>
    object? SettingsObject()
    {
        foreach (var window in Newest().Append(null))
        {
            var model = window?.Model ?? main;
            if (model?.GetType().GetProperty("Settings", BindingFlags.Public | BindingFlags.Instance)?.GetValue(model) is ISettings settings)
                return settings;
        }
        return null;
    }

    JsonObject SettingsNow() => SettingsObject() is { } settings && WireJson.Value(settings) is JsonObject json ? json : [];

    /// <summary>Keeps the settings as the screen has them: each that changed is set on the settings object, which keeps itself.</summary>
    void KeepSettings(JsonObject? settings)
    {
        if (settings is null || SettingsObject() is not { } own)
            return;
        var now = SettingsNow();
        foreach (var (name, value) in settings)
        {
            if (name == "id" || JsonNode.DeepEquals(value, now[name]) || Members.Member(own.GetType(), name) is not { CanWrite: true } property)
                continue;
            try
            {
                property.SetValue(own, WireJson.As(value, property.PropertyType));
            }
            catch (Exception e) when (e is JsonException or FormatException or InvalidCastException or ArgumentException)
            {
                Console.Error.WriteLine($"the setting {name} cannot take {value?.ToJsonString()}: {e.Message}");
            }
        }
    }

    // -- Long calls ------------------------------------------------------------------------

    /// <summary>
    /// The library's watch of a long call (clarity_ui.h): its handler runs on
    /// the library's worker thread between start_watch and end_watch; how far
    /// it is and Stop come from the screen's thread meanwhile. Watches are
    /// numbered upwards, so a Stop of one that ended stops nothing.
    /// </summary>
    JsonObject? Watch(string command, ulong id)
    {
        if (command == "watches")
            return Value(watches);
        lock (watchGate)
            switch (command)
            {
                case "start_watch":
                    watching = id;
                    if (stoppedEarly == id)
                        calls.StopAtStart();
                    stoppedEarly = null;
                    return Value(null);
                case "watch_progress":
                    return Value(watching == id && calls.Current is var (done, total) ? new JsonArray(done, total) : null);
                case "stop_watch":
                    if (id <= lastEnded && lastEnded > 0)
                        return Value(null);
                    if (watching == id)
                    {
                        calls.Stop();
                        // A Stop before the handler's own watch started holds once it starts.
                        calls.StopAtStart();
                    }
                    else
                        stoppedEarly = id;
                    return Value(null);
                default:
                    // end_watch: the handler ended; its Stop is forgotten, and it may come twice.
                    if (watching == id)
                        watching = null;
                    lastEnded = Math.Max(lastEnded, id);
                    if (stoppedEarly == id)
                        stoppedEarly = null;
                    calls.ForgetStop();
                    return Value(null);
            }
    }

    /// <summary>
    /// A handler waits for its long engine call on the library's worker
    /// thread: meanwhile the program answers the screen (what parts read,
    /// engines' frames), as a session lets go of the server while it waits.
    /// </summary>
    void WaitOutside(Task call)
    {
        if (!gate.IsHeldByCurrentThread)
        {
            ((IAsyncResult)call).AsyncWaitHandle.WaitOne();
            return;
        }
        gate.Exit();
        try
        {
            ((IAsyncResult)call).AsyncWaitHandle.WaitOne();
        }
        finally
        {
            gate.Enter();
        }
    }

    // -- Engines beside the screen -----------------------------------------------------------

    /// <summary>A new instance of the application's engine <paramref name="name"/> (<c>ScreenEngines.&lt;Name&gt;</c>): its number; none when it has no such engine.</summary>
    JsonObject? EngineOpen(string name)
    {
        var engines = app.GetTypes().FirstOrDefault(t => t.Name == "ScreenEngines");
        if (engines?.GetField(Members.Pascal(name), BindingFlags.Public | BindingFlags.Static)?.GetValue(null) is not Func<IFrames> make)
            return Value(null);
        var frames = make();
        var id = ++nextEngine;
        lock (this.engines)
            this.engines[id] = (frames, new Lock());
        return Value(id);
    }

    void EngineClose(ulong id)
    {
        (IFrames Frames, Lock Turn) engine;
        lock (engines)
            if (!engines.Remove(id, out engine))
                return;
        lock (engine.Turn)
            engine.Frames.Dispose();
    }

    JsonObject Engine(string command, JsonObject given)
    {
        var id = given["id"]?.GetValue<ulong>() ?? 0;
        (IFrames Frames, Lock Turn) engine;
        lock (engines)
            if (!engines.TryGetValue(id, out engine))
                return WireJson.Error("the engine is gone");
        lock (engine.Turn)
        {
            if (command == "engine_hears")
                return Value(new JsonObject { ["pointer"] = engine.Frames.Hears.Pointer, ["keys"] = engine.Frames.Hears.Keys });
            var input = new List<ScreenInput>();
            foreach (var item in given["input"] as JsonArray ?? [])
            {
                if (item?["pointer"] is JsonObject p)
                    input.Add(new ScreenInput.Pointer(p["kind"]?.GetValue<string>() ?? "", (long)Math.Round(p["x"]?.GetValue<double>() ?? 0), (long)Math.Round(p["y"]?.GetValue<double>() ?? 0)));
                else if (item?["key"] is JsonObject k)
                    input.Add(new ScreenInput.Key(k["kind"]?.GetValue<string>() ?? "", k["name"]?.GetValue<string>() ?? ""));
                else if (item?["hear"] is JsonObject h)
                    input.Add(new ScreenInput.Hear(h["name"]?.GetValue<string>() ?? "", [.. (h["values"] as JsonArray ?? []).Select(WireJson.Plain)]));
            }
            Drawn drawn;
            try
            {
                drawn = engine.Frames.Frame(given["ms"]?.GetValue<double>() ?? 0, input);
            }
            catch (Exception e)
            {
                return WireJson.Error(e.Message);
            }
            return Value(new JsonObject
            {
                ["shapes"] = drawn.Shapes.DeepClone(),
                ["tells"] = new JsonArray([.. drawn.Tells.Select(t => (JsonNode)new JsonObject
                {
                    ["tell"] = t.Tell,
                    ["values"] = new JsonArray([.. t.Values.Select(v => JsonNode.Parse(v.GetRawText()))]),
                })]),
            });
        }
    }
}
