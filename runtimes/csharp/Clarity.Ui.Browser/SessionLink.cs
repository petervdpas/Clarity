// How the browser program reaches the server's sessions (DESIGN 7.6): a
// request and its reply, as JSON, and the windows' markup the server serves.
// Over HTTP to the server the page came from; a test reaches sessions in its
// own process the same way.

using System.Text;
using System.Text.Json.Nodes;

namespace Clarity.Ui.Browser;

public interface ISessionLink
{
    /// <summary>Sends a session's request (<c>start</c>, <c>run</c>, <c>answer</c>, <c>fetch</c>, <c>close</c>) and gives its reply.</summary>
    Task<JsonObject> Send(JsonObject request);

    /// <summary>The windows' and app frames' markup and the themes the server's application carries, by name (<c>Windows/Main.cxaml</c>).</summary>
    Task<JsonObject> Markup();
}

/// <summary>The sessions of the server the page came from, over HTTP.</summary>
public sealed class HttpSessionLink(HttpClient http) : ISessionLink
{
    /// <summary>Where the server answers the sessions' requests (Clarity.Ui.Server's WebServer.SessionPath).</summary>
    public const string SessionPath = "/clarity/session";

    /// <summary>Where the server serves the windows' markup (Clarity.Ui.Server's WebServer.MarkupPath).</summary>
    public const string MarkupPath = "/clarity/markup";

    public async Task<JsonObject> Send(JsonObject request)
    {
        using var content = new StringContent(request.ToJsonString(), Encoding.UTF8, "application/json");
        using var reply = await http.PostAsync(SessionPath, content);
        return JsonNode.Parse(await reply.Content.ReadAsStringAsync()) as JsonObject ?? new JsonObject { ["error"] = "the server answered no JSON" };
    }

    public async Task<JsonObject> Markup() =>
        JsonNode.Parse(await http.GetStringAsync(MarkupPath)) as JsonObject ?? [];
}
