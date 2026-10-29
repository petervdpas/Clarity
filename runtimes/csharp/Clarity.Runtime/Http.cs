// HTTP calls from code (DESIGN 5.23): `http get url` and the others, through
// HttpClient. A call never fails: when no answer comes, the reply has status
// 0 and says why, so the handler decides what that means. The other runtimes
// give the same replies (conformance/http.json).

using System.Text;

namespace Clarity.Runtime;

/// <summary>What a web API answered: its status (0 when no answer came), whether it is ok (200 to 299), and its text, or why no answer came.</summary>
public sealed record HttpReply(long Status, bool Ok, string Text);

/// <summary>How a web API hands out a long list (DESIGN 5.23, paging): the project file's paging line. Kind is page, offset,
/// cursor or link; Items is where a page's items are in its JSON, empty when a page is a list.</summary>
public sealed record HttpPaging(string Kind, string Param, string SizeParam, int Size, string Next, string Items);

/// <summary>The calls code makes to web APIs, each with its headers as (name, value).</summary>
public static class Http
{
    /// <summary>How long a call waits for its whole answer.</summary>
    public static readonly TimeSpan Timeout = TimeSpan.FromSeconds(30);

    static readonly HttpClient Client = new() { Timeout = Timeout };

    public static HttpReply Get(string url, (string Name, string Value)[] headers) => Call(HttpMethod.Get, url, null, headers);

    public static HttpReply Post(string url, string body, (string Name, string Value)[] headers) => Call(HttpMethod.Post, url, body, headers);

    public static HttpReply Put(string url, string body, (string Name, string Value)[] headers) => Call(HttpMethod.Put, url, body, headers);

    public static HttpReply Patch(string url, string body, (string Name, string Value)[] headers) => Call(HttpMethod.Patch, url, body, headers);

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

    /// <summary>A secret where the project file says it is: env:NAME reads that environment variable, setting:program/name
    /// a secret setting; empty when it is not there.</summary>
    static string Secret(string location) =>
        location.StartsWith("env:", StringComparison.Ordinal) ? Environment.GetEnvironmentVariable(location[4..]) ?? ""
        // A secret setting (DESIGN 5.27): setting:<program>/<name>, from the system's protected store.
        : location.StartsWith("setting:", StringComparison.Ordinal) && location[8..].Split('/', 2) is [var program, var name] ? Secrets.Get(program, name) ?? ""
        : location;

    /// <summary>Calls <paramref name="path"/> of the web API at <paramref name="baseAddress"/> (a data source's address) with the
    /// query parameters, encoded and in order, and the source's auth (<paramref name="authKind"/> bearer, api-key or basic, its
    /// secret read from <paramref name="secret"/>), as <c>http get "/orders" from shop query "status": s</c> does.</summary>
    public static HttpReply At(string method, string baseAddress, string path, (string Name, string Value)[] query, string? body, (string Name, string Value)[] headers, string? authKind, string? secret) =>
        FetchAt(method, baseAddress, path, query, body, headers, authKind, secret).Reply;

    /// <summary>At most this many pages are read.</summary>
    public const int MostPages = 1000;

    /// <summary>The JSON at a dotted path of <paramref name="value"/>: meta.next; the value itself for "".</summary>
    static System.Text.Json.JsonElement? JsonAt(System.Text.Json.JsonElement value, string path)
    {
        if (path.Length == 0)
            return value;
        foreach (var step in path.Split('.'))
        {
            if (value.ValueKind != System.Text.Json.JsonValueKind.Object || !value.TryGetProperty(step, out value))
                return null;
        }
        return value;
    }

    /// <summary>Gets <paramref name="path"/> of the web API at <paramref name="baseAddress"/> page after page, as
    /// <paramref name="paging"/> says, as <c>http get "/orders" from shop every page</c> does: one reply whose text is one
    /// JSON list of the items of every page (conformance/http.json).</summary>
    public static HttpReply Pages(string baseAddress, string path, (string Name, string Value)[] query, (string Name, string Value)[] headers, string? authKind, string? secret, HttpPaging paging)
    {
        var items = new List<string>();
        string? cursor = null, link = null;
        for (var n = 0; n < MostPages; n++)
        {
            var sent = query.ToList();
            switch (paging.Kind)
            {
                case "page":
                    sent.Add((paging.Param, (n + 1).ToString(System.Globalization.CultureInfo.InvariantCulture)));
                    sent.Add((paging.SizeParam, paging.Size.ToString(System.Globalization.CultureInfo.InvariantCulture)));
                    break;
                case "offset":
                    sent.Add((paging.Param, (n * paging.Size).ToString(System.Globalization.CultureInfo.InvariantCulture)));
                    sent.Add((paging.SizeParam, paging.Size.ToString(System.Globalization.CultureInfo.InvariantCulture)));
                    break;
                case "cursor" when cursor is not null:
                    sent.Add((paging.Param, cursor));
                    break;
            }
            // A next link is a whole address, with its own query.
            var (reply, next) = link is not null
                ? FetchAt("GET", "", link, [], null, headers, authKind, secret)
                : FetchAt("GET", baseAddress, path, [.. sent], null, headers, authKind, secret);
            if (!reply.Ok)
                return reply;
            System.Text.Json.JsonDocument? page = null;
            try
            {
                page = System.Text.Json.JsonDocument.Parse(reply.Text);
            }
            catch (System.Text.Json.JsonException)
            {
            }
            using (page)
            {
                var list = page is null ? null : JsonAt(page.RootElement, paging.Items);
                if (list is not { ValueKind: System.Text.Json.JsonValueKind.Array } found)
                    return None($"page {n + 1} of {path} has no list at {(paging.Items.Length == 0 ? "the top" : paging.Items)}");
                var count = 0;
                foreach (var item in found.EnumerateArray())
                {
                    // Written compactly, without reflection.
                    using var buffer = new MemoryStream();
                    using (var writer = new System.Text.Json.Utf8JsonWriter(buffer))
                        item.WriteTo(writer);
                    items.Add(Encoding.UTF8.GetString(buffer.ToArray()));
                    count++;
                }
                bool done;
                switch (paging.Kind)
                {
                    case "cursor":
                        var at = JsonAt(page!.RootElement, paging.Next);
                        cursor = at switch
                        {
                            { ValueKind: System.Text.Json.JsonValueKind.String } t when t.GetString() is { Length: > 0 } text => text,
                            { ValueKind: System.Text.Json.JsonValueKind.Number } number => number.GetRawText(),
                            _ => null,
                        };
                        done = cursor is null;
                        break;
                    case "link":
                        link = next;
                        done = link is null;
                        break;
                    default:
                        done = count < paging.Size;
                        break;
                }
                if (done)
                    return new HttpReply(reply.Status, true, "[" + string.Join(",", items) + "]");
            }
        }
        return None($"stopped after {MostPages} pages of {path}");
    }

    /// <summary>The address a Link header names rel="next": &lt;url&gt;; rel="next".</summary>
    static string? NextLink(string header)
    {
        foreach (var part in header.Split(','))
        {
            var semicolon = part.IndexOf(';');
            if (semicolon < 0)
                continue;
            var next = part[(semicolon + 1)..].Split(';').Any(p => p.Trim().Replace(" ", "") is var q
                && (q.Equals("rel=\"next\"", StringComparison.OrdinalIgnoreCase) || q.Equals("rel=next", StringComparison.OrdinalIgnoreCase)));
            if (next)
                return part[..semicolon].Trim().TrimStart('<').TrimEnd('>');
        }
        return null;
    }

    /// <summary>A call as <see cref="At"/> makes it, with the address its Link header names next.</summary>
    static (HttpReply Reply, string? Next) FetchAt(string method, string baseAddress, string path, (string Name, string Value)[] query, string? body, (string Name, string Value)[] headers, string? authKind, string? secret)
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
            "PATCH" => HttpMethod.Patch,
            "DELETE" => HttpMethod.Delete,
            _ => HttpMethod.Get,
        };
        return Fetch(kind, url, body, [.. all]);
    }

    static HttpReply None(string why) => new(0, false, why);

    static HttpReply Call(HttpMethod method, string url, string? body, (string Name, string Value)[] headers) => Fetch(method, url, body, headers).Reply;

    /// <summary>A call as <see cref="Call"/> makes it, with the address its Link header names next.</summary>
    static (HttpReply Reply, string? Next) Fetch(HttpMethod method, string url, string? body, (string Name, string Value)[] headers)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var address) || (address.Scheme != Uri.UriSchemeHttp && address.Scheme != Uri.UriSchemeHttps))
            return (None($"only http and https addresses are called, not {url}"), null);
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
            var next = response.Headers.TryGetValues("Link", out var links) ? links.Select(NextLink).FirstOrDefault(l => l is not null) : null;
            return (new HttpReply(status, status is >= 200 and < 300, reader.ReadToEnd()), next);
        }
        catch (TaskCanceledException)
        {
            return (None($"cannot reach {Host(url)}: no answer within {(int)Timeout.TotalSeconds} seconds"), null);
        }
        catch (Exception e) when (e is HttpRequestException or IOException or InvalidOperationException)
        {
            return (None($"cannot reach {Host(url)}: {e.Message}"), null);
        }
    }
}
