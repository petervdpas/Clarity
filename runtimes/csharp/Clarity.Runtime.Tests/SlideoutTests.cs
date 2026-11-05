// A slide-out's state as every runtime keeps it (conformance/slideouts.json):
// closed until it is opened, pinned only when the person or code pins it,
// and both carried on the wire between the window and Clarity.UI.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class SlideoutTests
{
    static readonly JsonObject File = JsonNode.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "slideouts.json")))!.AsObject();

    [Fact]
    public void A_slideout_is_open_and_pinned_as_every_runtime_says()
    {
        foreach (var c in File["slideout"]!.AsArray())
        {
            // The generator starts it closed, as the window's view model writes it.
            var part = new PartState("details") { IsVisible = false };
            WireJson.ApplyPart(part, c!["part"]!.AsObject());
            Assert.Equal(c["visible"]!.GetValue<bool>(), part.IsVisible);
            Assert.Equal(c["pinned"]!.GetValue<bool>(), part.IsPinned);
        }
    }

    [Fact]
    public void Setting_it_sends_it_to_the_window()
    {
        foreach (var c in File["set"]!.AsArray())
        {
            var part = new PartState("details");
            if (c!["visible"] is { } visible)
                part.IsVisible = visible.GetValue<bool>();
            if (c["pinned"] is { } pinned)
                part.IsPinned = pinned.GetValue<bool>();
            var sent = WireJson.Part(part, null);
            foreach (var (name, value) in c["part"]!.AsObject())
                Assert.Equal(value!.GetValue<bool>(), sent[name]!.GetValue<bool>());
        }
    }
}
