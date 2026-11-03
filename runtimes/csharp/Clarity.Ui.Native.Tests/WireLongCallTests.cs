// What code asks of the screen beyond its window, and long engine calls, as
// the C# window model answers the library: quitting and the keys window from
// code (the reply's quit and keys), a window opened in a window of its own
// (the open's new_window), and a long call whose handler runs on the
// library's worker thread while the screen asks how far it is and stops it
// (watches, start_watch, watch_progress, stop_watch, end_watch). First the
// wire alone, with the library's threads played by the test; then through
// the library itself, driven without a window.

using System.Text.Json.Nodes;
using System.Windows.Input;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Native.Tests;

public sealed class LongViewModel() : WindowViewModel(Db.Empty())
{
    public const string Markup = """
        <Window name="Long">
          <Button name="quitButton" at="1,1" span="3" caption="Quit" onclick="quitNow" />
          <Button name="keysButton" at="4,1" span="3" caption="Keys" onclick="keysNow" />
          <Button name="apartButton" at="7,1" span="3" caption="Apart" onclick="openApart" />
          <Button name="longButton" at="10,1" span="3" caption="Long" onclick="long" />
        </Window>
        """;

    public override string Title => "Long";

    // Fields, not properties: they are what the test sees, no values of the window.
    /// <summary>Whether the long call was stopped, and the thread its handler ran on.</summary>
    public volatile bool Stopped;
    public volatile int RanOn;

    /// <summary>Set once the long call said how far it is, so the test knows it waits.</summary>
    public readonly ManualResetEventSlim Reported = new();

    public ICommand QuitNowCommand => new RelayCommand(() => Handle(QuitApplication));
    public ICommand KeysNowCommand => new RelayCommand(() => Handle(ShowKeys));
    public ICommand OpenApartCommand => new RelayCommand(() => Handle(() => Open(new OtherViewModel(), newWindow: true)));
    public ICommand LongCommand => new RelayCommand(() => Handle(OnLong));

    // As a generated handler calls an engine that says how far it is: the call
    // on another thread, reporting 30 of 120, running until it is stopped (at most 10 s).
    void OnLong()
    {
        RanOn = Environment.CurrentManagedThreadId;
        var watch = Watching.Current!;
        var call = Task.Run(() =>
        {
            watch.Progress!(30, 120);
            Reported.Set();
            var until = DateTime.UtcNow.AddSeconds(10);
            while (!watch.Stop.IsCancellationRequested && DateTime.UtcNow < until)
                Thread.Sleep(5);
            return watch.Stop.IsCancellationRequested;
        });
        watch.Wait!(call);
        Stopped = call.Result;
    }
}

/// <summary>The tests that set the one desktop program (DesktopProgram.Current) run one at a time.</summary>
[CollectionDefinition("Desktop program", DisableParallelization = true)]
public sealed class DesktopProgramCollection;

[Collection("Desktop program")]
public sealed class WireLongCallTests
{
    static (ProgramWire Wire, LongViewModel Long) Started(bool watches = true)
    {
        var main = new LongViewModel();
        var markups = new Dictionary<string, string> { ["Long"] = LongViewModel.Markup, ["Other"] = OtherViewModel.Markup };
        return (new ProgramWire(typeof(LongViewModel).Assembly, main, n => markups.GetValueOrDefault(n), watches: watches), main);
    }

    static JsonNode? Value(JsonObject? answer)
    {
        Assert.NotNull(answer);
        Assert.Null(answer["error"]);
        return answer["value"];
    }

    static JsonObject Open(ProgramWire wire) => Value(wire.Answer("open_window", """{"window":"Long","with":{}}"""))!["state"]!.AsObject();

    static JsonObject Dispatch(ProgramWire wire, JsonObject state, string handler) =>
        Value(wire.Answer("dispatch_event", new JsonObject { ["state"] = state.DeepClone(), ["handler"] = handler }.ToJsonString()))!.AsObject();

    static JsonNode? Watch(ProgramWire wire, string command, ulong id) => Value(wire.Answer(command, $$"""{"id":{{id}}}"""));

    /// <summary>Runs <paramref name="action"/> with the wire as the desktop program, as the desktop does.</summary>
    static void AsDesktop(ProgramWire wire, Action action)
    {
        DesktopProgram.Current = wire;
        try
        {
            action();
        }
        finally
        {
            DesktopProgram.Current = null;
        }
    }

    // -- Quit, keys, a window of its own ------------------------------------------------

    [Fact]
    public void Quitting_from_code_says_quit_in_the_reply_once_and_closes_no_window_itself()
    {
        var (wire, _) = Started();
        AsDesktop(wire, () =>
        {
            var state = Open(wire);
            var reply = Dispatch(wire, state, "quitNow");
            Assert.True(reply["quit"]!.GetValue<bool>());
            Assert.False(reply["close"]!.GetValue<bool>());
            Assert.Null(Dispatch(wire, state, "keysNow")["quit"]);
        });
    }

    [Fact]
    public void The_keys_window_from_code_is_said_in_the_reply_once()
    {
        var (wire, _) = Started();
        AsDesktop(wire, () =>
        {
            var state = Open(wire);
            Assert.True(Dispatch(wire, state, "keysNow")["keys"]!.GetValue<bool>());
            var again = Dispatch(wire, state, "openApart");
            Assert.Null(again["keys"]);
            Assert.Null(again["quit"]);
        });
    }

    [Fact]
    public void A_window_opened_in_new_window_says_so_in_its_open_and_one_in_place_does_not()
    {
        var (wire, _) = Started();
        var state = Open(wire);
        var open = Dispatch(wire, state, "openApart")["open"]!.AsArray();
        Assert.Equal("Other", open[0]!["window"]!.GetValue<string>());
        Assert.True(open[0]!["new_window"]!.GetValue<bool>());
        Assert.NotNull(open[0]!["with"]![ProgramWire.WindowToken]);
    }

    // -- Long calls --------------------------------------------------------------------

    [Fact]
    public void The_program_watches_only_when_an_engine_says_how_far_it_is()
    {
        Assert.True(Value(Started(watches: true).Wire.Answer("watches", "{}"))!.GetValue<bool>());
        Assert.False(Value(Started(watches: false).Wire.Answer("watches", "{}"))!.GetValue<bool>());
        // The test assembly carries no generated Engines class: it watches nothing.
        var main = new LongViewModel();
        Assert.False(Value(new ProgramWire(typeof(LongViewModel).Assembly, main, _ => null).Answer("watches", "{}"))!.GetValue<bool>());
    }

    /// <summary>
    /// As the library runs a watched handler: on its worker thread between
    /// start_watch and end_watch, while the screen's thread asks how far it is,
    /// reads what parts read (the program lets go while the call waits), and stops it.
    /// </summary>
    [Fact]
    public void A_long_call_on_the_worker_thread_says_how_far_it_is_and_stops_from_the_screen_s_thread()
    {
        var (wire, model) = Started();
        var state = Open(wire);
        Assert.Null(Watch(wire, "watch_progress", 5));
        JsonObject? reply = null;
        var worker = new Thread(() =>
        {
            Watch(wire, "start_watch", 5);
            reply = Dispatch(wire, state, "long");
            Watch(wire, "end_watch", 5);
        });
        worker.Start();
        Assert.True(model.Reported.Wait(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken), "the long call reports");
        var progress = Watch(wire, "watch_progress", 5)!.AsArray();
        Assert.Equal((30L, 120L), (progress[0]!.GetValue<long>(), progress[1]!.GetValue<long>()));
        // Another watch's progress is not this call's.
        Assert.Null(Watch(wire, "watch_progress", 4));
        // While the call waits the program answers the screen.
        Assert.Null(wire.Answer("ref_options", """{"entity":"Person"}""")?["error"]);
        Watch(wire, "stop_watch", 5);
        Assert.True(worker.Join(TimeSpan.FromSeconds(5)), "Stop ends the call");
        Assert.True(model.Stopped);
        Assert.NotEqual(Environment.CurrentManagedThreadId, model.RanOn);
        Assert.NotNull(reply);
        Assert.Null(Watch(wire, "watch_progress", 5));
        Assert.False(wire.Calls.Waiting);
        // end_watch comes once more from the screen's thread; a late Stop of the ended watch stops nothing next.
        Watch(wire, "end_watch", 5);
        Watch(wire, "stop_watch", 5);
        model.Stopped = false;
        model.Reported.Reset();
        var next = new Thread(() =>
        {
            Watch(wire, "start_watch", 6);
            Dispatch(wire, state, "long");
            Watch(wire, "end_watch", 6);
        });
        next.Start();
        Assert.True(model.Reported.Wait(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.False(model.Stopped);
        Watch(wire, "stop_watch", 6);
        Assert.True(next.Join(TimeSpan.FromSeconds(5)));
        Assert.True(model.Stopped);
    }

    /// <summary>The library shows Stop as it asks for the call: a Stop before its start_watch holds once the call starts.</summary>
    [Fact]
    public void A_stop_before_the_watch_started_holds_once_it_starts()
    {
        var (wire, model) = Started();
        var state = Open(wire);
        Watch(wire, "stop_watch", 9);
        var worker = new Thread(() =>
        {
            Watch(wire, "start_watch", 9);
            Dispatch(wire, state, "long");
            Watch(wire, "end_watch", 9);
        });
        worker.Start();
        Assert.True(worker.Join(TimeSpan.FromSeconds(5)), "the call starts stopped");
        Assert.True(model.Stopped);
    }

    // -- Through the library -----------------------------------------------------------

    static Driven Run(LongViewModel main, bool keys = false)
    {
        var markups = new Dictionary<string, string> { ["Long"] = LongViewModel.Markup, ["Other"] = OtherViewModel.Markup };
        var wire = new ProgramWire(typeof(LongViewModel).Assembly, main, n => markups.GetValueOrDefault(n), watches: true);
        DesktopProgram.Current = wire;
        var files = new NativeFiles();
        files.Window("Long", LongViewModel.Markup);
        files.Window("Other", OtherViewModel.Markup);
        files.Title("Long");
        if (keys)
            files.Model("""{"project": {"locale": {"language": "en"}, "keysEditable": true, "files": []}, "keys": []}""");
        return new Driven(wire.Answer, files);
    }

    static JsonObject Do(Driven driven, string step, JsonObject? more = null)
    {
        var json = new JsonObject { ["do"] = step };
        foreach (var (key, value) in more ?? [])
            json[key] = value?.DeepClone();
        var answer = driven.Step(json);
        Assert.True(answer["ok"]?.GetValue<bool>() ?? false, $"{json.ToJsonString()}: {answer.ToJsonString()}");
        return answer;
    }

    /// <summary>
    /// Through the library, a click's handler runs on the library's worker
    /// thread with its watch, and the bar asks how far it is. A driven run
    /// waits for work off the screen's thread at each step, so Stop is
    /// pressed in the wire's test above and live.
    /// </summary>
    [Fact]
    public void Through_the_library_a_click_s_handler_runs_watched_on_the_library_s_worker_thread()
    {
        var model = new LongViewModel();
        var seen = new List<(string Command, int Thread)>();
        var markups = new Dictionary<string, string> { ["Long"] = LongViewModel.Markup, ["Other"] = OtherViewModel.Markup };
        var wire = new ProgramWire(typeof(LongViewModel).Assembly, model, n => markups.GetValueOrDefault(n), watches: true);
        var files = new NativeFiles();
        files.Window("Long", LongViewModel.Markup);
        files.Window("Other", OtherViewModel.Markup);
        files.Title("Long");
        using var driven = new Driven((command, args) =>
        {
            lock (seen)
                seen.Add((command, Environment.CurrentManagedThreadId));
            // The call runs only a moment: a Stop as it starts.
            if (command == "start_watch")
                wire.Answer("stop_watch", args);
            return wire.Answer(command, args);
        }, files);
        Do(driven, "click", new() { ["part"] = "longButton" });
        Do(driven, "settle", new() { ["ms"] = 200 });
        Assert.True(model.Stopped, "the Stop before its start held");
        Assert.NotEqual(Environment.CurrentManagedThreadId, model.RanOn);
        List<(string Command, int Thread)> calls;
        lock (seen)
            calls = [.. seen];
        var start = calls.FindIndex(c => c.Command == "start_watch");
        Assert.True(start >= 0, string.Join(", ", calls.Select(c => c.Command)));
        Assert.Equal(("dispatch_event", calls[start].Thread), calls[start + 1]);
        Assert.Equal(model.RanOn, calls[start].Thread);
        Assert.Contains(calls, c => c.Command == "watch_progress");
    }

    /// <summary>Clarity's own words (an engine stopped) go with their English, which the library shows where the project has no translation.</summary>
    [Fact]
    public void Clarity_s_own_words_go_with_their_english()
    {
        var said = WireJson.Text(Clarity.Runtime.ClarityException.Own("engineStopped", "{engine} was stopped", ("engine", "Worker")))!;
        Assert.Equal("engineStopped", said["key"]!.GetValue<string>());
        Assert.Equal("{engine} was stopped", said["text"]!.GetValue<string>());
        Assert.Equal("Worker", said["values"]!["engine"]!["value"]!.GetValue<string>());
        // A project's own key carries no English of its own.
        Assert.Null(WireJson.Text(new Text("saveCaption") { Key = "saveCaption" })!["text"]);
    }

    [Fact]
    public void Through_the_library_code_opens_the_keys_window_and_quits()
    {
        var model = new LongViewModel();
        try
        {
            using var driven = Run(model, keys: true);
            Assert.False(Do(driven, "read")["keys"]!.GetValue<bool>());
            Do(driven, "click", new() { ["part"] = "keysButton" });
            Do(driven, "settle", new() { ["ms"] = 200 });
            Assert.True(Do(driven, "read")["keys"]!.GetValue<bool>());
            Do(driven, "key", new() { ["key"] = "Escape" });
            Do(driven, "click", new() { ["part"] = "quitButton" });
            Do(driven, "settle", new() { ["ms"] = 200 });
            var read = Do(driven, "read");
            Assert.True(read["quit"]!.GetValue<bool>(), read.ToJsonString());
            Assert.True(read["ended"]!.GetValue<bool>());
        }
        finally
        {
            DesktopProgram.Current = null;
        }
    }
}
