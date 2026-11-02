// Windows the tests open: a shelf of books, a book, a pick and a window that
// reads a setting; and the sessions reached as a browser reaches them, as
// JSON text through the server, in the same process.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using Clarity.Ui.Browser;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Server.Tests;

sealed partial class ShelfViewModel(Db? db) : WindowViewModel(db)
{
    public override string Title => "Shelf";

    public List<string> Log { get; set; } = [];
    public long Steps { get; set; }
    public PartState Name { get; } = new("name");
    public PartState Clock { get; } = new("clock");

    protected override void OnOpen() => Log.Add("opened");

    protected override void OnReturn() => Log.Add("back");

    [RelayCommand]
    void Greet() => Handle(() => Say(Level.Info, Text.Plain("Hello " + Name.Text)));

    [RelayCommand]
    void CopyName() => Handle(() => Copy(Text.Plain("Name: " + Name.Text)));

    [RelayCommand]
    void Stepped() => Handle(() => Steps++);

    [RelayCommand]
    void Detail() => Handle(() => Open(new BookViewModel(Db)));

    [RelayCommand]
    void Pick() => Handle(() => Ask(new PickViewModel(Db), picked => Log.Add($"picked {picked}")));

    [RelayCommand]
    void Remove() => Handle(() => Ask(Text.Plain("Remove it?"), new ConfirmForm(null, null), _ => Log.Add("removed")));
}

sealed partial class BookViewModel(Db? db) : WindowViewModel(db)
{
    /// <summary>The book's cover, from the server as a generated window fetches it.</summary>
    public Func<Task<byte[]?>> CoverPicture => () => Fetched(nameof(CoverPicture), () => [1, 2, 3]);

    /// <summary>A file of the store, by its path.</summary>
    public Func<string, Task<byte[]?>> CoverFile => path => Fetched(nameof(CoverFile), () => path == "a.png" ? [4] : null, path);

    public override string Title => "Book";

    [RelayCommand]
    void Back() => Handle(Close);
}

sealed partial class PickViewModel(Db? db) : WindowViewModel(db)
{
    public override bool IsDialog => true;

    [RelayCommand]
    void Choose(string what) => Handle(() => CloseWith(what), [what]);
}

/// <summary>A person's settings, one at a time on the server, as a generated Settings is.</summary>
static class Prefs
{
    public static int Speed { get; set; } = 1;

    public static (Func<JsonObject> Of, Action<JsonObject> Apply) Hooks => (() => new JsonObject { ["speed"] = Speed }, sent => Speed = sent["speed"]?.GetValue<int>() ?? Speed);
}

sealed partial class FasterViewModel(Db? db) : WindowViewModel(db)
{
    public int Seen { get; set; }

    [RelayCommand]
    void Faster() => Handle(() =>
    {
        Seen = Prefs.Speed;
        Prefs.Speed++;
    });
}

/// <summary>
/// The server reached as the browser reaches it: each request and reply as
/// JSON text through its answer, and the markup from its markup path. With a
/// gate, a request waits for it before the server answers, as a slow server does.
/// </summary>
sealed class InProcess(WebServer server) : ISessionLink
{
    public InProcess(Func<WindowViewModel> main) : this(new WebServer(new Sessions(main), "", typeof(InProcess).Assembly)) { }

    /// <summary>What each request asked, in the order sent.</summary>
    public List<string> Sent { get; } = [];

    /// <summary>The commands run, in the order sent.</summary>
    public List<string> Commands { get; } = [];

    public TaskCompletionSource? Gate { get; set; }

    public async Task<JsonObject> Send(JsonObject request)
    {
        Sent.Add(request["do"]!.GetValue<string>());
        if (request["command"]?.GetValue<string>() is { } command)
            Commands.Add(command);
        if (Gate is { } gate)
            await gate.Task;
        var reply = server.Answer("POST", WebServer.SessionPath, request.ToJsonString());
        return JsonNode.Parse(reply.Body)!.AsObject();
    }

    public Task<JsonObject> Markup() =>
        Task.FromResult(JsonNode.Parse(server.Answer("GET", WebServer.MarkupPath, "").Body)!.AsObject());
}
