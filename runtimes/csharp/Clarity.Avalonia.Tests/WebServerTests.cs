// The server that shows Avalonia windows in browsers (DESIGN 7.6): it
// serves the browser program's files, and answers the sessions' requests.

using System.Net;
using System.Text;
using System.Text.Json.Nodes;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class WebServerTests
{
    static string Folder()
    {
        var dir = Path.Combine(Path.GetTempPath(), "clarity-web-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Path.Combine(dir, "_framework"));
        File.WriteAllText(Path.Combine(dir, "index.html"), "<html><body></body></html>");
        File.WriteAllText(Path.Combine(dir, "_framework", "dotnet.js"), "// the runtime");
        File.WriteAllBytes(Path.Combine(dir, "_framework", "app.wasm"), [0, 97, 115, 109]);
        return dir;
    }

    [Fact]
    public void It_serves_the_browser_programs_files_and_answers_the_sessions()
    {
        using var db = Db.Memory();
        var dir = Folder();
        try
        {
            var server = new WebServer(new Sessions(() => new ShelfViewModel(db)), dir);
            var page = server.Answer("GET", "/", "");
            Assert.Equal((200, "text/html; charset=utf-8"), (page.Status, page.Type));
            Assert.Equal("<html><body></body></html>", Encoding.UTF8.GetString(page.Body));
            Assert.Equal("text/javascript", server.Answer("GET", "/_framework/dotnet.js", "").Type);
            var wasm = server.Answer("GET", "/_framework/app.wasm", "");
            Assert.Equal(("application/wasm", 4), (wasm.Type, wasm.Body.Length));
            Assert.Equal(404, server.Answer("GET", "/missing.js", "").Status);
            Assert.Equal(404, server.Answer("GET", "/../secret", "").Status);
            var started = server.Answer("POST", WebServer.SessionPath, """{"do": "start"}""");
            Assert.Equal((200, "application/json"), (started.Status, started.Type));
            Assert.NotNull(JsonNode.Parse(started.Body)!["session"]);
            Assert.Equal(405, server.Answer("GET", WebServer.SessionPath, "").Status);
            Assert.Contains("error", Encoding.UTF8.GetString(server.Answer("POST", WebServer.SessionPath, "not json").Body));
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }

    [Fact]
    public void A_file_goes_compressed_to_a_browser_that_takes_it()
    {
        using var db = Db.Memory();
        var dir = Folder();
        try
        {
            // The publish writes a Brotli and a gzip copy beside each file.
            File.WriteAllBytes(Path.Combine(dir, "_framework", "app.wasm.br"), [1, 2]);
            File.WriteAllBytes(Path.Combine(dir, "_framework", "app.wasm.gz"), [3, 4, 5]);
            var server = new WebServer(new Sessions(() => new ShelfViewModel(db)), dir);
            var br = server.Answer("GET", "/_framework/app.wasm", "", "gzip, deflate, br, zstd");
            Assert.Equal(("application/wasm", "br", 2), (br.Type, br.Encoding, br.Body.Length));
            var gzip = server.Answer("GET", "/_framework/app.wasm", "", "gzip");
            Assert.Equal(("application/wasm", "gzip", 3), (gzip.Type, gzip.Encoding, gzip.Body.Length));
            // A browser that takes neither, or a file without a copy: as it is.
            var plain = server.Answer("GET", "/_framework/app.wasm", "");
            Assert.Equal(((string?)null, 4), (plain.Encoding, plain.Body.Length));
            Assert.Null(server.Answer("GET", "/_framework/dotnet.js", "", "br").Encoding);
            // Asking for the copy itself is not a way around the folder.
            Assert.Equal(404, server.Answer("GET", "/../secret", "", "br").Status);
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }

    [Fact]
    public void It_listens_where_it_is_told()
    {
        Assert.Equal("0.0.0.0:9000", WebServer.Address(["--address", "0.0.0.0:9000"], "127.0.0.1:8080"));
        if (Environment.GetEnvironmentVariable("CLARITY_ADDRESS") is null)
            Assert.Equal("127.0.0.1:8080", WebServer.Address([], "127.0.0.1:8080"));
        Assert.Equal(new IPEndPoint(IPAddress.Loopback, 8080), WebServer.Endpoint("127.0.0.1:8080"));
        Assert.Equal(new IPEndPoint(IPAddress.Any, 9000), WebServer.Endpoint("0.0.0.0:9000"));
        Assert.Equal(new IPEndPoint(IPAddress.Loopback, 7000), WebServer.Endpoint("localhost:7000"));
    }

    /// <summary>
    /// Over a plain socket, as the other stacks serve: Windows's own HTTP
    /// listener (http.sys) refuses 127.0.0.1 and 0.0.0.0 to a person who is
    /// not an administrator.
    /// </summary>
    [Fact]
    public async Task It_answers_browsers_over_a_plain_socket()
    {
        using var db = Db.Memory();
        var dir = Folder();
        try
        {
            File.WriteAllBytes(Path.Combine(dir, "_framework", "app.wasm.br"), [1, 2]);
            File.WriteAllText(Path.Combine(dir, "_framework", "a b.js"), "// spaced");
            var server = new WebServer(new Sessions(() => new ShelfViewModel(db)), dir);
            using var listening = server.Start("127.0.0.1:0");
            using var http = new HttpClient(new SocketsHttpHandler { AutomaticDecompression = DecompressionMethods.None }) { BaseAddress = new Uri($"http://127.0.0.1:{listening.Port}") };
            // Several requests, one after the other on the same connection.
            var page = await http.GetAsync("/");
            Assert.Equal(HttpStatusCode.OK, page.StatusCode);
            Assert.Equal("text/html", page.Content.Headers.ContentType!.MediaType);
            Assert.Equal("<html><body></body></html>", await page.Content.ReadAsStringAsync());
            var asked = new HttpRequestMessage(HttpMethod.Get, "/_framework/app.wasm");
            asked.Headers.AcceptEncoding.ParseAdd("gzip, br");
            var wasm = await http.SendAsync(asked);
            Assert.Equal(["br"], wasm.Content.Headers.ContentEncoding);
            Assert.Equal([1, 2], await wasm.Content.ReadAsByteArrayAsync());
            Assert.Equal("// spaced", await http.GetStringAsync("/_framework/a%20b.js"));
            Assert.Equal(HttpStatusCode.NotFound, (await http.GetAsync("/missing.js")).StatusCode);
            Assert.Equal(HttpStatusCode.NotFound, (await http.GetAsync("/_framework/..%5C..%5Csecret")).StatusCode);
            var started = await http.PostAsync(WebServer.SessionPath, new StringContent("""{"do": "start"}""", Encoding.UTF8, "application/json"));
            Assert.NotNull(JsonNode.Parse(await started.Content.ReadAsStringAsync())!["session"]);
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }
}
