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
