// The server that shows a Clarity.UI program's windows in browsers (DESIGN
// 7.6, 7.9): the browser runs Clarity.UI itself (the browser files, which
// the application carries under browser/), and asks the program here, at
// InvokePath, as the native library asks it on the desktop: ProgramWire
// answers both. It serves the page, the browser files, the program's files
// (files.json: its windows' markup, themes and model) and its resources. It
// speaks HTTP over a plain socket, as the other stacks' servers do: Windows's
// own HTTP listener (http.sys) lets only an administrator listen on
// 127.0.0.1 or 0.0.0.0.

using System.Collections.Concurrent;
using System.Globalization;
using System.IO.Compression;
using System.Net;
using System.Net.Sockets;
using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Server;

/// <summary>The server of a program's windows, shown in browsers by Clarity.UI.</summary>
public sealed class WebServer
{
    readonly Assembly app;
    readonly ProgramWire wire;
    readonly string? title;
    readonly Extent? size;

    /// <summary>
    /// The server of the application whose main window is <paramref name="main"/>,
    /// made on the server's databases: the first browser shows it, every other
    /// browser a new one of its kind. Windows a browser left alone for half
    /// an hour (a tab that closed) are let go.
    /// </summary>
    public WebServer(WindowViewModel main) : this(main.GetType().Assembly, main)
    {
    }

    /// <summary>The server of <paramref name="app"/>, starting with <paramref name="main"/>.</summary>
    public WebServer(Assembly app, WindowViewModel main)
    {
        this.app = app;
        title = main.Title;
        size = main.WindowSize;
        // Each browser's long call goes by the watch its handler's command names.
        wire = new ProgramWire(app, main, ProgramWire.MarkupOf(app)) { Served = true };
        // The built-in `keys` from code: the next reply says so, and the library in the browser opens the keys window.
        DesktopProgram.Current = wire;
    }

    /// <summary>The program, for tests.</summary>
    internal ProgramWire Wire => wire;

    /// <summary>Where the library in the browser asks the program: a POST of <c>{"command", "args"}</c>.</summary>
    public const string InvokePath = "/clarity/invoke";

    /// <summary>Where the library in the browser reads the program's files.</summary>
    public const string FilesPath = "/clarity/files.json";

    /// <summary>Where the library in the browser reads each file of the resources folder by its name.</summary>
    public const string ResourcesPath = "/clarity/resources/";

    /// <summary>The folder of the application's embedded resources that holds the browser files.</summary>
    const string BrowserFolder = "browser/";

    /// <summary>The program's files, once: what the application carries does not change while it runs.</summary>
    AppFiles? carried;
    byte[]? filesJson;

    AppFiles Carried => carried ??= AppFiles.Of(app, title);

    /// <summary>
    /// The program's files as the library in the browser reads them:
    /// <c>{"title", "width", "height", "windows", "apps", "themes", "model", "resources"}</c>.
    /// </summary>
    public JsonObject Files()
    {
        var files = Carried;
        static JsonObject Named(SortedDictionary<string, string> all) => new(all.Select(p => KeyValuePair.Create(p.Key, (JsonNode?)p.Value)));
        return new JsonObject
        {
            ["title"] = files.Title,
            ["width"] = size?.Width,
            ["height"] = size?.Height,
            ["windows"] = Named(files.Windows),
            ["apps"] = Named(files.Apps),
            ["themes"] = Named(files.Themes),
            ["model"] = files.Model,
            ["resources"] = new JsonArray([.. files.Resources.Select(r => (JsonNode?)r)]),
        };
    }

    /// <summary>The page: Clarity.UI's canvas and its page script, under the program's title.</summary>
    public string Page() =>
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n"
        + $"<title>{WebUtility.HtmlEncode(Carried.Title)}</title>\n"
        + "<style>html, body { margin: 0; height: 100%; overflow: hidden; } #clarity-ui { display: block; width: 100vw; height: 100vh; touch-action: none; }</style>\n"
        + "</head>\n<body>\n<canvas id=\"clarity-ui\"></canvas>\n<script src=\"clarity-ui-page.js\"></script>\n</body>\n</html>\n";

    /// <summary>A reply: its status, the kind of its body, the body, and how the body is compressed (none when it is not).</summary>
    public readonly record struct Reply(int Status, string Type, byte[] Body, string? Encoding = null);

    /// <summary>The encodings the server compresses with, the smaller first.</summary>
    static readonly string[] Encodings = ["br", "gzip"];

    /// <summary>Each file compressed once, by its name and encoding.</summary>
    readonly ConcurrentDictionary<(string File, string Encoding), byte[]> compressed = new();

    /// <summary>Where the server listens: <c>--address host:port</c>, else CLARITY_ADDRESS, else <paramref name="project"/>, where the project file says.</summary>
    public static string Address(string[] args, string project)
    {
        var at = Array.IndexOf(args, "--address");
        if (at >= 0 && at + 1 < args.Length)
            return args[at + 1];
        return Environment.GetEnvironmentVariable("CLARITY_ADDRESS") is { Length: > 0 } address ? address : project;
    }

    /// <summary>Where <paramref name="address"/> (host:port) listens: every address of the machine for 0.0.0.0.</summary>
    public static IPEndPoint Endpoint(string address)
    {
        var colon = address.LastIndexOf(':');
        var host = address[..colon].Trim('[', ']');
        var port = int.Parse(address[(colon + 1)..], CultureInfo.InvariantCulture);
        var ip = host switch
        {
            "" or "0.0.0.0" or "*" or "+" => IPAddress.Any,
            "localhost" => IPAddress.Loopback,
            _ => IPAddress.TryParse(host, out var parsed) ? parsed : Dns.GetHostAddresses(host).First(),
        };
        return new IPEndPoint(ip, port);
    }

    static Reply Text(int status, string type, string body) => new(status, type, Encoding.UTF8.GetBytes(body));

    /// <summary>The reply to one request; <paramref name="accepts"/> is its Accept-Encoding.</summary>
    public Reply Answer(string method, string path, string body, string? accepts = null)
    {
        var file = Uri.UnescapeDataString(path.Split('?')[0]);
        if (file == InvokePath)
        {
            if (method != "POST")
                return Text(405, "text/plain", "ask the program with POST");
            return Text(200, "application/json", Invoke(body).ToJsonString());
        }
        if (method is not ("GET" or "HEAD"))
            return Text(405, "text/plain", "");
        if (file == FilesPath)
        {
            filesJson ??= Encoding.UTF8.GetBytes(Files().ToJsonString());
            return Compressed("files.json", "application/json", filesJson, accepts);
        }
        // The page the application carries (every language's), else the server's own.
        if (file is "/" or "/index.html")
            return new Reply(200, Kind(".html"), Carried.Read(BrowserFolder + "index.html") ?? Encoding.UTF8.GetBytes(Page()));
        // Only the application's own files: nothing above their folder, on any system.
        var name = file.TrimStart('/');
        if (name.Contains('\\') || name.Contains(':') || name.Split('/').Any(part => part is ".." or "." or ""))
            return Text(404, "text/plain", "");
        if (file.StartsWith(ResourcesPath, StringComparison.Ordinal))
        {
            var resource = file[ResourcesPath.Length..];
            return Carried.Resources.Contains(resource) && Carried.Resource(resource) is { } bytes
                ? new Reply(200, Kind(Path.GetExtension(resource)), bytes)
                : Text(404, "text/plain", "");
        }
        if (!name.Contains('/') && Carried.Read(BrowserFolder + name) is { } browser)
            return Compressed(name, Kind(Path.GetExtension(name)), browser, accepts);
        return Text(404, "text/plain", "");
    }

    /// <summary>
    /// The program's answer to the library's command in <paramref name="body"/>
    /// (<c>{"command", "args", "browser", "settings"}</c>): <c>{"value"}</c> or
    /// <c>{"error"}</c>, and <c>{}</c> where the program leaves the library's
    /// default. <c>browser</c> names the page that asks (one per tab), and
    /// <c>settings</c> are its person's settings as the page keeps them: the
    /// handler sees those, in that page's language, and what it asks of the
    /// screen goes to that page alone. The answer brings the page's settings
    /// back (<c>"settings"</c>) when the command changed them, for the page to keep.
    /// </summary>
    JsonObject Invoke(string body)
    {
        JsonObject asked;
        try
        {
            asked = JsonNode.Parse(body) as JsonObject ?? throw new JsonException("a command is a JSON object");
        }
        catch (JsonException e)
        {
            return new JsonObject { ["error"] = e.Message };
        }
        var command = asked["command"] is JsonValue c && c.TryGetValue<string>(out var named) ? named : "";
        if (command == "")
            return new JsonObject { ["error"] = "a command has its name" };
        var args = asked["args"] switch
        {
            null => "{}",
            JsonValue text when text.TryGetValue<string>(out var json) => json,
            var given => given.ToJsonString(),
        };
        var browser = asked["browser"] is JsonValue b && b.TryGetValue<string>(out var page) ? page : "";
        var sent = asked["settings"] as JsonObject;
        var (answer, settings) = wire.AnswerFor(browser, sent, command, args);
        var reply = answer ?? [];
        if (settings is not null)
            reply["settings"] = settings;
        return reply;
    }

    /// <summary><paramref name="bytes"/>, compressed once, to a browser that takes it.</summary>
    Reply Compressed(string file, string type, byte[] bytes, string? accepts)
    {
        var taken = Taken(accepts);
        foreach (var encoding in Encodings)
            if (taken.Contains(encoding))
                return new Reply(200, type, compressed.GetOrAdd((file, encoding), _ => Compress(bytes, encoding)), encoding);
        return new Reply(200, type, bytes);
    }

    static byte[] Compress(byte[] bytes, string encoding)
    {
        using var into = new MemoryStream();
        using (Stream by = encoding == "br" ? new BrotliStream(into, CompressionLevel.Optimal) : new GZipStream(into, CompressionLevel.Optimal))
            by.Write(bytes);
        return into.ToArray();
    }

    /// <summary>The encodings an Accept-Encoding names, without those it refuses (<c>;q=0</c>).</summary>
    static HashSet<string> Taken(string? accepts) =>
    [
        .. (accepts ?? "").Split(',')
            .Select(part => part.Split(';').Select(p => p.Trim()).ToArray())
            .Where(part => !part.Skip(1).Any(p => p.Replace(" ", "") is "q=0" or "q=0.0" or "q=0.00" or "q=0.000"))
            .Select(part => part[0].ToLowerInvariant()),
    ];

    /// <summary>The kind of a file, by its extension.</summary>
    static string Kind(string extension) => extension.ToLowerInvariant() switch
    {
        ".html" => "text/html; charset=utf-8",
        ".js" or ".mjs" => "text/javascript",
        ".css" => "text/css",
        ".json" => "application/json",
        ".wasm" => "application/wasm",
        ".svg" => "image/svg+xml",
        ".png" => "image/png",
        ".ico" => "image/x-icon",
        ".jpg" or ".jpeg" => "image/jpeg",
        ".gif" => "image/gif",
        ".webp" => "image/webp",
        ".pdf" => "application/pdf",
        ".txt" or ".md" => "text/plain; charset=utf-8",
        ".csv" => "text/csv; charset=utf-8",
        ".woff2" => "font/woff2",
        ".ttf" => "font/ttf",
        _ => "application/octet-stream",
    };

    /// <summary>Serves at <paramref name="address"/> until the program stops.</summary>
    public void Serve(string name, string address) => Serve(name, address, Console.Error, CancellationToken.None);

    /// <summary>Serves at <paramref name="address"/>, saying where on <paramref name="said"/>, until <paramref name="stop"/> is cancelled.</summary>
    public void Serve(string name, string address, TextWriter said, CancellationToken stop)
    {
        using var listening = Start(address);
        said.WriteLine($"{name}: serving on http://{address}");
        try
        {
            listening.Done.Wait(stop);
        }
        catch (OperationCanceledException) when (stop.IsCancellationRequested)
        {
            // Asked to stop: the listening ends as it is disposed.
        }
    }

    /// <summary>A server listening, until it is disposed.</summary>
    public sealed class Listening(TcpListener listener, CancellationTokenSource stop, Func<CancellationToken, Task> accept) : IDisposable
    {
        /// <summary>The port it listens on: the one asked for, or the one the system gave for 0.</summary>
        public int Port { get; } = ((IPEndPoint)listener.LocalEndpoint).Port;

        /// <summary>Done when it stopped.</summary>
        public Task Done { get; } = Task.Run(() => accept(stop.Token));

        public void Dispose()
        {
            stop.Cancel();
            listener.Stop();
        }
    }

    /// <summary>Starts listening at <paramref name="address"/>; requests are answered until the result is disposed.</summary>
    public Listening Start(string address)
    {
        var listener = new TcpListener(Endpoint(address));
        listener.Start();
        return new Listening(listener, new CancellationTokenSource(), stop => Accept(listener, stop));
    }

    async Task Accept(TcpListener listener, CancellationToken stop)
    {
        while (!stop.IsCancellationRequested)
        {
            TcpClient client;
            try
            {
                client = await listener.AcceptTcpClientAsync(stop);
            }
            catch (Exception) when (stop.IsCancellationRequested)
            {
                return;
            }
            catch (SocketException)
            {
                continue;
            }
            _ = Task.Run(() => Connection(client, stop));
        }
    }

    /// <summary>The longest head of a request, and the largest body, it reads.</summary>
    const int HeadLimit = 64 * 1024, BodyLimit = 64 * 1024 * 1024;

    /// <summary>The requests of one connection, one after the other, until the browser closes it.</summary>
    async Task Connection(TcpClient client, CancellationToken stop)
    {
        using var _ = client;
        try
        {
            var stream = client.GetStream();
            var buffer = new byte[HeadLimit];
            var filled = 0;
            while (true)
            {
                int end;
                while ((end = HeadEnd(buffer, filled)) < 0)
                {
                    if (filled == buffer.Length)
                        return;
                    var read = await stream.ReadAsync(buffer.AsMemory(filled), stop);
                    if (read == 0)
                        return;
                    filled += read;
                }
                var lines = Encoding.ASCII.GetString(buffer, 0, end).Split("\r\n");
                var start = lines[0].Split(' ');
                if (start.Length != 3)
                    return;
                var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                foreach (var line in lines.Skip(1))
                    if (line.IndexOf(':') is > 0 and var colon)
                        headers[line[..colon].Trim()] = line[(colon + 1)..].Trim();
                var length = headers.TryGetValue("Content-Length", out var said) && int.TryParse(said, CultureInfo.InvariantCulture, out var n) ? n : 0;
                if (length is < 0 or > BodyLimit)
                    return;
                // The body: what came with the head, then the rest.
                var body = new byte[length];
                var bodyStart = end + 4;
                var had = Math.Min(filled - bodyStart, length);
                Array.Copy(buffer, bodyStart, body, 0, had);
                while (had < length)
                {
                    var read = await stream.ReadAsync(body.AsMemory(had), stop);
                    if (read == 0)
                        return;
                    had += read;
                }
                // What came after it belongs to the next request.
                var used = bodyStart + Math.Min(filled - bodyStart, length);
                Array.Copy(buffer, used, buffer, 0, filled - used);
                filled -= used;
                // The program answers one command at a time (ProgramWire); files come meanwhile.
                var reply = Answer(start[0], start[1], Encoding.UTF8.GetString(body), headers.GetValueOrDefault("Accept-Encoding"));
                var close = start[2] == "HTTP/1.0" || headers.GetValueOrDefault("Connection")?.Equals("close", StringComparison.OrdinalIgnoreCase) == true;
                var head = new StringBuilder()
                    .Append(CultureInfo.InvariantCulture, $"HTTP/1.1 {reply.Status} {Reason(reply.Status)}\r\n")
                    .Append(CultureInfo.InvariantCulture, $"Content-Type: {reply.Type}\r\n")
                    .Append(CultureInfo.InvariantCulture, $"Content-Length: {reply.Body.Length}\r\n")
                    // Which copy a browser gets depends on what it takes.
                    .Append("Vary: Accept-Encoding\r\n");
                if (reply.Encoding is { } encoding)
                    head.Append(CultureInfo.InvariantCulture, $"Content-Encoding: {encoding}\r\n");
                if (close)
                    head.Append("Connection: close\r\n");
                head.Append("\r\n");
                await stream.WriteAsync(Encoding.ASCII.GetBytes(head.ToString()), stop);
                if (start[0] != "HEAD")
                    await stream.WriteAsync(reply.Body, stop);
                if (close)
                    return;
            }
        }
        catch (Exception e) when (e is IOException or SocketException or OperationCanceledException or ObjectDisposedException)
        {
            // The browser went, or the server stops.
        }
    }

    /// <summary>Where the head of a request ends (its blank line), or -1 while it has not come yet.</summary>
    static int HeadEnd(byte[] buffer, int filled)
    {
        for (var i = 0; i + 3 < filled; i++)
            if (buffer[i] == '\r' && buffer[i + 1] == '\n' && buffer[i + 2] == '\r' && buffer[i + 3] == '\n')
                return i;
        return -1;
    }

    static string Reason(int status) => status switch
    {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "",
    };
}
