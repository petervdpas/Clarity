// Each person on a server sees their own language and formats (DESIGN 7.6):
// a request brings the language and the formats the person's page shows,
// and what their handler builds outside a slot (a text made from a key, a
// number written out) is in them, as the desktop shows its one person's.
// Two sessions in two languages; one waits for a long call meanwhile and
// goes on in its own language after it.

using System.Globalization;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Runtime.Tests;

sealed partial class GreetingViewModel(Db? db) : WindowViewModel(db)
{
    /// <summary>What a long call waits for; none runs no call.</summary>
    public static Task? Call { get; set; }

    /// <summary>The handler waits for its call: the request has let go of the server.</summary>
    public static TaskCompletionSource Waiting { get; } = new();

    public string Before { get; set; } = "";
    public string After { get; set; } = "";
    public string Amount { get; set; } = "";

    [RelayCommand]
    void Greet() => Handle(() =>
    {
        Before = Locale.Show(Text.Of("greeting"));
        if (Call is { } call)
        {
            Waiting.TrySetResult();
            Watching.Current!.Wait!(call);
        }
        After = Locale.Show(Text.Of("greeting"));
        Amount = Locale.Formats.Culture.Name + " " + 1.5m.ToString(CultureInfo.CurrentCulture);
    });
}

[Collection(nameof(SaidTextTests))]
public class SessionLanguageTests
{
    static readonly Dictionary<string, string> Dutch = new() { ["greeting"] = "Hallo" };
    static readonly Dictionary<string, string> English = new() { ["greeting"] = "Hello" };

    static JsonObject Started(Sessions sessions) => sessions.Answer(new JsonObject { ["do"] = "start" });

    static JsonObject Greet(Sessions sessions, JsonObject started, string language, string formats) => sessions.Answer(new JsonObject
    {
        ["do"] = "run",
        ["session"] = started["session"]!.GetValue<string>(),
        ["window"] = started["windows"]![0]!["window"]!.GetValue<long>(),
        ["command"] = "Greet",
        ["locale"] = new JsonObject { ["language"] = language, ["formats"] = formats },
    });

    static string Data(JsonObject reply, string name) => reply["windows"]![0]!["state"]!["data"]![name]!.GetValue<string>();

    [Fact]
    public async Task Each_session_s_handler_runs_in_its_person_s_language_and_formats()
    {
        Locale.Start("en", ["nl"], null, (key, culture) => (culture.Name == "nl" ? Dutch : English).GetValueOrDefault(key), (_, value) => value, CultureInfo.InvariantCulture);
        var gate = new TaskCompletionSource();
        try
        {
            var sessions = new Sessions(() => new GreetingViewModel(null));
            var dutch = Started(sessions);
            var english = Started(sessions);

            // The Dutch person's handler waits for a long call; the English person's runs meanwhile.
            GreetingViewModel.Call = gate.Task;
            var waiting = Task.Run(() => Greet(sessions, dutch, "nl", "nl-NL"), TestContext.Current.CancellationToken);
            await GreetingViewModel.Waiting.Task.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
            GreetingViewModel.Call = null;
            var theirs = Greet(sessions, english, "en", "en-US");
            Assert.Equal("Hello", Data(theirs, "Before"));
            Assert.Equal("en-US 1.5", Data(theirs, "Amount"));

            gate.SetResult();
            var mine = await waiting.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
            Assert.Null(mine["error"]);
            Assert.Equal("Hallo", Data(mine, "Before"));
            Assert.Equal("Hallo", Data(mine, "After"));
            Assert.Equal("nl-NL 1,5", Data(mine, "Amount"));

            // The application's own language stays, outside the requests.
            Assert.Null(Locale.Language);
            Assert.Equal("Hello", Locale.Show(Text.Of("greeting")));
        }
        finally
        {
            gate.TrySetResult();
            GreetingViewModel.Call = null;
            Locale.Reset();
        }
    }

    [Fact]
    public void A_language_the_application_does_not_offer_shows_its_own()
    {
        Locale.Start("en", ["nl"], null, (key, culture) => (culture.Name == "nl" ? Dutch : English).GetValueOrDefault(key), (_, value) => value, CultureInfo.InvariantCulture);
        try
        {
            var sessions = new Sessions(() => new GreetingViewModel(null));
            var reply = Greet(sessions, Started(sessions), "fr", "no-such-culture-xx");
            Assert.Null(reply["error"]);
            Assert.Equal("Hello", Data(reply, "Before"));
        }
        finally
        {
            Locale.Reset();
        }
    }
}
