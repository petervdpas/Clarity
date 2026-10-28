// The sessions of a server that shows Avalonia windows in browsers
// (DESIGN 7.6): each tab's windows live on the server, where their handlers
// run, as the desktop shell runs them; the browser sends what the person
// did and gets the windows' state back.

using System.Text.Json.Nodes;
using Clarity.Avalonia;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Avalonia.Tests;

sealed partial class ShelfViewModel(Db? db) : WindowViewModel(db)
{
    public List<string> Log { get; set; } = [];
    public PartState Name { get; } = new("name");

    protected override void OnOpen() => Log.Add("opened");

    protected override void OnReturn() => Log.Add("back");

    [RelayCommand]
    void Greet() => Handle(() => Say(Level.Info, Text.Plain("Hello " + Name.Text)));

    [RelayCommand]
    void Detail() => Handle(() => Open(new BookViewModel(Db)));

    [RelayCommand]
    void Pick() => Handle(() => Ask(new PickViewModel(Db), picked => Log.Add($"picked {picked}")));

    [RelayCommand]
    void Remove() => Handle(() => Ask(Text.Plain("Remove it?"), new ConfirmForm(null, null), _ => Log.Add("removed")));
}

sealed partial class BookViewModel(Db? db) : WindowViewModel(db)
{
    [RelayCommand]
    void Back() => Handle(Close);
}

sealed partial class PickViewModel(Db? db) : WindowViewModel(db)
{
    public override bool IsDialog => true;

    [RelayCommand]
    void Choose(string what) => Handle(() => CloseWith(what), [what]);
}

public class SessionTests
{
    static JsonObject Request(string json) => JsonNode.Parse(json)!.AsObject();

    static JsonObject Run(Sessions sessions, string session, long window, string command, string args = "[]", JsonObject? state = null) =>
        sessions.Answer(Request($$"""{"session": "{{session}}", "do": "run", "window": {{window}}, "command": "{{command}}", "args": {{args}}}""").Also(r => { if (state is not null) r["state"] = state; }));

    static long[] Windows(JsonObject reply) => [.. reply["windows"]!.AsArray().Select(w => w!["window"]!.GetValue<long>())];

    static string Type(JsonObject reply, int at) => reply["windows"]![at]!["type"]!.GetValue<string>();

    [Fact]
    public void A_tab_starts_a_session_with_the_main_window_opened()
    {
        using var db = Db.Memory();
        var sessions = new Sessions(() => new ShelfViewModel(db));
        var reply = sessions.Answer(Request("""{"do": "start"}"""));
        Assert.False(string.IsNullOrEmpty(reply["session"]?.GetValue<string>()));
        Assert.Single(Windows(reply));
        Assert.StartsWith(typeof(ShelfViewModel).FullName!, Type(reply, 0));
        // It opened as on the desktop: its on open ran, and its state shows it.
        Assert.Equal("opened", reply["windows"]![0]!["state"]!["data"]!["Log"]![0]!.GetValue<string>());
    }

    [Fact]
    public void A_command_runs_on_the_state_the_person_left()
    {
        using var db = Db.Memory();
        var sessions = new Sessions(() => new ShelfViewModel(db));
        var start = sessions.Answer(Request("""{"do": "start"}"""));
        var session = start["session"]!.GetValue<string>();
        var main = Windows(start)[0];
        // The person typed a name in the browser's mirror.
        var state = start["windows"]![0]!["state"]!.DeepClone().AsObject();
        state["parts"]!["Name"]!["text"] = "Ada";
        var reply = Run(sessions, session, main, "Greet", state: state);
        var said = reply["said"]!.AsArray().Single()!;
        Assert.Equal("Info", said["level"]!.GetValue<string>());
        Assert.Equal("Hello Ada", said["text"]!["template"]!.GetValue<string>());
    }

    [Fact]
    public void Windows_open_in_place_and_close_back_to_the_one_below()
    {
        using var db = Db.Memory();
        var sessions = new Sessions(() => new ShelfViewModel(db));
        var start = sessions.Answer(Request("""{"do": "start"}"""));
        var session = start["session"]!.GetValue<string>();
        var main = Windows(start)[0];
        var opened = Run(sessions, session, main, "Detail");
        Assert.Equal(2, Windows(opened).Length);
        Assert.StartsWith(typeof(BookViewModel).FullName!, Type(opened, 1));
        var closed = Run(sessions, session, Windows(opened)[1], "Back");
        Assert.Equal([main], Windows(closed));
        // The window below is on top again, and was told.
        Assert.Equal(["opened", "back"], closed["windows"]![0]!["state"]!["data"]!["Log"]!.AsArray().Select(l => l!.GetValue<string>()));
    }

    [Fact]
    public void A_dialog_closes_with_its_value_to_the_window_that_asked()
    {
        using var db = Db.Memory();
        var sessions = new Sessions(() => new ShelfViewModel(db));
        var start = sessions.Answer(Request("""{"do": "start"}"""));
        var session = start["session"]!.GetValue<string>();
        var main = Windows(start)[0];
        var asked = Run(sessions, session, main, "Pick");
        Assert.True(asked["windows"]![1]!["dialog"]!.GetValue<bool>());
        var dialog = Windows(asked)[1];
        var chosen = Run(sessions, session, dialog, "Choose", """["blue"]""");
        Assert.Equal([main], Windows(chosen));
        Assert.Contains("picked blue", chosen["windows"]![0]!["state"]!["data"]!["Log"]!.AsArray().Select(l => l!.GetValue<string>()));
    }

    [Fact]
    public void A_question_goes_to_the_browser_and_its_answer_comes_back()
    {
        using var db = Db.Memory();
        var sessions = new Sessions(() => new ShelfViewModel(db));
        var start = sessions.Answer(Request("""{"do": "start"}"""));
        var session = start["session"]!.GetValue<string>();
        var asked = Run(sessions, session, Windows(start)[0], "Remove");
        var question = asked["asked"]!.AsArray().Single()!;
        Assert.Equal("Remove it?", question["text"]!["template"]!.GetValue<string>());
        Assert.IsType<ConfirmForm>(Snapshot.Form(question["form"]));
        var answered = sessions.Answer(Request($$"""{"session": "{{session}}", "do": "answer", "question": {{question["question"]}}, "answer": null}"""));
        Assert.Contains("removed", answered["windows"]![0]!["state"]!["data"]!["Log"]!.AsArray().Select(l => l!.GetValue<string>()));
    }

    [Fact]
    public void An_unknown_session_or_command_is_an_error_not_a_crash()
    {
        using var db = Db.Memory();
        var sessions = new Sessions(() => new ShelfViewModel(db));
        Assert.Equal("the session has ended; start again", sessions.Answer(Request("""{"session": "nope", "do": "run", "window": 1, "command": "Greet"}"""))["error"]!.GetValue<string>());
        var start = sessions.Answer(Request("""{"do": "start"}"""));
        var reply = Run(sessions, start["session"]!.GetValue<string>(), Windows(start)[0], "Nothing");
        Assert.Equal("the window has no command Nothing", reply["error"]!.GetValue<string>());
    }

    [Fact]
    public void A_quiet_session_ends_and_closes_its_windows()
    {
        using var db = Db.Memory();
        var now = DateTime.UtcNow;
        var sessions = new Sessions(() => new ShelfViewModel(db), TimeSpan.FromMinutes(30), () => now);
        var start = sessions.Answer(Request("""{"do": "start"}"""));
        var session = start["session"]!.GetValue<string>();
        now += TimeSpan.FromMinutes(31);
        sessions.Answer(Request("""{"do": "start"}"""));
        Assert.Equal("the session has ended; start again", Run(sessions, session, Windows(start)[0], "Greet")["error"]!.GetValue<string>());
    }
}

static class JsonObjectExtensions
{
    public static JsonObject Also(this JsonObject o, Action<JsonObject> change)
    {
        change(o);
        return o;
    }
}
