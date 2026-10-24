// HTTP calls (DESIGN 5.23) against the cases every runtime shares
// (conformance/http.json), on a small server of the test's own.

using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class HttpTests : IDisposable
{
    readonly HttpListener listener = new();
    readonly string server;

    public HttpTests()
    {
        // A free port, as the system hands one out.
        var probe = new TcpListener(IPAddress.Loopback, 0);
        probe.Start();
        var port = ((IPEndPoint)probe.LocalEndpoint).Port;
        probe.Stop();
        server = $"http://127.0.0.1:{port}";
        listener.Prefixes.Add(server + "/");
        listener.Start();
        _ = Task.Run(Serve);
    }

    async Task Serve()
    {
        while (listener.IsListening)
        {
            HttpListenerContext context;
            try
            {
                context = await listener.GetContextAsync();
            }
            catch (Exception)
            {
                return;
            }
            var request = context.Request;
            using var reader = new StreamReader(request.InputStream, Encoding.UTF8);
            var body = reader.ReadToEnd();
            var (status, text) = request.Url!.AbsolutePath switch
            {
                "/hello" => (200, "hello"),
                "/echo" => (200, $"{request.HttpMethod} {body}"),
                "/header" => (200, request.Headers["X-Test"] ?? ""),
                "/query" => (200, request.Url.Query.TrimStart('?')),
                "/auth" => (200, request.Headers["Authorization"] ?? request.Headers["X-Api-Key"] ?? ""),
                _ => (404, "no such page"),
            };
            var bytes = Encoding.UTF8.GetBytes(text);
            context.Response.StatusCode = status;
            context.Response.ContentType = "text/plain; charset=utf-8";
            context.Response.ContentLength64 = bytes.Length;
            context.Response.OutputStream.Write(bytes);
            context.Response.Close();
        }
    }

    public void Dispose() => listener.Close();

    [Fact]
    public void EveryCallGivesTheReplyTheCasesName()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "http.json"))).RootElement;
        foreach (var env in file.GetProperty("env").EnumerateObject())
            Environment.SetEnvironmentVariable(env.Name, env.Value.GetString());
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var url = c.TryGetProperty("url", out var u) ? u.GetString()! : server + c.GetProperty("path").GetString();
            var headers = c.TryGetProperty("headers", out var h) ? h.EnumerateObject().Select(p => (p.Name, p.Value.GetString()!)).ToArray() : [];
            var body = c.TryGetProperty("body", out var b) ? b.GetString()! : null;
            var reply = c.TryGetProperty("base", out var at)
                ? Http.At(c.GetProperty("method").GetString()!, at.GetString()!.Replace("{server}", server), c.GetProperty("path").GetString()!,
                    c.TryGetProperty("query", out var q) ? q.EnumerateArray().Select(p => (p[0].GetString()!, p[1].GetString()!)).ToArray() : [],
                    body, headers,
                    c.TryGetProperty("auth", out var auth) ? auth.GetProperty("kind").GetString() : null,
                    c.TryGetProperty("auth", out var auth2) ? auth2.GetProperty("secret").GetString() : null)
                : c.GetProperty("method").GetString() switch
            {
                "POST" => Http.Post(url, body!, headers),
                "PUT" => Http.Put(url, body!, headers),
                "DELETE" => Http.Delete(url, headers),
                _ => Http.Get(url, headers),
            };
            Assert.True(reply.Status == c.GetProperty("status").GetInt64(), $"{name}: {reply}");
            Assert.True(reply.Ok == c.GetProperty("ok").GetBoolean(), $"{name}: {reply}");
            if (c.TryGetProperty("text", out var text))
                Assert.Equal(text.GetString(), reply.Text);
            if (c.TryGetProperty("text_starts", out var starts))
                Assert.StartsWith(starts.GetString()!, reply.Text);
        }
    }

    [Theory]
    [InlineData("https://api.example.com/orders?x=1", "api.example.com")]
    [InlineData("http://127.0.0.1:1/", "127.0.0.1")]
    [InlineData("https://me:secret@example.com:8443", "example.com")]
    [InlineData("http://[::1]:8080/x", "::1")]
    public void AUrlNamesItsHost(string url, string host) => Assert.Equal(host, Http.Host(url));
}
