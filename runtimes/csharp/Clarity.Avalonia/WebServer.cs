// The server that shows Avalonia windows in browsers (DESIGN 7.6): it
// serves the browser program's files and answers its sessions at
// SessionPath, one request at a time. It speaks HTTP over a plain socket, as
// the other stacks' servers do: Windows's own HTTP listener (http.sys) lets
// only an administrator listen on 127.0.0.1 or 0.0.0.0.

using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json.Nodes;

namespace Clarity.Avalonia;

public sealed class WebServer(Sessions sessions, string files)
{
    /// <summary>Where the browser program sends its sessions' requests.</summary>
    public const string SessionPath = "/clarity/session";

    /// <summary>A reply: its status, the kind of its body, the body, and how the body is compressed (none when it is not).</summary>
    public readonly record struct Reply(int Status, string Type, byte[] Body, string? Encoding = null);

    /// <summary>The compressed copies the publish writes beside each file, the smaller first.</summary>
    static readonly (string Encoding, string Extension)[] Copies = [("br", ".br"), ("gzip", ".gz")];

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
        if (path == SessionPath)
        {
            if (method != "POST")
                return Text(405, "text/plain", "send a session's request with POST");
            JsonObject answer;
            try
            {
                answer = sessions.Answer(JsonNode.Parse(body) as JsonObject ?? throw new SessionException("a request is a JSON object"));
            }
            catch (Exception e) when (e is System.Text.Json.JsonException or SessionException)
            {
                answer = new JsonObject { ["error"] = e.Message };
            }
            return Text(200, "application/json", answer.ToJsonString());
        }
        var file = Uri.UnescapeDataString(path.Split('?')[0]).TrimStart('/');
        if (file == "")
            file = "index.html";
        // Only the browser program's own files: nothing above its folder, on any system.
        if (file.Contains('\\') || file.Contains(':') || file.Split('/').Any(part => part is ".." or "." or ""))
            return Text(404, "text/plain", "");
        var full = Path.Combine(files, file);
        if (!File.Exists(full))
            return Text(404, "text/plain", "");
        // The compressed copy, to a browser that takes it: a few times smaller.
        var taken = Taken(accepts);
        foreach (var (encoding, extension) in Copies)
            if (taken.Contains(encoding) && File.Exists(full + extension))
                return new Reply(200, Kind(file), File.ReadAllBytes(full + extension), encoding);
        return new Reply(200, Kind(file), File.ReadAllBytes(full));
    }

    /// <summary>The encodings an Accept-Encoding names, without those it refuses (<c>;q=0</c>).</summary>
    static HashSet<string> Taken(string? accepts) =>
    [
        .. (accepts ?? "").Split(',')
            .Select(part => part.Split(';').Select(p => p.Trim()).ToArray())
            .Where(part => !part.Skip(1).Any(p => p.Replace(" ", "") is "q=0" or "q=0.0" or "q=0.00" or "q=0.000"))
            .Select(part => part[0].ToLowerInvariant()),
    ];

    /// <summary>The kind of a file of the browser program, by its extension.</summary>
    static string Kind(string file) => Path.GetExtension(file) switch
    {
        ".html" => "text/html; charset=utf-8",
        ".js" or ".mjs" => "text/javascript",
        ".css" => "text/css",
        ".json" => "application/json",
        ".wasm" => "application/wasm",
        ".svg" => "image/svg+xml",
        ".png" => "image/png",
        ".ico" => "image/x-icon",
        ".woff2" => "font/woff2",
        ".ttf" => "font/ttf",
        _ => "application/octet-stream",
    };

    /// <summary>Serves at <paramref name="address"/> until the program stops.</summary>
    public void Serve(string name, string address)
    {
        using var listening = Start(address);
        Console.Error.WriteLine($"{name}: serving on http://{address}");
        listening.Done.Wait();
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

    /// <summary>One request at a time, as the sessions are answered.</summary>
    readonly SemaphoreSlim one = new(1, 1);

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
                Reply reply;
                await one.WaitAsync(stop);
                try
                {
                    reply = Answer(start[0], start[1], Encoding.UTF8.GetString(body), headers.GetValueOrDefault("Accept-Encoding"));
                }
                finally
                {
                    one.Release();
                }
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
