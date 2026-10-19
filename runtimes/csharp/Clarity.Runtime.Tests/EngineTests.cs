// Engines hosted by the C# runtime, against the cases every host shares
// (conformance/engines.json).

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class EngineTests
{
    static readonly string Folder = Path.Combine(AppContext.BaseDirectory, "conformance");
    static readonly JsonElement Cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(Folder, "engines.json"))).RootElement;
    static byte[] TestEngine => File.ReadAllBytes(Path.Combine(Folder, Cases.GetProperty("engine").GetString()!));
    static string[] Names(JsonElement list) => list.EnumerateArray().Select(f => f.GetString()!).ToArray();

    [Fact]
    public void EveryCallGivesWhatEveryHostGives()
    {
        var engine = Engine.Load("Test", TestEngine, Names(Cases.GetProperty("functions")));
        foreach (var call in Cases.GetProperty("calls").EnumerateArray())
        {
            var args = call.GetProperty("args").EnumerateArray().Select(a => (object?)JsonNode.Parse(a.GetRawText())).ToArray();
            JsonNode got;
            try
            {
                got = new JsonObject { ["value"] = engine.Call(call.GetProperty("function").GetString()!, args)?.DeepClone() };
            }
            catch (RefusedException e)
            {
                got = new JsonObject { ["error"] = e.Message };
            }
            Assert.True(JsonNode.DeepEquals(JsonNode.Parse(call.GetProperty("gives").GetRawText()), got), $"{call}: {got}");
        }
    }

    [Fact]
    public void AModuleThatIsNoEngineIsRefusedWhenItLoads()
    {
        foreach (var refused in Cases.GetProperty("refused").EnumerateArray())
        {
            var bytes = refused.TryGetProperty("bytes", out var hex) ? Convert.FromHexString(hex.GetString()!) : TestEngine;
            var functions = refused.TryGetProperty("functions", out var f) ? Names(f) : [];
            var error = Assert.Throws<EngineException>(() => Engine.Load("Test", bytes, functions));
            Assert.Contains(refused.GetProperty("message").GetString()!, error.Message);
            Assert.StartsWith("engine Test ", error.Message);
        }
    }

    [Fact]
    public void AValueComesBackAsTheTypeCodeDeclares()
    {
        var engine = Engine.Load("Test", TestEngine, ["add", "total"]);
        Assert.Equal(42L, engine.Call<long>("add", 2L, 40L));
        Assert.Equal(29.95m, engine.Call<decimal>("total", new object[] { new[] { new { price = 9.95m, count = 3L }, new { price = 0.10m, count = 1L } } }));
    }
}
