// The windows of a program served to the browser (DESIGN 7.6, 7.9): each
// window the session on the server keeps has a mirror here, the same view
// model made without a database, drawn by Clarity.UI from the window's
// markup, which the server serves. The top window shows in place and the
// dialogs over it, as the server orders them. A mirror runs no handler: its
// commands come here (it is their IRemote) and go to the server with the
// state the person left, one request after the other; the reply sets every
// window's state, opens and closes windows, says messages, copies text and
// asks questions, which the person answers here. A command sent while the
// same one waits unsent (a timer that ticks faster than the server answers)
// takes the place of the one that waits. While a handler's request is out,
// the page asks how far its long call is and shows the bar with Stop.

using System.Collections.ObjectModel;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Xml;
using System.Xml.Linq;
using Clarity.Runtime;

namespace Clarity.Ui.Browser;

public sealed class MirrorStack : Element, IRemote
{
    readonly ISessionLink link;
    readonly Func<string, string?> carried;
    readonly (Func<JsonObject> Of, Action<JsonObject> Apply)? settings;
    readonly IReadOnlyList<XElement>? appFrame;
    string session = "";

    /// <summary>A window shown: its mirror, the host its parts read it through, and its view.</summary>
    sealed record Shown(long Id, WindowViewModel Model, ViewModelHost Host, WindowView View);

    /// <summary>The mirror of each open window, by the server's number of it.</summary>
    readonly Dictionary<long, WindowViewModel> mirrors = [];
    readonly Dictionary<long, Shown> views = [];
    readonly List<WindowViewModel> stack = [];
    readonly Dictionary<long, Overlay?> dialogs = [];
    readonly List<(long Number, Question Question)> unasked = [];

    /// <summary>
    /// A request that waits its turn: where its reply goes when it is not
    /// followed (a fetch), and the window whose state it takes as it is sent
    /// (a command), so it carries what the replies before it left.
    /// </summary>
    sealed record Waiting(JsonObject Request, TaskCompletionSource<JsonObject>? Reply, WindowViewModel? Window = null);

    readonly List<Waiting> waiting = [];
    bool sending;

    MirrorStack(ISessionLink link, Func<string, string?> carried, IReadOnlyList<string> names, (Func<JsonObject> Of, Action<JsonObject> Apply)? settings, Action<string>? openAddress, IFilePicker? files)
    {
        this.link = link;
        this.carried = carried;
        this.settings = settings;
        OpenAddress = openAddress;
        Files = files;
        appFrame = AppFrame(carried, names);
    }

    /// <summary>
    /// Reads the windows' markup from the server, starts a session and shows
    /// its main window; with the person's <paramref name="settings"/> when the
    /// application has them, which each request takes to the server.
    /// <paramref name="own"/> gives what the server did not send, from the
    /// application the program carries; addresses open through
    /// <paramref name="openAddress"/>, files through <paramref name="files"/>.
    /// </summary>
    public static async Task<MirrorStack> Start(ISessionLink link, (Func<JsonObject> Of, Action<JsonObject> Apply)? settings = null, Func<string, string?>? own = null, Action<string>? openAddress = null, IFilePicker? files = null)
    {
        var sent = await link.Markup();
        var markup = sent.Where(m => m.Value is JsonValue).ToDictionary(m => m.Key, m => m.Value!.GetValue<string>());
        var mirror = new MirrorStack(link, name => markup.GetValueOrDefault(name) ?? own?.Invoke(name), [.. markup.Keys], settings, openAddress, files);
        await mirror.Send(new JsonObject { ["do"] = "start" });
        return mirror;
    }

    /// <summary>A resource of the application as the server sent it (<c>Windows/Main.cxaml</c>, <c>Themes/Clarity.light.json</c>), else as the program carries it; none when neither has it.</summary>
    public string? Carried(string name) => carried(name);

    /// <summary>How the host opens an address (a link's).</summary>
    public Action<string>? OpenAddress { get; }

    /// <summary>The page's file input and downloads, which every window gives its parts; none picks nothing.</summary>
    public IFilePicker? Files { get; }

    /// <summary>The messages on screen, of every window.</summary>
    public ObservableCollection<Message> Messages { get; } = [];

    /// <summary>The window on top, in place.</summary>
    public WindowViewModel? Current => stack.Count > 0 ? stack[^1] : null;

    /// <summary>The view of the window on top.</summary>
    public WindowView? View => Current is { } top && views.TryGetValue(IdOf(top), out var shown) ? shown.View : null;

    /// <summary>The open dialogs, over the window on top.</summary>
    public IReadOnlyList<WindowViewModel> Dialogs => [.. dialogs.Keys.Select(id => mirrors[id])];

    /// <summary>The view of an open window or dialog; none when it is not open.</summary>
    public WindowView? ViewOf(WindowViewModel model) => views.TryGetValue(IdOf(model), out var shown) ? shown.View : null;

    /// <summary>The question asked now and not answered; none when none is.</summary>
    public Question? Asking { get; private set; }

    /// <summary>Done when every request sent so far has been answered and followed.</summary>
    public Task Idle { get; private set; } = Task.CompletedTask;

    /// <summary>Another window is on top: the page shows its title.</summary>
    public event Action? TitleChanged;

    long IdOf(WindowViewModel model) => mirrors.FirstOrDefault(m => m.Value == model).Key;

    // -- What the person does --------------------------------------------------------

    /// <summary>A mirror's command: to the server, with the state the person left in its window.</summary>
    public void Command(WindowViewModel window, string command, object?[] args)
    {
        // What the window does when it opens, closes or is on top again is the server's: it ran there.
        if (command is nameof(WindowViewModel.Opened) or nameof(WindowViewModel.Closing) or nameof(WindowViewModel.Returned))
            return;
        _ = Send(new JsonObject
        {
            ["do"] = "run",
            ["window"] = IdOf(window),
            ["command"] = command,
            ["args"] = Snapshot.Arguments(args),
        }, window);
    }

    /// <summary>A picture or a file of the database: the server fetches it, after what was sent before.</summary>
    public Task<byte[]?> Fetch(WindowViewModel window, string fetcher, object?[] args)
    {
        var reply = new TaskCompletionSource<JsonObject>();
        Queue(new Waiting(new JsonObject { ["do"] = "fetch", ["window"] = IdOf(window), ["fetcher"] = fetcher, ["args"] = Snapshot.Arguments(args) }, reply));
        return Fetched(reply.Task);

        static async Task<byte[]?> Fetched(Task<JsonObject> replied) =>
            (await replied)["bytes"]?.GetValue<string>() is { } data ? Convert.FromBase64String(data) : null;
    }

    /// <summary>The person closed a dialog without an answer: nothing goes back to the window that asked.</summary>
    public Task CloseDialog(WindowViewModel dialog) => Send(new JsonObject { ["do"] = "close", ["window"] = IdOf(dialog) });

    /// <summary>The window's own keys: the window on top's (shortcuts, the default and cancel buttons). True when it took the key.</summary>
    public bool KeyPressed(KeyInput key) => View?.KeyPressed(key) ?? false;

    /// <summary>A key let go: the window on top's (Alt for its menu bar).</summary>
    public bool KeyReleased(KeyInput key) => View?.KeyReleased(key) ?? false;

    /// <summary>Time went on: the window on top's info messages go when their time is up.</summary>
    public void Tick() => View?.Tick();

    // -- The requests, one after the other -------------------------------------------------

    /// <summary>Sends a request after the ones before it, and follows its reply.</summary>
    Task Send(JsonObject request, WindowViewModel? window = null)
    {
        // The same command of the same window waits unsent: it goes once, with the state as it is then.
        if (request["do"]?.GetValue<string>() == "run" && waiting.Count > 0 && waiting[^1] is { Reply: null } last && Same(last.Request, request))
            return Idle;
        Queue(new Waiting(request, null, window));
        return Idle;
    }

    /// <summary>The language and the formats the page shows, which the server's handlers take for this person.</summary>
    internal static JsonObject LocaleNow() => new() { ["language"] = Locale.Current, ["formats"] = Locale.Formats.Culture.Name };

    static bool Same(JsonObject a, JsonObject b) =>
        a["do"]?.GetValue<string>() == "run"
        && a["window"]?.GetValue<long>() == b["window"]?.GetValue<long>()
        && a["command"]?.GetValue<string>() == b["command"]?.GetValue<string>()
        && JsonNode.DeepEquals(a["args"], b["args"]);

    void Queue(Waiting request)
    {
        waiting.Add(request);
        if (!sending)
            Idle = Pump();
    }

    async Task Pump()
    {
        sending = true;
        try
        {
            while (waiting.Count > 0)
            {
                var (request, reply, window) = waiting[0];
                waiting.RemoveAt(0);
                if (window is not null)
                {
                    // A window the server closed meanwhile has nothing left to run.
                    if (!mirrors.ContainsValue(window))
                        continue;
                    request["state"] = Snapshot.Of(window);
                }
                if (session != "")
                    request["session"] = session;
                if (settings is { } kept)
                    request["settings"] = kept.Of().DeepClone();
                // The server answers in the person's language and formats, which the page shows.
                request["locale"] = LocaleNow();
                JsonObject answer;
                try
                {
                    answer = await Answered(request);
                }
                catch (Exception e) when (e is HttpRequestException or JsonException or TaskCanceledException or InvalidOperationException)
                {
                    // Fail safe: the server cannot be reached; the window says so and goes on.
                    answer = new JsonObject { ["error"] = e.Message };
                }
                if (reply is not null)
                    reply.SetResult(answer);
                else
                    Follow(answer, request);
            }
        }
        finally
        {
            sending = false;
        }
    }

    // -- A long engine call ----------------------------------------------------------------

    /// <summary>How often the page asks how far a long call is while a handler's request is out.</summary>
    internal static TimeSpan PollEvery { get; set; } = TimeSpan.FromMilliseconds(250);

    /// <summary>How far the session's long call is, as the server last said; none while it says nothing.</summary>
    (long Done, long Total)? progress;

    /// <summary>The bar of the session's long call, over the window while the server says how far it is; none otherwise.</summary>
    public LongCallBar? CallBar { get; private set; }

    Overlay? callBarOverlay;

    /// <summary>
    /// The reply to <paramref name="request"/>. While a request that runs a
    /// handler is out, the page asks the server how far the session's long
    /// call is: once it says, the bar shows over the window, as on the
    /// desktop, until the reply comes; its Stop stops the call on the server.
    /// </summary>
    async Task<JsonObject> Answered(JsonObject request)
    {
        var replied = link.Send(request);
        if (request["do"]?.GetValue<string>() is not ("run" or "answer" or "close") || session == "")
            return await replied;
        try
        {
            while (await Task.WhenAny(replied, Task.Delay(PollEvery)) != replied)
            {
                JsonObject said;
                try
                {
                    said = await link.Send(new JsonObject { ["do"] = "progress", ["session"] = session });
                }
                catch (Exception e) when (e is HttpRequestException or JsonException or TaskCanceledException or InvalidOperationException)
                {
                    // Fail safe: a question that is not answered asks again; the request itself goes on.
                    continue;
                }
                if (said["waiting"]?.GetValue<bool>() == true && said["progress"] is JsonArray { Count: 2 } at)
                {
                    progress = (at[0]!.GetValue<long>(), at[1]!.GetValue<long>());
                    ShowCallBar();
                }
            }
            return await replied;
        }
        finally
        {
            progress = null;
            HideCallBar();
        }
    }

    void ShowCallBar()
    {
        if (CallBar is null)
        {
            // The bar is known only once it shows, so whoever sees CallBar sees it on the screen.
            var bar = new LongCallBar(() => progress, StopCall);
            if (Screen is { } screen)
            {
                callBarOverlay = new Overlay(bar, screen.Whole) { Modal = true, Covers = true };
                screen.Open(callBarOverlay);
            }
            CallBar = bar;
        }
        Invalidate();
    }

    void HideCallBar()
    {
        if (CallBar is null)
            return;
        if (callBarOverlay is { } shown)
            Screen?.Close(shown);
        CallBar = null;
        callBarOverlay = null;
        Invalidate();
    }

    /// <summary>The person's Stop: the session's long call ends at its next look.</summary>
    void StopCall() => _ = Stop();

    async Task Stop()
    {
        try
        {
            await link.Send(new JsonObject { ["do"] = "stop", ["session"] = session });
        }
        catch (Exception e) when (e is HttpRequestException or JsonException or TaskCanceledException or InvalidOperationException)
        {
            // Fail safe: the server cannot be reached; the bar's Stop has greyed, the call ends as it does.
            Console.Error.WriteLine($"the long call's stop did not reach the server: {e.Message}");
        }
    }

    // -- What the server answered ---------------------------------------------------------

    /// <summary>Every open window with its state, and what was said, copied and asked.</summary>
    void Follow(JsonObject reply, JsonObject? request = null)
    {
        if (reply["error"]?.GetValue<string>() is { } error)
        {
            Messages.Add(new Message(Level.Error, Text.Plain(error)));
            Invalidate();
            return;
        }
        session = reply["session"]?.GetValue<string>() ?? session;
        // What a handler changed of the person's settings, which their browser keeps.
        // Only what the server changed of them: a setting the person changed while the request was out stays.
        if (settings is { } kept && reply["settings"] is JsonObject changed)
            kept.Apply(request?["settings"] is JsonObject before ? Changed(before, changed) : changed);
        var sentFrom = request?["window"]?.GetValue<long>();
        var top = Current;
        var open = new List<long>();
        var openDialogs = new List<long>();
        stack.Clear();
        foreach (var sent in reply["windows"]?.AsArray() ?? [])
        {
            if (sent is not JsonObject window)
                continue;
            var id = window["window"]!.GetValue<long>();
            if (!mirrors.TryGetValue(id, out var mirror))
            {
                var type = Type.GetType(window["type"]!.GetValue<string>()) ?? throw new SessionException($"no window {window["type"]}");
                mirror = (WindowViewModel)Activator.CreateInstance(type, [null])!;
                mirror.Remote = this;
                mirror.Messages = Messages;
                mirrors[id] = mirror;
            }
            if (window["state"] is JsonObject state)
            {
                // The window that sent its state keeps what the person did while the request was out, where the handler left it as it was.
                if (id == sentFrom && request?["state"] is JsonObject before)
                    state = Merged(before, state, Snapshot.Of(mirror));
                Snapshot.Apply(mirror, state);
            }
            if (window["dialog"]?.GetValue<bool>() == true)
                openDialogs.Add(id);
            else
            {
                open.Add(id);
                stack.Add(mirror);
            }
        }
        // The windows the server closed go, with their dialogs' overlays.
        foreach (var gone in mirrors.Keys.Where(id => !open.Contains(id) && !openDialogs.Contains(id)).ToList())
        {
            if (dialogs.Remove(gone, out var overlay) && overlay is not null)
                Screen?.Close(overlay);
            if (views.Remove(gone, out var shown))
            {
                if (shown.View.Parent == this)
                    RemoveChild(shown.View);
                shown.View.Dispose();
                shown.Host.Dispose();
            }
            mirrors.Remove(gone);
        }
        foreach (var id in openDialogs)
            dialogs.TryAdd(id, null);
        if (Current != top)
            ShowTop();
        foreach (var said in reply["said"]?.AsArray() ?? [])
            Messages.Add(new Message(Enum.Parse<Level>(said!["level"]!.GetValue<string>()), said["text"].Deserialize<Text>(Snapshot.Options)!));
        // What a handler on the server copied goes on the page's clipboard, as it shows.
        if (reply["copy"].Deserialize<Text>(Snapshot.Options) is { } copied && Screen is { } screen)
            screen.Clipboard.SetText(Locale.Show(copied));
        foreach (var asked in reply["asked"]?.AsArray() ?? [])
        {
            var number = asked!["question"]!.GetValue<long>();
            var question = new Question(asked["text"].Deserialize<Text>(Snapshot.Options)!, Snapshot.Form(asked["form"]), answer => Answer(number, answer));
            unasked.Add((number, question));
            Asking = question;
        }
        ShowOver();
        Invalidate();
    }

    /// <summary>The values of <paramref name="replied"/> that differ from those <paramref name="before"/>.</summary>
    internal static JsonObject Changed(JsonObject before, JsonObject replied)
    {
        var changed = new JsonObject();
        foreach (var (key, value) in replied)
        {
            if (!JsonNode.DeepEquals(before[key], value))
                changed[key] = value?.DeepClone();
        }
        return changed;
    }

    /// <summary>
    /// A window's state as the server replied it, except where the handler
    /// left a value as it was <paramref name="before"/> the request: there the
    /// window's value <paramref name="now"/> stays, which the person may have
    /// changed meanwhile. Data field by field, each part's state value by value.
    /// </summary>
    internal static JsonObject Merged(JsonObject before, JsonObject replied, JsonObject now)
    {
        static JsonObject Level(JsonObject? before, JsonObject replied, JsonObject? now, Func<string, JsonNode?, JsonNode?, JsonNode?, JsonNode?> each)
        {
            var merged = new JsonObject();
            foreach (var (key, value) in replied)
                merged[key] = each(key, before?[key], value, now?[key])?.DeepClone();
            return merged;
        }
        JsonNode? Leaf(string key, JsonNode? was, JsonNode? replied, JsonNode? now) =>
            now is not null && JsonNode.DeepEquals(was, replied) ? now : replied;
        return Level(before, replied, now, (section, wasSection, repliedSection, nowSection) => (section, repliedSection) switch
        {
            ("data", JsonObject data) => Level(wasSection as JsonObject, data, nowSection as JsonObject, Leaf),
            ("parts", JsonObject parts) => Level(wasSection as JsonObject, parts, nowSection as JsonObject, (_, wasPart, repliedPart, nowPart) =>
                repliedPart is JsonObject part ? Level(wasPart as JsonObject, part, nowPart as JsonObject, Leaf) : repliedPart),
            _ => repliedSection,
        });
    }

    /// <summary>The window on top shows in place; the page takes its title.</summary>
    void ShowTop()
    {
        foreach (var child in views.Values.Where(v => v.View.Parent == this).ToList())
            RemoveChild(child.View);
        if (Current is { } top)
        {
            var shown = ShownOf(top);
            AddChild(shown.View);
            shown.View.Arrange(Bounds);
            BrowserPage.Current?.SetTitle(top.Title);
        }
        Screen?.Focus(null);
        TitleChanged?.Invoke();
    }

    Shown ShownOf(WindowViewModel model)
    {
        var id = IdOf(model);
        if (!views.TryGetValue(id, out var shown))
            views[id] = shown = Make(id, model);
        return shown;
    }

    /// <summary>The dialogs and questions not shown yet, over the window, once there is a screen to show them on.</summary>
    void ShowOver()
    {
        if (Screen is not { } screen)
            return;
        foreach (var id in dialogs.Where(d => d.Value is null).Select(d => d.Key).ToList())
        {
            var model = mirrors[id];
            var view = ShownOf(model).View;
            var overlay = new Overlay(new DialogOverlay(model, view), Bounds) { Modal = true, Covers = true };
            dialogs[id] = overlay;
            screen.Open(overlay);
            screen.Focus(view.InOrder().FirstOrDefault(e => e.Focusable && e.ShowsInTree));
        }
        foreach (var (number, question) in unasked.ToList())
        {
            unasked.RemoveAll(u => u.Number == number);
            var ask = new AskDialog(question);
            var overlay = Clarity.Ui.Dialogs.Open(screen, ask, ask.First);
            ask.Answered = answer =>
            {
                screen.Close(overlay);
                question.Answered(answer);
            };
            ask.Cancelled = () =>
            {
                screen.Close(overlay);
                if (Asking == question)
                    Asking = null;
            };
        }
    }

    /// <summary>The person answered: the handler the answer goes to runs on the server.</summary>
    void Answer(long number, object? answer)
    {
        Asking = null;
        unasked.RemoveAll(u => u.Number == number);
        _ = Send(new JsonObject { ["do"] = "answer", ["question"] = number, ["answer"] = JsonSerializer.SerializeToNode(answer, typeof(object), Snapshot.Options) });
    }

    /// <summary>On a screen: what waited for one shows.</summary>
    protected override void OnAttached() => ShowOver();

    // -- Drawing a window from its markup ---------------------------------------------------

    Shown Make(long id, WindowViewModel model)
    {
        var name = $"Windows/{WindowStack.NameOf(model)}.cxaml";
        string? problem = null;
        WindowSpec spec;
        var columns = WindowView.StandardColumns;
        try
        {
            var text = Carried(name) ?? throw new FormatException($"the server sent no markup for the window {WindowStack.NameOf(model)}");
            spec = WindowMarkup.Read(text);
            columns = WindowView.ColumnsOf(text);
        }
        catch (Exception e) when (e is FormatException or XmlException)
        {
            // Fail safe: the window says why it shows nothing, and the application goes on.
            Console.Error.WriteLine(e.Message);
            spec = new WindowSpec([], []);
            problem = e.Message;
        }
        var host = new ViewModelHost(model, spec) { Opener = OpenAddress, Files = Files ?? new NoFilePicker() };
        // A dialog says what it says in the window under it, so a message shows once.
        var view = new WindowView(spec, host, model.IsDialog ? null : model, columns) { Problem = problem };
        if (appFrame is { } frame && !model.IsDialog)
            view.AppFrame = frame;
        return new Shown(id, model, host, view);
    }

    /// <summary>The regions of the app's frame the server sent (<c>Apps/Shop.cxaml</c>); none when it sent none or one that cannot be read.</summary>
    static IReadOnlyList<XElement>? AppFrame(Func<string, string?> carried, IReadOnlyList<string> names)
    {
        var name = names.Where(n => n.StartsWith("Apps/", StringComparison.Ordinal) && n.EndsWith(".cxaml", StringComparison.Ordinal)).Order(StringComparer.Ordinal).FirstOrDefault();
        if (name is null || carried(name) is not { } text)
            return null;
        try
        {
            return WindowMarkup.ReadApp(text);
        }
        catch (Exception e) when (e is FormatException or XmlException)
        {
            Console.Error.WriteLine($"the app's frame {name} cannot be read: {e.Message}");
            return null;
        }
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        View?.Arrange(box);
    }
}
