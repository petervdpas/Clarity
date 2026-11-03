// The server's plain socket as browsers meet it (DESIGN 7.6): HEAD answers
// the head alone, a connection stays open for the next request unless the
// browser speaks HTTP/1.0 or says close, commands sent at once are each
// answered, and a head or body past its limit ends the connection unanswered.

using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Ui.Server.Tests;

public class WebServerSocketTests
{
    /// <summary>Sends <paramref name="request"/> on a new connection and reads until the server closes it or a while passed.</summary>
    static async Task<(string Answer, bool Closed)> Ask(int port, byte[] request, int wait = 1500)
    {
        using var client = new TcpClient();
        await client.ConnectAsync("127.0.0.1", port);
        var stream = client.GetStream();
        try
        {
            await stream.WriteAsync(request);
        }
        catch (IOException)
        {
            // The server stopped reading: what it answered still counts.
        }
        var answer = new MemoryStream();
        var buffer = new byte[8192];
        using var timeout = new CancellationTokenSource(wait);
        try
        {
            while (true)
            {
                var read = await stream.ReadAsync(buffer, timeout.Token);
                if (read == 0)
                    return (Encoding.ASCII.GetString(answer.ToArray()), true);
                answer.Write(buffer, 0, read);
            }
        }
        catch (OperationCanceledException)
        {
            return (Encoding.ASCII.GetString(answer.ToArray()), false);
        }
        catch (IOException)
        {
            return (Encoding.ASCII.GetString(answer.ToArray()), true);
        }
    }

    static byte[] Ascii(string text) => Encoding.ASCII.GetBytes(text);

    static readonly int PageLength = Encoding.UTF8.GetByteCount(new WebServer(new ShelfViewModel(null)).Page());

    static async Task With(Func<int, Task> test)
    {
        using var listening = new WebServer(new ShelfViewModel(null)).Start("127.0.0.1:0");
        await test(listening.Port);
    }

    [Fact]
    public Task HEAD_answers_the_head_alone() => With(async port =>
    {
        var (answer, _) = await Ask(port, Ascii("HEAD / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"));
        Assert.StartsWith("HTTP/1.1 200 OK\r\n", answer);
        Assert.Contains($"Content-Length: {PageLength}\r\n", answer);
        Assert.EndsWith("\r\n\r\n", answer);
    });

    [Fact]
    public Task A_connection_stays_open_for_the_next_request_until_the_browser_says_close() => With(async port =>
    {
        // Two requests at once on one connection: both answered, and it stays open.
        var (kept, keptClosed) = await Ask(port, Ascii("GET / HTTP/1.1\r\nHost: x\r\n\r\nGET / HTTP/1.1\r\nHost: x\r\n\r\n"), 800);
        Assert.Equal(2, kept.Split("HTTP/1.1 200 OK").Length - 1);
        Assert.False(keptClosed);
        var (closing, closed) = await Ask(port, Ascii("GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"));
        Assert.Contains("Connection: close\r\n", closing);
        Assert.True(closed);
        var (old, oldClosed) = await Ask(port, Ascii("GET / HTTP/1.0\r\n\r\n"));
        Assert.EndsWith("</html>\n", old);
        Assert.True(oldClosed);
    });

    [Fact]
    public Task A_head_or_a_body_past_its_limit_ends_the_connection_unanswered() => With(async port =>
    {
        var (tooLong, closed) = await Ask(port, Ascii("GET / HTTP/1.1\r\nX-Long: " + new string('a', 70 * 1024) + "\r\n\r\n"));
        Assert.Equal("", tooLong);
        Assert.True(closed);
        var (tooLarge, ended) = await Ask(port, Ascii($"POST {WebServer.InvokePath} HTTP/1.1\r\nContent-Length: {65 * 1024 * 1024}\r\n\r\n"));
        Assert.Equal("", tooLarge);
        Assert.True(ended);
    });

    [Fact]
    public Task Commands_sent_at_once_from_many_connections_are_each_answered() => With(async port =>
    {
        var body = Wire.Command("open_window", new JsonObject { ["window"] = "Shelf" });
        var request = Ascii($"POST {WebServer.InvokePath} HTTP/1.1\r\nContent-Length: {Encoding.UTF8.GetByteCount(body)}\r\nConnection: close\r\n\r\n{body}");
        var answers = await Task.WhenAll(Enumerable.Range(0, 12).Select(_ => Ask(port, request)));
        var windows = answers.Select(a => JsonNode.Parse(a.Answer[(a.Answer.IndexOf("\r\n\r\n", StringComparison.Ordinal) + 4)..])!["value"]!["state"]!["instance"]!.GetValue<long>()).ToList();
        // Each its own window, none lost to another answered at the same time.
        Assert.Equal(12, windows.Distinct().Count());
    });
}
