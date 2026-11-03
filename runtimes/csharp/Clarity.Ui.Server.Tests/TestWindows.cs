// Windows the tests open: a shelf that greets and steps, and a window whose
// handler waits for a long engine call; and the program's commands sent as
// the library in the browser sends them.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Server.Tests;

sealed partial class ShelfViewModel(Db? db) : WindowViewModel(db)
{
    public override string Title => "Shelf";

    public override Extent? WindowSize => new(800, 600);

    public long Steps { get; set; }
    public PartState Name { get; } = new("name");

    [RelayCommand]
    void Greet() => Handle(() => Say(Level.Info, Text.Plain("Hello " + Name.Text)));

    [RelayCommand]
    void Stepped() => Handle(() => Steps++);

    /// <summary>The person's settings, as a generated window shows them.</summary>
    public ISettings Settings => ShelfSettings.Current;

    /// <summary>What the last handler saw: the speed setting, and the culture it ran in.</summary>
    public string Seen { get; set; } = "";

    [RelayCommand]
    void Look() => Handle(() => Seen = $"{ShelfSettings.Current.Speed} {System.Globalization.CultureInfo.CurrentCulture.Name}");

    [RelayCommand]
    void Faster() => Handle(() => ShelfSettings.Current.Speed++);

    [RelayCommand]
    void QuitNow() => Handle(QuitApplication);

    [RelayCommand]
    void KeysNow() => Handle(ShowKeys);
}

/// <summary>Settings as a generated app writes them: one object, each setting a property.</summary>
public sealed class ShelfSettings : ISettings
{
    public static ShelfSettings Current { get; } = new();

    public long Speed { get; set; } = 1;
    public string Greeting { get; set; } = "hi";

    public JsonObject ToJson() => new() { ["speed"] = Speed, ["greeting"] = Greeting };

    public void Apply(JsonObject sent)
    {
        Speed = sent["speed"]?.GetValue<long>() ?? Speed;
        Greeting = sent["greeting"]?.GetValue<string>() ?? Greeting;
    }
}

sealed partial class CrunchViewModel(Db? db) : WindowViewModel(db)
{
    /// <summary>Lets the long call end.</summary>
    public static readonly ManualResetEventSlim Done = new();

    public long Steps { get; set; }
    public bool Stopped { get; set; }

    [RelayCommand]
    void Stepped() => Handle(() => Steps++);

    /// <summary>As a handler's engine call does: it reports 30 of 120 from its own thread and runs until it is stopped or let end (at most 20 s).</summary>
    [RelayCommand]
    void Crunch() => Handle(() =>
    {
        var watch = Watching.Current!;
        var call = Task.Run(() =>
        {
            watch.Progress!(30, 120);
            var until = DateTime.UtcNow.AddSeconds(20);
            while (!watch.Stop.IsCancellationRequested && !Done.IsSet && DateTime.UtcNow < until)
                Thread.Sleep(5);
            Stopped = watch.Stop.IsCancellationRequested;
        });
        watch.Wait!(call);
    });
}

/// <summary>The application's engines say how far a long call is, as a generated Engines does.</summary>
public static class Engines
{
    public const bool Watches = true;
}

static class Wire
{
    /// <summary>A command as the worker posts it: <c>{"command", "args"}</c>, with the page that asks and its settings when given.</summary>
    public static string Command(string command, JsonObject? args = null, string? browser = null, JsonObject? settings = null)
    {
        var sent = new JsonObject { ["command"] = command, ["args"] = args ?? [] };
        if (browser is not null)
            sent["browser"] = browser;
        if (settings is not null)
            sent["settings"] = settings;
        return sent.ToJsonString();
    }

    /// <summary>A handler of the window <paramref name="instance"/> named <paramref name="window"/>, with its state, under watch <paramref name="watch"/> if one is given.</summary>
    public static string Dispatch(string window, long instance, string handler, JsonObject? data = null, ulong? watch = null, string? browser = null, JsonObject? settings = null)
    {
        var args = new JsonObject
        {
            ["state"] = new JsonObject { ["window"] = window, ["instance"] = instance, ["data"] = data ?? [] },
            ["handler"] = handler,
        };
        if (watch is { } id)
            args["watch"] = id;
        return Command("dispatch_event", args, browser, settings);
    }
}
