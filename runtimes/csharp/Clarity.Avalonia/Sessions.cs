// The sessions of a server that shows Avalonia windows in browsers
// (DESIGN 7.6). Each browser tab has one: its windows live here, where their
// handlers run against the server's database, opened, closed and returned to
// as the desktop shell does it in its one window (a window marked `in new
// window` opens in place, since a page has one). The browser keeps a mirror
// of each window: it sends what the person did, with the state they left,
// and gets every open window's state back, with what was said and asked.
// One request at a time, as every stack runs one handler at a time.

using System.Collections.Specialized;
using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public sealed class Sessions(Func<WindowViewModel> main, TimeSpan? quiet = null, Func<DateTime>? clock = null)
{
    readonly Dictionary<string, Session> sessions = [];
    readonly TimeSpan quiet = quiet ?? TimeSpan.FromMinutes(30);
    readonly Func<DateTime> now = clock ?? (() => DateTime.UtcNow);
    readonly Lock gate = new();

    /// <summary>
    /// The application's settings, one for the whole program, as each person's
    /// browser keeps their own (DESIGN 7.6): a request brings the person's,
    /// which the handlers then read, and the reply takes back what they changed.
    /// None for an application without settings.
    /// </summary>
    public (Func<JsonObject> Of, Action<JsonObject> Apply)? Settings { get; init; }

    /// <summary>
    /// Answers one request of a browser: <c>start</c> a session; <c>run</c> a
    /// window's command with the state the person left; <c>answer</c> a
    /// question; <c>close</c> a dialog the person closed. The reply holds every
    /// open window, bottom first, dialogs last, with its state.
    /// </summary>
    public JsonObject Answer(JsonObject request)
    {
        lock (gate)
        {
            EndQuiet();
            try
            {
                return Do(request);
            }
            catch (Exception e) when (e is SessionException or ClarityException or JsonException or InvalidOperationException or System.Reflection.TargetInvocationException)
            {
                var why = e is System.Reflection.TargetInvocationException { InnerException: { } inner } ? inner : e;
                return new JsonObject { ["error"] = why.Message };
            }
        }
    }

    JsonObject Do(JsonObject request)
    {
        // The person's settings, before anything of theirs runs.
        if (Settings is { } settings && request["settings"] is JsonObject theirs)
            settings.Apply(theirs);
        var reply = DoWith(request);
        if (Settings is { } kept && !reply.ContainsKey("error"))
            reply["settings"] = kept.Of();
        return reply;
    }

    JsonObject DoWith(JsonObject request)
    {
        var what = request["do"]?.GetValue<string>();
        if (what == "start")
        {
            var started = new Session(Guid.NewGuid().ToString("N"), now());
            sessions[started.Id] = started;
            started.Push(main());
            return started.Reply();
        }
        var id = request["session"]?.GetValue<string>() ?? "";
        if (!sessions.TryGetValue(id, out var session))
            throw new SessionException("the session has ended; start again");
        session.LastSeen = now();
        switch (what)
        {
            case "run":
                var window = session.Window(request["window"]?.GetValue<long>() ?? 0);
                if (request["state"] is JsonObject state)
                    Snapshot.Apply(window, state);
                var command = request["command"]?.GetValue<string>() ?? "";
                Snapshot.Run(window, command, Snapshot.Arguments(window, command, request["args"] as JsonArray));
                break;
            case "answer":
                session.Answer(request["question"]?.GetValue<long>() ?? 0, request["answer"]?.Deserialize<object>(Snapshot.Options));
                break;
            case "fetch":
                // A picture or a file of the database, by the fetcher of the window that shows it.
                var shows = session.Window(request["window"]?.GetValue<long>() ?? 0);
                var bytes = Snapshot.Fetch(shows, request["fetcher"]?.GetValue<string>() ?? "", request["args"] as JsonArray);
                return new JsonObject { ["session"] = session.Id, ["bytes"] = bytes is null ? null : Convert.ToBase64String(bytes) };
            case "close":
                session.CloseDialog(session.Window(request["window"]?.GetValue<long>() ?? 0));
                break;
            default:
                throw new SessionException($"no request {what}");
        }
        var reply = session.Reply();
        if (session.Ended)
            sessions.Remove(session.Id);
        return reply;
    }

    /// <summary>Ends the sessions quiet longer than they may be: their windows close.</summary>
    void EndQuiet()
    {
        foreach (var session in sessions.Values.Where(s => now() - s.LastSeen > quiet).ToList())
        {
            session.CloseAll();
            sessions.Remove(session.Id);
        }
    }

    /// <summary>One tab's windows.</summary>
    sealed class Session(string id, DateTime started)
    {
        public string Id { get; } = id;
        public DateTime LastSeen { get; set; } = started;
        public bool Ended { get; private set; }

        readonly List<WindowViewModel> stack = [];
        readonly List<WindowViewModel> dialogs = [];
        /// <summary>The questions not answered yet, by their number: the window that asked, and the question.</summary>
        readonly Dictionary<long, (long Window, Question Asked)> questions = [];
        long nextQuestion;
        /// <summary>The questions asked since the last reply.</summary>
        readonly List<long> asked = [];
        readonly System.Collections.ObjectModel.ObservableCollection<Message> messages = [];
        readonly List<Message> said = [];
        /// The text a handler copied since the last reply, for the browser's clipboard.
        Text? copied;
        bool listening;

        IEnumerable<WindowViewModel> Open => stack.Concat(dialogs);

        public WindowViewModel Window(long id) =>
            Open.FirstOrDefault(w => w.Instance == id) ?? throw new SessionException($"window {id} is not open");

        void Attach(WindowViewModel model)
        {
            if (!listening)
                messages.CollectionChanged += Said;
            listening = true;
            model.Messages = messages;
            model.OpenRequested += (other, _) =>
            {
                if (other.IsDialog)
                    OpenDialog(other, model);
                else
                    Push(other);
            };
            model.CloseRequested += () => Close(model);
            model.AskRequested += question =>
            {
                var at = ++nextQuestion;
                questions[at] = (model.Instance, question);
                asked.Add(at);
            };
            model.CopyRequested += text => copied = text;
        }

        void Said(object? sender, NotifyCollectionChangedEventArgs e) => said.AddRange(e.NewItems?.OfType<Message>() ?? []);

        public void Push(WindowViewModel model)
        {
            Attach(model);
            stack.Add(model);
            model.Opened();
        }

        void OpenDialog(WindowViewModel dialog, WindowViewModel opener)
        {
            Attach(dialog);
            dialogs.Add(dialog);
            dialog.CloseRequested += () => FinishDialog(dialog, opener);
            dialog.Opened();
        }

        void FinishDialog(WindowViewModel dialog, WindowViewModel opener)
        {
            if (!dialogs.Remove(dialog))
                return;
            dialog.Closing();
            // The window below it is told: the one that opened it, if it is still open.
            (Open.Contains(opener) ? opener : Open.LastOrDefault())?.Returned();
        }

        /// <summary>The person closed a dialog without an answer: nothing runs on the window that asked.</summary>
        public void CloseDialog(WindowViewModel dialog)
        {
            if (!dialog.IsDialog)
                throw new SessionException("only a dialog closes this way");
            var opener = stack.LastOrDefault();
            if (opener is not null)
                FinishDialog(dialog, opener);
        }

        void Close(WindowViewModel model)
        {
            // A window closes only from the top; a dialog closes by its own event.
            if (model.IsDialog || stack.LastOrDefault() != model)
                return;
            stack.RemoveAt(stack.Count - 1);
            model.Closing();
            if (stack.Count == 0)
            {
                Ended = true;
                return;
            }
            stack[^1].Returned();
        }

        /// <summary>The person answered: the handler the answer goes to runs on the window that asked.</summary>
        public void Answer(long question, object? answer)
        {
            if (!questions.Remove(question, out var asked))
                throw new SessionException("the question was answered already");
            Window(asked.Window).Answer(asked.Asked, answer);
        }

        public void CloseAll()
        {
            foreach (var window in Open.Reverse().ToList())
                window.Closing();
            stack.Clear();
            dialogs.Clear();
        }

        /// <summary>Every open window with its state, and what was said and asked since the last reply.</summary>
        public JsonObject Reply()
        {
            var windows = new JsonArray();
            foreach (var window in Open)
            {
                windows.Add(new JsonObject
                {
                    ["window"] = window.Instance,
                    ["type"] = window.GetType().AssemblyQualifiedName,
                    ["dialog"] = window.IsDialog,
                    ["state"] = Snapshot.Of(window, told: true),
                });
            }
            var questionsAsked = new JsonArray();
            foreach (var at in asked.Where(questions.ContainsKey))
            {
                var (owner, question) = questions[at];
                questionsAsked.Add(new JsonObject
                {
                    ["question"] = at,
                    ["window"] = owner,
                    ["text"] = JsonSerializer.SerializeToNode(question.Text, Snapshot.Options),
                    ["form"] = JsonSerializer.SerializeToNode(question.Form, typeof(object), Snapshot.Options),
                });
            }
            var saidNow = new JsonArray();
            foreach (var message in said)
                saidNow.Add(new JsonObject { ["level"] = message.Level.ToString(), ["text"] = JsonSerializer.SerializeToNode(message.Text, Snapshot.Options) });
            said.Clear();
            messages.Clear();
            var reply = new JsonObject { ["session"] = Id, ["windows"] = windows, ["asked"] = questionsAsked, ["said"] = saidNow, ["ended"] = Ended };
            if (copied is { } text)
                reply["copy"] = JsonSerializer.SerializeToNode(text, Snapshot.Options);
            copied = null;
            asked.Clear();
            return reply;
        }
    }
}

/// <summary>What a browser asked that the server cannot do: a session that ended, a window that is not open, a command a window has not.</summary>
public sealed class SessionException(string why) : Exception(why);
