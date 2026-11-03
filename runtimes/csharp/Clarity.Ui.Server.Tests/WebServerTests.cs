// The server that shows a Clarity.UI program's windows in browsers (DESIGN
// 7.6, 7.9): it serves the page, the browser files and the program's files
// the application carries, and answers the library's commands in the browser
// through ProgramWire, as the desktop does.

using System.IO.Compression;
using System.Net;
using System.Text;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Ui.Server.Tests;

public class WebServerTests
{
    static WebServer Server() => new(new ShelfViewModel(null));

    static JsonObject Json(WebServer.Reply reply) => JsonNode.Parse(reply.Body)!.AsObject();

    [Fact]
    public void It_serves_the_page_and_the_browser_files_the_application_carries()
    {
        var server = Server();
        var page = server.Answer("GET", "/", "");
        Assert.Equal((200, "text/html; charset=utf-8"), (page.Status, page.Type));
        var html = Encoding.UTF8.GetString(page.Body);
        Assert.Contains("<canvas id=\"clarity-ui\"></canvas>", html);
        Assert.Contains("<script src=\"clarity-ui-page.js\"></script>", html);
        // The main window's title, written safely.
        Assert.Contains("<title>Shelf</title>", html);
        Assert.Equal(html, Encoding.UTF8.GetString(server.Answer("GET", "/index.html", "").Body));
        var script = server.Answer("GET", "/clarity-ui-page.js", "");
        Assert.Equal(("text/javascript", "// the page script\n"), (script.Type, Encoding.UTF8.GetString(script.Body)));
        var wasm = server.Answer("GET", "/clarity-ui.wasm", "");
        Assert.Equal(("application/wasm", 512), (wasm.Type, wasm.Body.Length));
        Assert.Equal("// spaced\n", Encoding.UTF8.GetString(server.Answer("GET", "/a%20b.js", "").Body));
        Assert.Equal(404, server.Answer("GET", "/missing.js", "").Status);
        Assert.Equal(404, server.Answer("GET", "/../secret", "").Status);
        // Only what the application carries under browser/, nothing else of it.
        Assert.Equal(404, server.Answer("GET", "/model.json", "").Status);
        Assert.Equal(404, server.Answer("GET", "/Windows/Shelf.cxaml", "").Status);
        Assert.Equal(405, server.Answer("DELETE", "/", "").Status);
    }

    [Fact]
    public void It_serves_the_programs_files_as_the_library_reads_them()
    {
        var reply = Server().Answer("GET", WebServer.FilesPath, "");
        Assert.Equal((200, "application/json"), (reply.Status, reply.Type));
        var files = Json(reply);
        Assert.Equal("Shelf", files["title"]!.GetValue<string>());
        Assert.Equal((800d, 600d), (files["width"]!.GetValue<double>(), files["height"]!.GetValue<double>()));
        Assert.Equal(["Crunch", "Shelf"], files["windows"]!.AsObject().Select(w => w.Key));
        Assert.Contains("<Window name=\"Shelf\">", files["windows"]!["Shelf"]!.GetValue<string>());
        Assert.Empty(files["apps"]!.AsObject());
        Assert.Contains("#3366ff", files["themes"]!["plain.light"]!.GetValue<string>());
        Assert.Contains("Shelves", files["model"]!.GetValue<string>());
        Assert.Equal("[\"logo.png\"]", files["resources"]!.ToJsonString());
    }

    [Fact]
    public void It_serves_each_resource_by_its_name_and_nothing_beside_them()
    {
        var server = Server();
        var logo = server.Answer("GET", WebServer.ResourcesPath + "logo.png", "");
        Assert.Equal((200, "image/png"), (logo.Status, logo.Type));
        Assert.Equal([137, 80, 78, 71], logo.Body);
        Assert.Equal(404, server.Answer("GET", WebServer.ResourcesPath + "missing.png", "").Status);
        Assert.Equal(404, server.Answer("GET", WebServer.ResourcesPath + "../model.json", "").Status);
        Assert.Equal(404, server.Answer("GET", WebServer.ResourcesPath + "..%2Fmodel.json", "").Status);
    }

    [Fact]
    public void A_file_goes_compressed_to_a_browser_that_takes_it()
    {
        var server = Server();
        var br = server.Answer("GET", "/clarity-ui.wasm", "", "gzip, deflate, br, zstd");
        Assert.Equal(("application/wasm", "br"), (br.Type, br.Encoding));
        Assert.True(br.Body.Length < 512);
        using (var read = new BrotliStream(new MemoryStream(br.Body), CompressionMode.Decompress))
        {
            var plain = new MemoryStream();
            read.CopyTo(plain);
            Assert.Equal(512, plain.Length);
        }
        var gzip = server.Answer("GET", "/clarity-ui.wasm", "", "gzip");
        Assert.Equal("gzip", gzip.Encoding);
        // Compressed once: the same bytes again.
        Assert.Same(gzip.Body, server.Answer("GET", "/clarity-ui.wasm", "", "gzip").Body);
        Assert.Equal("br", server.Answer("GET", WebServer.FilesPath, "", "br").Encoding);
        // A browser that takes neither, or refuses one: as it is.
        Assert.Null(server.Answer("GET", "/clarity-ui.wasm", "").Encoding);
        Assert.Equal("gzip", server.Answer("GET", "/clarity-ui.wasm", "", "br;q=0, gzip").Encoding);
    }

    [Fact]
    public void The_program_answers_the_librarys_commands_at_its_path()
    {
        var server = Server();
        Assert.Equal(405, server.Answer("GET", WebServer.InvokePath, "").Status);
        Assert.Contains("error", Encoding.UTF8.GetString(server.Answer("POST", WebServer.InvokePath, "not json").Body));
        Assert.Contains("error", Encoding.UTF8.GetString(server.Answer("POST", WebServer.InvokePath, "{}").Body));
        var started = Json(server.Answer("POST", WebServer.InvokePath, Wire.Command("started")));
        Assert.Equal("Shelf", started["value"]!["main"]!.GetValue<string>());
        // The application's engines say how far a long call is (its Engines.Watches): the browser shows the bar with Stop.
        Assert.True(Json(server.Answer("POST", WebServer.InvokePath, Wire.Command("watches")))["value"]!.GetValue<bool>());
        var opened = Json(server.Answer("POST", WebServer.InvokePath, Wire.Command("open_window", new JsonObject { ["window"] = "Shelf", ["with"] = new JsonObject() })));
        var instance = opened["value"]!["state"]!["instance"]!.GetValue<long>();
        var greeted = Json(server.Answer("POST", WebServer.InvokePath, Wire.Dispatch("Shelf", instance, "greet")));
        Assert.Contains("Hello", greeted["value"]!["messages"]!.ToJsonString());
        // What the program leaves to the library: no value and no error.
        Assert.Equal("{}", Encoding.UTF8.GetString(server.Answer("POST", WebServer.InvokePath, Wire.Command("nothing_like_it")).Body));
        // Args may also come as the JSON text of an object.
        var named = new JsonObject { ["command"] = "open_window", ["args"] = """{"window": "Nothing"}""" }.ToJsonString();
        Assert.Contains("Nothing", Json(server.Answer("POST", WebServer.InvokePath, named))["error"]!.ToJsonString());
    }

    [Fact]
    public void Every_browser_gets_a_main_window_of_its_own()
    {
        var server = Server();
        string Open() => Encoding.UTF8.GetString(server.Answer("POST", WebServer.InvokePath, Wire.Command("open_window", new JsonObject { ["window"] = "Shelf" })).Body);
        long Instance(string reply) => JsonNode.Parse(reply)!["value"]!["state"]!["instance"]!.GetValue<long>();
        var first = Instance(Open());
        var second = Instance(Open());
        Assert.NotEqual(first, second);
        // One browser's step is its own.
        var stepped = Json(server.Answer("POST", WebServer.InvokePath, Wire.Dispatch("Shelf", first, "stepped")));
        Assert.Equal(1, stepped["value"]!["state"]!["data"]!["steps"]!.GetValue<long>());
        var other = Json(server.Answer("POST", WebServer.InvokePath, Wire.Dispatch("Shelf", second, "stepped")));
        Assert.Equal(1, other["value"]!["state"]!["data"]!["steps"]!.GetValue<long>());
        // Closed, the first main window is not shown again: a new one is made.
        server.Answer("POST", WebServer.InvokePath, Wire.Command("close_window", new JsonObject { ["state"] = new JsonObject { ["window"] = "Shelf", ["instance"] = first } }));
        Assert.NotEqual(first, Instance(Open()));
    }

    [Fact]
    public void A_window_a_browser_left_alone_is_let_go()
    {
        var server = Server();
        var now = new DateTime(2026, 11, 3, 12, 0, 0, DateTimeKind.Utc);
        server.Wire.Clock = () => now;
        var opened = Json(server.Answer("POST", WebServer.InvokePath, Wire.Command("open_window", new JsonObject { ["window"] = "Shelf" })));
        var instance = opened["value"]!["state"]!["instance"]!.GetValue<long>();
        now = now.AddMinutes(29);
        Assert.Null(Json(server.Answer("POST", WebServer.InvokePath, Wire.Dispatch("Shelf", instance, "stepped")))["error"]);
        // Each use keeps it: 29 minutes after the last one it is there still, past 30 it is gone.
        now = now.AddMinutes(29);
        Assert.Null(Json(server.Answer("POST", WebServer.InvokePath, Wire.Dispatch("Shelf", instance, "stepped")))["error"]);
        now = now.AddMinutes(31);
        Assert.NotNull(Json(server.Answer("POST", WebServer.InvokePath, Wire.Dispatch("Shelf", instance, "stepped")))["error"]);
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
        using var listening = Server().Start("127.0.0.1:0");
        using var http = new HttpClient(new SocketsHttpHandler { AutomaticDecompression = DecompressionMethods.None }) { BaseAddress = new Uri($"http://127.0.0.1:{listening.Port}") };
        // Several requests, one after the other on the same connection.
        var cancel = TestContext.Current.CancellationToken;
        var page = await http.GetAsync("/", cancel);
        Assert.Equal(HttpStatusCode.OK, page.StatusCode);
        Assert.Equal("text/html", page.Content.Headers.ContentType!.MediaType);
        var asked = new HttpRequestMessage(HttpMethod.Get, "/clarity-ui.wasm");
        asked.Headers.AcceptEncoding.ParseAdd("gzip, br");
        var wasm = await http.SendAsync(asked, cancel);
        Assert.Equal(["br"], wasm.Content.Headers.ContentEncoding);
        Assert.Equal("// spaced\n", await http.GetStringAsync("/a%20b.js", cancel));
        Assert.Equal(HttpStatusCode.NotFound, (await http.GetAsync("/missing.js", cancel)).StatusCode);
        Assert.Equal(HttpStatusCode.NotFound, (await http.GetAsync("/..%5C..%5Csecret", cancel)).StatusCode);
        var started = await http.PostAsync(WebServer.InvokePath, new StringContent(Wire.Command("started"), Encoding.UTF8, "application/json"), cancel);
        Assert.Equal("Shelf", JsonNode.Parse(await started.Content.ReadAsStringAsync(cancel))!["value"]!["main"]!.GetValue<string>());
    }

    [Fact]
    public async Task Serve_says_where_it_serves_and_serves_until_it_is_stopped()
    {
        var said = new StringWriter();
        using var stop = new CancellationTokenSource();
        var serving = Task.Run(() => Server().Serve("Shelf", "127.0.0.1:0", said, stop.Token), TestContext.Current.CancellationToken);
        await Task.Delay(300, TestContext.Current.CancellationToken);
        Assert.False(serving.IsCompleted);
        Assert.Equal("Shelf: serving on http://127.0.0.1:0", said.ToString().Trim());
        stop.Cancel();
        await serving.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
    }
}
