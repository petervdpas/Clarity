// HTTP calls from code (DESIGN 5.23): `http get url` and the others, through
// HttpClient. A call never fails: when no answer comes, the reply has status
// 0 and says why, so the handler decides what that means. The other runtimes
// give the same replies (conformance/http.json).

using System.Text;

namespace Clarity.Runtime;

/// <summary>What a web API answered: its status (0 when no answer came), whether it is ok (200 to 299), and its text, or why no answer came.</summary>
public sealed record HttpReply(long Status, bool Ok, string Text);

/// <summary>The calls code makes to web APIs, each with its headers as (name, value).</summary>
public static class Http
{
    /// <summary>How long a call waits for its whole answer.</summary>
    public static readonly TimeSpan Timeout = TimeSpan.FromSeconds(30);

    static readonly HttpClient Client = new() { Timeout = Timeout };

    public static HttpReply Get(string url, (string Name, string Value)[] headers) => Call(HttpMethod.Get, url, null, headers);

    public static HttpReply Post(string url, string body, (string Name, string Value)[] headers) => Call(HttpMethod.Post, url, body, headers);

    public static HttpReply Put(string url, string body, (string Name, string Value)[] headers) => Call(HttpMethod.Put, url, body, headers);

    public static HttpReply Delete(string url, (string Name, string Value)[] headers) => Call(HttpMethod.Delete, url, null, headers);

    /// <summary>The host a url names, without the scheme, a login, the port and the path.</summary>
    public static string Host(string url)
    {
        var at = url.IndexOf("://", StringComparison.Ordinal);
        var rest = at >= 0 ? url[(at + 3)..] : url;
        var end = rest.IndexOfAny(['/', '?', '#']);
        if (end >= 0)
            rest = rest[..end];
        var login = rest.LastIndexOf('@');
        if (login >= 0)
            rest = rest[(login + 1)..];
        if (rest.StartsWith('['))
        {
            var close = rest.IndexOf(']');
            return close >= 0 ? rest[1..close] : rest[1..];
        }
        var port = rest.IndexOf(':');
        return port >= 0 ? rest[..port] : rest;
    }

    /// <summary>Text as a part of a url: letters, digits and - . _ ~ as they are, every other byte of its UTF-8 as %XX.</summary>
    public static string Part(string text)
    {
        var out_ = new StringBuilder();
        foreach (var b in Encoding.UTF8.GetBytes(text))
        {
            if (b is >= (byte)'a' and <= (byte)'z' or >= (byte)'A' and <= (byte)'Z' or >= (byte)'0' and <= (byte)'9' or (byte)'-' or (byte)'.' or (byte)'_' or (byte)'~')
                out_.Append((char)b);
            else
                out_.Append('%').Append(b.ToString("X2"));
        }
        return out_.ToString();
    }

    /// <summary>A secret where the project file says it is: env:NAME reads that environment variable; empty when it is not set.</summary>
    static string Secret(string location) =>
        location.StartsWith("env:", StringComparison.Ordinal) ? Environment.GetEnvironmentVariable(location[4..]) ?? "" : location;

    /// <summary>Calls <paramref name="path"/> of the web API at <paramref name="baseAddress"/> (a data source's address) with the
    /// query parameters, encoded and in order, and the source's auth (<paramref name="authKind"/> bearer, api-key or basic, its
    /// secret read from <paramref name="secret"/>), as <c>http get "/orders" from shop query "status": s</c> does.</summary>
    public static HttpReply At(string method, string baseAddress, string path, (string Name, string Value)[] query, string? body, (string Name, string Value)[] headers, string? authKind, string? secret)
    {
        var url = baseAddress.Length == 0 || path.Contains("://") ? path : $"{baseAddress.TrimEnd('/')}/{path.TrimStart('/')}";
        for (var i = 0; i < query.Length; i++)
            url += (i == 0 && !url.Contains('?') ? "?" : "&") + Part(query[i].Name) + "=" + Part(query[i].Value);
        var all = headers.ToList();
        switch (authKind)
        {
            case "bearer":
                all.Add(("Authorization", "Bearer " + Secret(secret ?? "")));
                break;
            case "api-key":
                all.Add(("X-Api-Key", Secret(secret ?? "")));
                break;
            case "basic":
                all.Add(("Authorization", "Basic " + Convert.ToBase64String(Encoding.UTF8.GetBytes(Secret(secret ?? "")))));
                break;
        }
        var kind = method switch
        {
            "POST" => HttpMethod.Post,
            "PUT" => HttpMethod.Put,
            "DELETE" => HttpMethod.Delete,
            _ => HttpMethod.Get,
        };
        return Call(kind, url, body, [.. all]);
    }

    static HttpReply None(string why) => new(0, false, why);

    static HttpReply Call(HttpMethod method, string url, string? body, (string Name, string Value)[] headers)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var address) || (address.Scheme != Uri.UriSchemeHttp && address.Scheme != Uri.UriSchemeHttps))
            return None($"only http and https addresses are called, not {url}");
        using var request = new HttpRequestMessage(method, address);
        if (body is not null)
            request.Content = new StringContent(body, Encoding.UTF8, "text/plain");
        foreach (var (name, value) in headers)
        {
            if (request.Headers.TryAddWithoutValidation(name, value))
                continue;
            // A header of what is sent, such as Content-Type, goes with the body.
            if (request.Content is not null)
            {
                request.Content.Headers.Remove(name);
                request.Content.Headers.TryAddWithoutValidation(name, value);
            }
        }
        try
        {
            using var response = Client.Send(request);
            using var stream = response.Content.ReadAsStream();
            using var reader = new StreamReader(stream, Encoding.UTF8);
            var status = (long)response.StatusCode;
            return new HttpReply(status, status is >= 200 and < 300, reader.ReadToEnd());
        }
        catch (TaskCanceledException)
        {
            return None($"cannot reach {Host(url)}: no answer within {(int)Timeout.TotalSeconds} seconds");
        }
        catch (Exception e) when (e is HttpRequestException or IOException or InvalidOperationException)
        {
            return None($"cannot reach {Host(url)}: {e.Message}");
        }
    }
}
