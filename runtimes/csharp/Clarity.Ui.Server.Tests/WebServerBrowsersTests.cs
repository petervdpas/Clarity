// Every browser of a served program is its own (DESIGN 7.9, B2): a handler
// sees the settings and the language of the page that asked, a setting it
// changes goes back to that page alone, and quit and keys from code reach
// only the reply of the page whose handler ran them.

using System.Text.Json.Nodes;
using Xunit;

// The program's quit and keys go through the one desktop program a server has.
[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Clarity.Ui.Server.Tests;

public class WebServerBrowsersTests
{
    static JsonObject Ask(WebServer server, string body) => JsonNode.Parse(server.Answer("POST", WebServer.InvokePath, body).Body)!.AsObject();

    static long Open(WebServer server, string browser) =>
        Ask(server, Wire.Command("open_window", new JsonObject { ["window"] = "Shelf" }, browser))["value"]!["state"]!["instance"]!.GetValue<long>();

    static string Seen(JsonObject reply) => reply["value"]!["state"]!["data"]!["seen"]!.GetValue<string>();

    [Fact]
    public void A_handler_sees_the_settings_of_the_page_that_asked_and_changes_only_theirs()
    {
        var server = new WebServer(new ShelfViewModel(null));
        var a = Open(server, "a");
        var b = Open(server, "b");
        // Each page sends its settings as it keeps them.
        Assert.StartsWith("5 ", Seen(Ask(server, Wire.Dispatch("Shelf", a, "look", browser: "a", settings: new() { ["speed"] = 5 }))));
        Assert.StartsWith("9 ", Seen(Ask(server, Wire.Dispatch("Shelf", b, "look", browser: "b", settings: new() { ["speed"] = 9 }))));
        // A page that sends none keeps those it had.
        Assert.StartsWith("5 ", Seen(Ask(server, Wire.Dispatch("Shelf", a, "look", browser: "a"))));
        // A page that never sent any sees the server's own.
        var c = Open(server, "c");
        Assert.StartsWith("1 ", Seen(Ask(server, Wire.Dispatch("Shelf", c, "look", browser: "c"))));
        // A setting a handler changes goes back to its page, to keep; the other page's stay.
        var faster = Ask(server, Wire.Dispatch("Shelf", a, "faster", browser: "a", settings: new() { ["speed"] = 5, ["greeting"] = "hi" }));
        Assert.Equal(6, faster["settings"]!["speed"]!.GetValue<long>());
        Assert.StartsWith("9 ", Seen(Ask(server, Wire.Dispatch("Shelf", b, "look", browser: "b"))));
        Assert.StartsWith("6 ", Seen(Ask(server, Wire.Dispatch("Shelf", a, "look", browser: "a"))));
        // Unchanged, none comes back.
        Assert.Null(Ask(server, Wire.Dispatch("Shelf", b, "look", browser: "b", settings: new() { ["speed"] = 9, ["greeting"] = "hi" }))["settings"]);
    }

    [Fact]
    public void The_language_a_page_chose_is_that_pages_alone()
    {
        var server = new WebServer(new ShelfViewModel(null));
        var a = Open(server, "a");
        var b = Open(server, "b");
        var before = Seen(Ask(server, Wire.Dispatch("Shelf", b, "look", browser: "b")));
        Assert.Null(Ask(server, Wire.Command("set_language", new() { ["language"] = "nl" }, "a"))["error"]);
        Assert.EndsWith(" nl", Seen(Ask(server, Wire.Dispatch("Shelf", a, "look", browser: "a"))));
        // The other page, and the server itself, keep theirs.
        Assert.Equal(before, Seen(Ask(server, Wire.Dispatch("Shelf", b, "look", browser: "b"))));
        Assert.NotEqual("nl", System.Globalization.CultureInfo.CurrentCulture.Name);
    }

    [Fact]
    public void Quit_and_keys_from_code_reach_only_the_page_whose_handler_ran_them()
    {
        var server = new WebServer(new ShelfViewModel(null));
        var a = Open(server, "a");
        var b = Open(server, "b");
        var quit = Ask(server, Wire.Dispatch("Shelf", a, "quitNow", browser: "a"))["value"]!;
        Assert.True(quit["quit"]!.GetValue<bool>());
        var keys = Ask(server, Wire.Dispatch("Shelf", a, "keysNow", browser: "a"))["value"]!;
        Assert.True(keys["keys"]!.GetValue<bool>());
        Assert.Null(keys["quit"]);
        // The other page's next reply says neither, nor does the first page's after.
        var other = Ask(server, Wire.Dispatch("Shelf", b, "stepped", browser: "b"))["value"]!;
        Assert.Null(other["quit"]);
        Assert.Null(other["keys"]);
        Assert.Null(Ask(server, Wire.Dispatch("Shelf", a, "stepped", browser: "a"))["value"]!["quit"]);
    }
}
