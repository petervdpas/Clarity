// The browser's side of a server that shows Avalonia windows in browsers
// (DESIGN 7.6). The page has one window: the mirror shell shows a mirror of
// each window the session on the server keeps, the top one in place and
// dialogs over it, as the server orders them. A mirror runs no handler: what
// the person does goes to the server with the state they left, one request
// after the other, and the reply sets every window's state, opens and closes
// windows, says messages and asks questions, which the person answers here.

using System.Collections.ObjectModel;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Clarity.Runtime;

namespace Clarity.Avalonia;

/// <summary>How the mirror shell reaches the sessions: a request, and its reply.</summary>
public interface ISessionLink
{
    Task<JsonObject> Send(JsonObject request);
}

/// <summary>The sessions of the server the page came from, over HTTP.</summary>
public sealed class HttpSessionLink(HttpClient http) : ISessionLink
{
    public async Task<JsonObject> Send(JsonObject request)
    {
        using var content = new StringContent(request.ToJsonString(), Encoding.UTF8, "application/json");
        using var reply = await http.PostAsync(WebServer.SessionPath, content);
        return JsonNode.Parse(await reply.Content.ReadAsStringAsync()) as JsonObject ?? new JsonObject { ["error"] = "the server answered no JSON" };
    }
}

public sealed class MirrorShell : UserControl, IRemote
{
    readonly ISessionLink link;
    readonly Func<WindowViewModel, Control> view;
    readonly ContentControl content = new();
    readonly Panel over = new();
    string session = "";

    /// <summary>The mirror of each open window, by the server's number of it.</summary>
    readonly Dictionary<long, WindowViewModel> mirrors = [];
    readonly Dictionary<long, Control> views = [];
    readonly List<WindowViewModel> stack = [];
    readonly List<WindowViewModel> dialogs = [];

    /// <summary>The messages on screen.</summary>
    public ObservableCollection<Message> Messages { get; } = [];

    /// <summary>The window on top, in place.</summary>
    public WindowViewModel? Current => stack.LastOrDefault();

    /// <summary>The view of the window on top.</summary>
    public Control? Shown => content.Content as Control;

    /// <summary>The open dialogs, over the window on top.</summary>
    public IReadOnlyList<WindowViewModel> Dialogs => dialogs;

    /// <summary>The question asked now; none when none is.</summary>
    public Question? Asking { get; private set; }

    /// <summary>Done when every request sent so far has been answered and followed.</summary>
    public Task Idle { get; private set; } = Task.CompletedTask;

    MirrorShell(ISessionLink link, Func<WindowViewModel, Control> view, TimeSpan infoDuration)
    {
        this.link = link;
        this.view = view;
        Content = new Panel { Children = { content, over, new MessageList { Messages = Messages, InfoDuration = infoDuration } } };
    }

    /// <summary>Starts a session, and shows its main window.</summary>
    public static async Task<MirrorShell> Start(ISessionLink link, Func<WindowViewModel, Control> view, TimeSpan? infoDuration = null)
    {
        var shell = new MirrorShell(link, view, infoDuration ?? TimeSpan.FromSeconds(5));
        shell.Follow(await link.Send(new JsonObject { ["do"] = "start" }));
        return shell;
    }

    /// <summary>A mirror's command: to the server, with the state the person left in its window.</summary>
    public void Command(WindowViewModel window, string command, object?[] args)
    {
        var id = mirrors.FirstOrDefault(m => m.Value == window).Key;
        Send(new JsonObject
        {
            ["do"] = "run",
            ["window"] = id,
            ["command"] = command,
            ["args"] = Snapshot.Arguments(args),
            ["state"] = Snapshot.Of(window),
        });
    }

    /// <summary>The person closed a dialog: nothing goes back to the window that asked.</summary>
    public Task CloseDialog(WindowViewModel dialog)
    {
        var id = mirrors.FirstOrDefault(m => m.Value == dialog).Key;
        return Send(new JsonObject { ["do"] = "close", ["window"] = id });
    }

    /// <summary>Sends a request after the ones before it, and follows its reply.</summary>
    Task Send(JsonObject request)
    {
        async Task After(Task before)
        {
            await before;
            request["session"] = session;
            Follow(await link.Send(request));
        }
        return Idle = After(Idle);
    }

    /// <summary>What the server answered: every open window with its state, and what was said and asked.</summary>
    void Follow(JsonObject reply)
    {
        if (reply["error"]?.GetValue<string>() is { } error)
        {
            Messages.Add(new Message(Level.Error, Text.Plain(error)));
            return;
        }
        session = reply["session"]?.GetValue<string>() ?? session;
        var open = new HashSet<long>();
        stack.Clear();
        dialogs.Clear();
        foreach (var sent in reply["windows"]?.AsArray() ?? [])
        {
            var id = sent!["window"]!.GetValue<long>();
            open.Add(id);
            if (!mirrors.TryGetValue(id, out var mirror))
            {
                var type = Type.GetType(sent["type"]!.GetValue<string>()) ?? throw new SessionException($"no window {sent["type"]}");
                mirror = (WindowViewModel)Activator.CreateInstance(type, [null])!;
                mirror.Remote = this;
                mirror.Messages = Messages;
                mirrors[id] = mirror;
            }
            if (sent["state"] is JsonObject state)
                Snapshot.Apply(mirror, state);
            (sent["dialog"]?.GetValue<bool>() == true ? dialogs : stack).Add(mirror);
        }
        foreach (var gone in mirrors.Keys.Where(id => !open.Contains(id)).ToList())
        {
            mirrors.Remove(gone);
            views.Remove(gone);
        }
        content.Content = Current is { } top ? ViewOf(top) : null;
        over.Children.Clear();
        foreach (var dialog in dialogs)
            over.Children.Add(Framed(ViewOf(dialog)));
        foreach (var said in reply["said"]?.AsArray() ?? [])
            Say(new Message(Enum.Parse<Level>(said!["level"]!.GetValue<string>()), said["text"].Deserialize<Text>(Snapshot.Options)!));
        foreach (var asked in reply["asked"]?.AsArray() ?? [])
            Ask(asked!["question"]!.GetValue<long>(), asked["text"].Deserialize<Text>(Snapshot.Options)!, Snapshot.Form(asked["form"]));
    }

    Control ViewOf(WindowViewModel mirror)
    {
        var id = mirrors.First(m => m.Value == mirror).Key;
        if (!views.TryGetValue(id, out var shown))
            views[id] = shown = view(mirror);
        return shown;
    }

    /// <summary>A dialog over the window, as a desktop shows it in a window of its own.</summary>
    static Control Framed(Control shown) => new Grid
    {
        Background = new SolidColorBrush(Color.FromArgb(96, 0, 0, 0)),
        Children =
        {
            new Border
            {
                Child = shown,
                Padding = new global::Avalonia.Thickness(16),
                CornerRadius = new global::Avalonia.CornerRadius(8),
                Background = Brushes.White,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
            },
        },
    };

    /// <summary>A message: an info shows for a while; a warning or an error in Clarity's dialog until OK.</summary>
    void Say(Message message)
    {
        Messages.Add(message);
        if (message.Level == Level.Info)
            return;
        var said = new SaidDialog(message);
        var shown = Framed(said);
        said.Done += () =>
        {
            over.Children.Remove(shown);
            Messages.Remove(message);
        };
        over.Children.Add(shown);
    }

    /// <summary>A question in Clarity's dialog; the answer goes to the server, where its handler runs.</summary>
    void Ask(long number, Text text, AskForm form)
    {
        Control? shown = null;
        var question = new Question(text, form, answer =>
        {
            Asking = null;
            if (shown is not null)
                over.Children.Remove(shown);
            Send(new JsonObject { ["do"] = "answer", ["question"] = number, ["answer"] = JsonSerializer.SerializeToNode(answer, typeof(object), Snapshot.Options) });
        });
        Asking = question;
        var dialog = new AskDialog(question);
        shown = Framed(dialog);
        dialog.Answered += question.Answered;
        dialog.Cancelled += () =>
        {
            Asking = null;
            over.Children.Remove(shown);
        };
        over.Children.Add(shown);
    }
}
