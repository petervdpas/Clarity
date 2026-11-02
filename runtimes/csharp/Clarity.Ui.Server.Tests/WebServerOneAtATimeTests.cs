// The server answers one request at a time (DESIGN 7.6): no request is
// answered beside another, however many connections a browser asks from at
// once (the sessions keep a lock of their own besides). And Serve
// says where it serves, then serves until it is stopped.

using System.Net.Sockets;
using System.Text;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Ui.Server.Tests;

public class WebServerOneAtATimeTests
{
    static string Folder()
    {
        var dir = Path.Combine(Path.GetTempPath(), "clarity-web-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        File.WriteAllText(Path.Combine(dir, "index.html"), "<html></html>");
        return dir;
    }

    /// <summary>Sends one request on a connection of its own and reads the answer to its end.</summary>
    static async Task<string> Ask(int port, byte[] request)
    {
        using var client = new TcpClient();
        await client.ConnectAsync("127.0.0.1", port, TestContext.Current.CancellationToken);
        var stream = client.GetStream();
        await stream.WriteAsync(request, TestContext.Current.CancellationToken);
        using var reader = new StreamReader(stream, Encoding.ASCII);
        return await reader.ReadToEndAsync(TestContext.Current.CancellationToken).WaitAsync(TimeSpan.FromSeconds(20), TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task No_two_requests_are_answered_at_the_same_time()
    {
        var dir = Folder();
        try
        {
            var inside = 0;
            var most = 0;
            var server = new WebServer(new Sessions(() => new ShelfViewModel(null)), dir);
            // Each answer takes a while, as a slow handler or a large file would.
            server.Answering = (method, path, sent, accepts) =>
            {
                var now = Interlocked.Increment(ref inside);
                InterlockedMax(ref most, now);
                Thread.Sleep(30);
                Interlocked.Decrement(ref inside);
                return server.Answer(method, path, sent, accepts);
            };
            using var listening = server.Start("127.0.0.1:0");
            var body = """{"do": "start"}""";
            var request = Encoding.ASCII.GetBytes($"POST {WebServer.SessionPath} HTTP/1.1\r\nContent-Length: {body.Length}\r\nConnection: close\r\n\r\n{body}");
            var answers = await Task.WhenAll(Enumerable.Range(0, 8).Select(_ => Task.Run(() => Ask(listening.Port, request), TestContext.Current.CancellationToken)));
            Assert.All(answers, a => Assert.Contains("\"session\"", a));
            // Without the server's semaphore, two of the eight would be inside at once.
            Assert.Equal(1, most);
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }

    static void InterlockedMax(ref int most, int now)
    {
        int seen;
        while ((seen = Volatile.Read(ref most)) < now && Interlocked.CompareExchange(ref most, now, seen) != seen)
        {
        }
    }

    [Fact]
    public async Task Serve_says_where_it_serves_and_serves_until_it_is_stopped()
    {
        var dir = Folder();
        try
        {
            var server = new WebServer(new Sessions(() => new ShelfViewModel(null)), dir);
            var said = new StringWriter();
            using var stop = new CancellationTokenSource();
            var serving = Task.Run(() => server.Serve("Shelf", "127.0.0.1:0", said, stop.Token), TestContext.Current.CancellationToken);
            await Task.Delay(300, TestContext.Current.CancellationToken);
            Assert.False(serving.IsCompleted);
            Assert.Equal("Shelf: serving on http://127.0.0.1:0", said.ToString().Trim());
            stop.Cancel();
            await serving.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }
}
