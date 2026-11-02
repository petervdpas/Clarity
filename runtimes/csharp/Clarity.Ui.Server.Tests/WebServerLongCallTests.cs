// A long engine call on the server belongs to the session that runs it (DESIGN
// 5.12, 7.6): while one person's handler waits for its call, the other
// sessions' requests are answered, their handlers run and see no progress,
// and a Stop stops only the call of the session that sent it.

using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Ui.Server.Tests;

sealed partial class CrunchViewModel(Db? db) : WindowViewModel(db)
{
    public long Steps { get; set; }
    public bool Stopped { get; set; }

    [RelayCommand]
    void Stepped() => Handle(() => Steps++);

    /// <summary>As a handler's engine call does: it reports 30 of 120 from its own thread and runs until it is stopped (at most 20 s).</summary>
    [RelayCommand]
    void Crunch() => Handle(() =>
    {
        var watch = Watching.Current!;
        var call = Task.Run(() =>
        {
            watch.Progress!(30, 120);
            var until = DateTime.UtcNow.AddSeconds(20);
            while (!watch.Stop.IsCancellationRequested && DateTime.UtcNow < until)
                Thread.Sleep(5);
            Stopped = watch.Stop.IsCancellationRequested;
        });
        watch.Wait!(call);
    });
}

public class WebServerLongCallTests
{
    /// <summary>Sends one session request on a connection of its own and reads the reply's body.</summary>
    static async Task<JsonObject> Ask(int port, JsonObject sent)
    {
        var body = sent.ToJsonString();
        var request = Encoding.UTF8.GetBytes($"POST {WebServer.SessionPath} HTTP/1.1\r\nContent-Length: {Encoding.UTF8.GetByteCount(body)}\r\nConnection: close\r\n\r\n{body}");
        using var client = new TcpClient();
        await client.ConnectAsync("127.0.0.1", port, TestContext.Current.CancellationToken);
        var stream = client.GetStream();
        await stream.WriteAsync(request, TestContext.Current.CancellationToken);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        var reply = await reader.ReadToEndAsync(TestContext.Current.CancellationToken).WaitAsync(TimeSpan.FromSeconds(20), TestContext.Current.CancellationToken);
        return JsonNode.Parse(reply[(reply.IndexOf("\r\n\r\n", StringComparison.Ordinal) + 4)..])!.AsObject();
    }

    static JsonObject Run(string session, JsonObject started, string command) => new()
    {
        ["do"] = "run",
        ["session"] = session,
        ["window"] = started["windows"]![0]!["window"]!.GetValue<long>(),
        ["command"] = command,
    };

    [Fact]
    public async Task A_long_call_waits_only_in_its_own_session_and_stops_only_there()
    {
        var server = new WebServer(new Sessions(() => new CrunchViewModel(null)), Path.GetTempPath());
        using var listening = server.Start("127.0.0.1:0");
        var port = listening.Port;
        var first = await Ask(port, new JsonObject { ["do"] = "start" });
        var second = await Ask(port, new JsonObject { ["do"] = "start" });
        var a = first["session"]!.GetValue<string>();
        var b = second["session"]!.GetValue<string>();

        // The first person's long call runs; its reply comes once it ended.
        var crunching = Task.Run(() => Ask(port, Run(a, first, "Crunch")), TestContext.Current.CancellationToken);
        JsonObject? progress = null;
        var until = DateTime.UtcNow.AddSeconds(10);
        while (DateTime.UtcNow < until && (progress = await Ask(port, new JsonObject { ["do"] = "progress", ["session"] = a }))["progress"] is null)
            await Task.Delay(10, TestContext.Current.CancellationToken);
        Assert.Equal("[30,120]", progress!["progress"]!.ToJsonString());
        Assert.True(progress["waiting"]!.GetValue<bool>());

        // The second person's handler runs meanwhile, and they see no progress.
        var stepped = await Ask(port, Run(b, second, "Stepped"));
        Assert.Null(stepped["error"]);
        Assert.Equal(1, stepped["windows"]![0]!["state"]!["data"]!["Steps"]!.GetValue<long>());
        var theirs = await Ask(port, new JsonObject { ["do"] = "progress", ["session"] = b });
        Assert.Null(theirs["progress"]);
        Assert.False(theirs["waiting"]!.GetValue<bool>());

        // The first person's own other requests wait for the call to end.
        var meanwhile = await Ask(port, Run(a, first, "Stepped"));
        Assert.NotNull(meanwhile["error"]);

        // The second person's Stop stops nothing of the first's call.
        await Ask(port, new JsonObject { ["do"] = "stop", ["session"] = b });
        await Task.Delay(100, TestContext.Current.CancellationToken);
        Assert.False(crunching.IsCompleted);

        // The first person's Stop ends it; the window was stopped, its bar gone.
        var stopping = await Ask(port, new JsonObject { ["do"] = "stop", ["session"] = a });
        Assert.Equal(a, stopping["session"]!.GetValue<string>());
        var done = await crunching.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
        Assert.Null(done["error"]);
        Assert.True(done["windows"]![0]!["state"]!["data"]!["Stopped"]!.GetValue<bool>());
        Assert.Equal(0, done["windows"]![0]!["state"]!["data"]!["Steps"]!.GetValue<long>());
        Assert.Null((await Ask(port, new JsonObject { ["do"] = "progress", ["session"] = a }))["progress"]);
    }
}
