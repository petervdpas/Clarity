// The server that shows Avalonia windows in browsers (DESIGN 7.6): it
// serves the browser program's files and answers its sessions at
// SessionPath, one request at a time, with .NET's own HTTP listener.

using System.Net;
using System.Text;
using System.Text.Json.Nodes;

namespace Clarity.Avalonia;

public sealed class WebServer(Sessions sessions, string files)
{
    /// <summary>Where the browser program sends its sessions' requests.</summary>
    public const string SessionPath = "/clarity/session";

    /// <summary>A reply: its status, the kind of its body, and the body.</summary>
    public readonly record struct Reply(int Status, string Type, byte[] Body);

    /// <summary>Where the server listens: <c>--address host:port</c>, else CLARITY_ADDRESS, else <paramref name="project"/>, where the project file says.</summary>
    public static string Address(string[] args, string project)
    {
        var at = Array.IndexOf(args, "--address");
        if (at >= 0 && at + 1 < args.Length)
            return args[at + 1];
        return Environment.GetEnvironmentVariable("CLARITY_ADDRESS") is { Length: > 0 } address ? address : project;
    }

    /// <summary>The listener's prefix for <paramref name="address"/>: every address of the machine for 0.0.0.0.</summary>
    public static string Prefix(string address)
    {
        var colon = address.LastIndexOf(':');
        var host = address[..colon];
        return $"http://{(host is "0.0.0.0" or "" ? "+" : host)}:{address[(colon + 1)..]}/";
    }

    static Reply Text(int status, string type, string body) => new(status, type, Encoding.UTF8.GetBytes(body));

    /// <summary>The reply to one request.</summary>
    public Reply Answer(string method, string path, string body)
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
        var file = path.Split('?')[0].TrimStart('/');
        if (file == "")
            file = "index.html";
        // Only the browser program's own files: nothing above its folder.
        if (file.Split('/').Any(part => part is ".." or ""))
            return Text(404, "text/plain", "");
        var full = Path.Combine(files, file);
        return File.Exists(full) ? new Reply(200, Kind(file), File.ReadAllBytes(full)) : Text(404, "text/plain", "");
    }

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
        using var listener = new HttpListener();
        listener.Prefixes.Add(Prefix(address));
        listener.Start();
        Console.Error.WriteLine($"{name}: serving on http://{address}");
        while (listener.IsListening)
        {
            var context = listener.GetContext();
            using var reader = new StreamReader(context.Request.InputStream, Encoding.UTF8);
            var reply = Answer(context.Request.HttpMethod, context.Request.Url?.AbsolutePath ?? "/", reader.ReadToEnd());
            context.Response.StatusCode = reply.Status;
            context.Response.ContentType = reply.Type;
            context.Response.ContentLength64 = reply.Body.Length;
            context.Response.OutputStream.Write(reply.Body);
            context.Response.Close();
        }
    }
}
