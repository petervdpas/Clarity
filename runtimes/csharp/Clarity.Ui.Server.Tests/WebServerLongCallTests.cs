// A long engine call on the server belongs to the browser that runs it
// (DESIGN 5.12, 7.6): while one browser's handler waits for its call, the
// server answers the others, whose handlers run; that browser's own windows
// take no other handler until the call ended. The watch its handler's
// command names says how far it is and stops it, from requests of their
// own, a Stop that came before the handler started too.

using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using Xunit;

namespace Clarity.Ui.Server.Tests;

public class WebServerLongCallTests
{
    /// <summary>Sends one command on a connection of its own and reads the reply's body.</summary>
    static async Task<JsonObject> Ask(int port, string body)
    {
        var request = Encoding.UTF8.GetBytes($"POST {WebServer.InvokePath} HTTP/1.1\r\nContent-Length: {Encoding.UTF8.GetByteCount(body)}\r\nConnection: close\r\n\r\n{body}");
        using var client = new TcpClient();
        await client.ConnectAsync("127.0.0.1", port, TestContext.Current.CancellationToken);
        var stream = client.GetStream();
        await stream.WriteAsync(request, TestContext.Current.CancellationToken);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        var reply = await reader.ReadToEndAsync(TestContext.Current.CancellationToken).WaitAsync(TimeSpan.FromSeconds(20), TestContext.Current.CancellationToken);
        return JsonNode.Parse(reply[(reply.IndexOf("\r\n\r\n", StringComparison.Ordinal) + 4)..])!.AsObject();
    }

    static long Instance(JsonObject opened) => opened["value"]!["state"]!["instance"]!.GetValue<long>();

    [Fact]
    public async Task A_long_call_holds_only_the_browser_that_runs_it()
    {
        CrunchViewModel.Done.Reset();
        using var listening = new WebServer(new CrunchViewModel(null)).Start("127.0.0.1:0");
        var port = listening.Port;
        var open = Wire.Command("open_window", new JsonObject { ["window"] = "Crunch" });
        var a = Instance(await Ask(port, open));
        var b = Instance(await Ask(port, open));

        // The first browser's long call runs; its reply comes once it ended.
        var crunching = Task.Run(() => Ask(port, Wire.Dispatch("Crunch", a, "crunch")), TestContext.Current.CancellationToken);
        await Task.Delay(200, TestContext.Current.CancellationToken);
        Assert.False(crunching.IsCompleted);

        // The second browser's handler runs meanwhile.
        var stepped = await Ask(port, Wire.Dispatch("Crunch", b, "stepped"));
        Assert.Null(stepped["error"]);
        Assert.Equal(1, stepped["value"]!["state"]!["data"]!["steps"]!.GetValue<long>());

        // The first browser's own window takes no other handler while its call runs.
        var meanwhile = await Ask(port, Wire.Dispatch("Crunch", a, "stepped"));
        Assert.Equal(0, meanwhile["value"]!["state"]!["data"]!["steps"]!.GetValue<long>());

        CrunchViewModel.Done.Set();
        var done = await crunching.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
        Assert.Null(done["error"]);
        var after = await Ask(port, Wire.Dispatch("Crunch", a, "stepped"));
        Assert.Equal(1, after["value"]!["state"]!["data"]!["steps"]!.GetValue<long>());
    }

    [Fact]
    public async Task A_watch_says_how_far_its_call_is_and_stops_it_even_before_it_started()
    {
        CrunchViewModel.Done.Reset();
        using var listening = new WebServer(new CrunchViewModel(null)).Start("127.0.0.1:0");
        var port = listening.Port;
        var a = Instance(await Ask(port, Wire.Command("open_window", new JsonObject { ["window"] = "Crunch" })));
        var b = Instance(await Ask(port, Wire.Command("open_window", new JsonObject { ["window"] = "Crunch" })));

        var crunching = Task.Run(() => Ask(port, Wire.Dispatch("Crunch", a, "crunch", watch: 5)), TestContext.Current.CancellationToken);
        JsonNode? progress = null;
        var until = DateTime.UtcNow.AddSeconds(10);
        while (DateTime.UtcNow < until && (progress = (await Ask(port, Wire.Command("watch_progress", new JsonObject { ["id"] = 5 })))["value"]) is null)
            await Task.Delay(10, TestContext.Current.CancellationToken);
        Assert.Equal("[30,120]", progress!.ToJsonString());
        // Another watch knows nothing of it, and its Stop stops nothing of it.
        Assert.Null((await Ask(port, Wire.Command("watch_progress", new JsonObject { ["id"] = 4 })))["value"]);
        await Ask(port, Wire.Command("stop_watch", new JsonObject { ["id"] = 4 }));
        await Task.Delay(100, TestContext.Current.CancellationToken);
        Assert.False(crunching.IsCompleted);

        await Ask(port, Wire.Command("stop_watch", new JsonObject { ["id"] = 5 }));
        var done = await crunching.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
        Assert.Null(done["error"]);
        Assert.True(done["value"]!["state"]!["data"]!["stopped"]!.GetValue<bool>());
        Assert.Null((await Ask(port, Wire.Command("watch_progress", new JsonObject { ["id"] = 5 })))["value"]);
        // The browser's end_watch after the server's own changes nothing.
        Assert.Null((await Ask(port, Wire.Command("end_watch", new JsonObject { ["id"] = 5 })))["error"]);

        // A Stop that comes before its handler started stops the call as it starts.
        await Ask(port, Wire.Command("stop_watch", new JsonObject { ["id"] = 6 }));
        var early = await Ask(port, Wire.Dispatch("Crunch", b, "crunch", watch: 6));
        Assert.True(early["value"]!["state"]!["data"]!["stopped"]!.GetValue<bool>());
        // A Stop of a watch that ended stops no later call.
        await Ask(port, Wire.Command("stop_watch", new JsonObject { ["id"] = 5 }));
        CrunchViewModel.Done.Set();
        var later = await Ask(port, Wire.Dispatch("Crunch", a, "crunch", watch: 7));
        Assert.False(later["value"]!["state"]!["data"]!["stopped"]!.GetValue<bool>());
    }
}
