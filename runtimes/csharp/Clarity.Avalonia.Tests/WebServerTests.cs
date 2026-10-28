// The server that shows Avalonia windows in browsers (DESIGN 7.6): it
// serves the browser program's files, and answers the sessions' requests.

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
    public void It_listens_where_it_is_told()
    {
        Assert.Equal("0.0.0.0:9000", WebServer.Address(["--address", "0.0.0.0:9000"], "127.0.0.1:8080"));
        if (Environment.GetEnvironmentVariable("CLARITY_ADDRESS") is null)
            Assert.Equal("127.0.0.1:8080", WebServer.Address([], "127.0.0.1:8080"));
        Assert.Equal("http://127.0.0.1:8080/", WebServer.Prefix("127.0.0.1:8080"));
        Assert.Equal("http://+:9000/", WebServer.Prefix("0.0.0.0:9000"));
    }
}
