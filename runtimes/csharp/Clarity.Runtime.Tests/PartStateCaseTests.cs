// The part state code reads and sets, as C# sends it to the library and takes
// it back (WireJson.Part, WireJson.ApplyPart), through the cases every stack
// runs: an accordion's open section, a foldable part, a ribbon's tab, a
// sidebar and a wizard's step (conformance/accordions.json, folding.json,
// ribbons.json, sidebars.json, steps.json).

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class PartStateCaseTests
{
    static JsonElement Cases(string file) =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", file))).RootElement;

    /// <summary>The part's state as the library sent it, given to a part as the design made it.</summary>
    static PartState Sent(JsonElement part, Action<PartState>? design = null)
    {
        var state = new PartState("part");
        design?.Invoke(state);
        WireJson.ApplyPart(state, JsonNode.Parse(part.GetRawText())!.AsObject());
        return state;
    }

    /// <summary>What the part sends back holds every field the case names, with its value.</summary>
    static void Sends(PartState state, JsonElement part, string what)
    {
        var sent = WireJson.Part(state, null);
        foreach (var field in part.EnumerateObject())
            Assert.True(JsonNode.DeepEquals(JsonNode.Parse(field.Value.GetRawText()), sent[field.Name]),
                $"{what}: sends {field.Name} {sent[field.Name]?.ToJsonString() ?? "nothing"}, want {field.Value.GetRawText()}");
    }

    [Fact]
    public void An_accordions_open_section_is_as_every_runtime_says()
    {
        var file = Cases("accordions.json");
        foreach (var c in file.GetProperty("open").EnumerateArray())
            Assert.Equal(c.GetProperty("open").GetInt64(), Sent(c.GetProperty("part")).OpenSection);
        foreach (var c in file.GetProperty("set").EnumerateArray())
            Sends(new PartState("part") { OpenSection = c.GetProperty("open").GetInt64() }, c.GetProperty("part"), $"open {c.GetProperty("open")}");
    }

    [Fact]
    public void A_foldable_part_is_folded_as_every_runtime_says()
    {
        var file = Cases("folding.json");
        foreach (var c in file.GetProperty("cases").EnumerateArray())
            Assert.Equal(c.GetProperty("folded").GetBoolean(), Sent(c.GetProperty("part"), p => p.IsFolded = c.GetProperty("design").GetBoolean()).IsFolded);
        foreach (var c in file.GetProperty("set").EnumerateArray())
            Sends(new PartState("part") { IsFolded = c.GetProperty("folded").GetBoolean() }, c.GetProperty("part"), $"folded {c.GetProperty("folded")}");
    }

    [Fact]
    public void A_ribbons_open_tab_is_as_every_runtime_says()
    {
        var file = Cases("ribbons.json");
        foreach (var c in file.GetProperty("tab").EnumerateArray())
            Assert.Equal(c.GetProperty("tab").GetInt64(), Sent(c.GetProperty("part")).Tab);
        foreach (var c in file.GetProperty("set").EnumerateArray())
            Sends(new PartState("part") { Tab = c.GetProperty("tab").GetInt64() }, c.GetProperty("part"), $"tab {c.GetProperty("tab")}");
    }

    [Fact]
    public void A_sidebar_is_collapsed_and_maximized_as_every_runtime_says()
    {
        var file = Cases("sidebars.json");
        foreach (var c in file.GetProperty("collapsed").EnumerateArray())
            Assert.Equal(c.GetProperty("collapsed").GetBoolean(), Sent(c.GetProperty("part"), p => p.IsCollapsed = c.GetProperty("design").GetBoolean()).IsCollapsed);
        foreach (var c in file.GetProperty("maximized").EnumerateArray())
            Assert.Equal(c.GetProperty("maximized").GetBoolean(), Sent(c.GetProperty("part")).IsMaximized);
        foreach (var c in file.GetProperty("set").EnumerateArray())
        {
            var state = new PartState("part");
            if (c.TryGetProperty("collapsed", out var collapsed))
                state.IsCollapsed = collapsed.GetBoolean();
            if (c.TryGetProperty("maximized", out var maximized))
                state.IsMaximized = maximized.GetBoolean();
            Sends(state, c.GetProperty("part"), c.GetRawText());
        }
    }

    [Fact]
    public void A_wizards_step_is_as_every_runtime_says()
    {
        var file = Cases("steps.json");
        foreach (var c in file.GetProperty("step").EnumerateArray())
            Assert.Equal(c.GetProperty("step").GetInt64(), Sent(c.GetProperty("part")).Step);
        foreach (var c in file.GetProperty("set").EnumerateArray())
            Sends(new PartState("part") { Step = c.GetProperty("step").GetInt64() }, c.GetProperty("part"), $"step {c.GetProperty("step")}");
    }
}
