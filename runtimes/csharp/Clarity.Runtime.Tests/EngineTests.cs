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
    static EngineLimits Limits => new(Cases.GetProperty("limits").GetProperty("seconds").GetUInt32(), Cases.GetProperty("limits").GetProperty("megabytes").GetUInt32());
    static string[] Uses => Names(Cases.GetProperty("uses"));
    static string[] Names(JsonElement list) => list.EnumerateArray().Select(f => f.GetString()!).ToArray();

    [Fact]
    public void EveryCallGivesWhatEveryHostGives()
    {
        var engine = Engine.Load("Test", TestEngine, new EngineDeclaration(Names(Cases.GetProperty("functions")), Uses, Limits));
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
            var error = Assert.Throws<EngineException>(() => Engine.Load("Test", bytes, new EngineDeclaration(functions, refused.TryGetProperty("uses", out var u) ? Names(u) : Uses, Limits, Opens: refused.TryGetProperty("open", out var o) && o.GetBoolean())));
            Assert.Contains(refused.GetProperty("message").GetString()!, error.Message);
            Assert.StartsWith("engine Test ", error.Message);
        }
    }

    [Fact]
    public void AValueComesBackAsTheTypeCodeDeclares()
    {
        var engine = Engine.Load("Test", TestEngine, new EngineDeclaration(["add", "total"], Uses, Limits));
        Assert.Equal(42L, engine.Call<long>("add", 2L, 40L));
        Assert.Equal(29.95m, engine.Call<decimal>("total", new object[] { new[] { new { price = 9.95m, count = 3L }, new { price = 0.10m, count = 1L } } }));
    }

    [Fact]
    public void ACallKeepsToTheLimitsOfItsEngine()
    {
        var engine = Engine.Load("Test", TestEngine, new EngineDeclaration(Names(Cases.GetProperty("functions")), Uses, Limits));
        foreach (var call in Cases.GetProperty("limited").EnumerateArray())
        {
            var function = call.GetProperty("function").GetString()!;
            var args = call.GetProperty("args").EnumerateArray().Select(a => (object?)JsonNode.Parse(a.GetRawText())).ToArray();
            if (call.TryGetProperty("fails", out var fails))
            {
                var error = Assert.Throws<EngineException>(() => engine.Call(function, args));
                Assert.StartsWith(fails.GetString()!, error.Message);
            }
            else
            {
                var got = new JsonObject { ["value"] = engine.Call(function, args)?.DeepClone() };
                Assert.True(JsonNode.DeepEquals(JsonNode.Parse(call.GetProperty("gives").GetRawText()), got), $"{call}: {got}");
            }
        }
    }

    [Fact]
    public void AnEngineUsesTheServicesItDeclares()
    {
        var lines = new List<string>();
        var services = Cases.GetProperty("services");
        var engine = Engine.Load("Test", TestEngine, new EngineDeclaration(Names(Cases.GetProperty("functions")), Uses, Limits))
            .WithServices(new EngineServices(services.GetProperty("clock").GetInt64(), services.GetProperty("seed").GetUInt64(), lines.Add, () => services.GetProperty("language").GetString()));
        foreach (var call in Cases.GetProperty("served").EnumerateArray())
        {
            lines.Clear();
            var args = call.GetProperty("args").EnumerateArray().Select(a => (object?)JsonNode.Parse(a.GetRawText())).ToArray();
            var got = new JsonObject { ["value"] = engine.Call(call.GetProperty("function").GetString()!, args)?.DeepClone() };
            Assert.True(JsonNode.DeepEquals(JsonNode.Parse(call.GetProperty("gives").GetRawText()), got), $"{call}: {got}");
            Assert.Equal(call.TryGetProperty("log", out var log) ? Names(log) : [], lines);
        }
    }

    [Fact]
    public void AnEngineWithStateKeepsItsInstanceFromCallToCall()
    {
        var kept = Cases.GetProperty("kept");
        Assert.Equal("app", kept.GetProperty("state").GetString());
        var engine = Engine.Load("Test", TestEngine,
            new EngineDeclaration(Names(Cases.GetProperty("functions")), Uses, Limits, EngineState.PerApp, kept.GetProperty("open").GetBoolean()));
        foreach (var call in kept.GetProperty("calls").EnumerateArray())
        {
            var function = call.GetProperty("function").GetString()!;
            var args = call.GetProperty("args").EnumerateArray().Select(a => (object?)JsonNode.Parse(a.GetRawText())).ToArray();
            if (call.TryGetProperty("fails", out var fails))
            {
                Assert.StartsWith(fails.GetString()!, Assert.Throws<EngineException>(() => engine.Call(function, args)).Message);
                continue;
            }
            JsonNode got;
            try
            {
                got = new JsonObject { ["value"] = engine.Call(function, args)?.DeepClone() };
            }
            catch (RefusedException e)
            {
                got = new JsonObject { ["error"] = e.Message };
            }
            Assert.True(JsonNode.DeepEquals(JsonNode.Parse(call.GetProperty("gives").GetRawText()), got), $"{call}: {got}");
        }
    }

    [Fact]
    public void AnEngineWithStatePerWindowKeepsAnInstanceForEachWindow()
    {
        var windows = Cases.GetProperty("windows");
        Assert.Equal("window", windows.GetProperty("state").GetString());
        var engine = Engine.Load("Test", TestEngine,
            new EngineDeclaration(Names(Cases.GetProperty("functions")), Uses, Limits, EngineState.PerWindow, windows.GetProperty("open").GetBoolean()));
        foreach (var step in windows.GetProperty("steps").EnumerateArray())
        {
            if (step.TryGetProperty("close", out var closed))
            {
                engine.CloseWindow(closed.GetInt64());
                continue;
            }
            var got = new JsonObject { ["value"] = engine.CallIn(step.GetProperty("window").GetInt64(), step.GetProperty("function").GetString()!)?.DeepClone() };
            Assert.True(JsonNode.DeepEquals(JsonNode.Parse(step.GetProperty("gives").GetRawText()), got), $"{step}: {got}");
        }
    }

    [Fact]
    public void AnEngineWithStatePerWindowDropsAWindowsInstanceWhenItCloses()
    {
        var engine = Engine.Load("Test", TestEngine, new EngineDeclaration(Names(Cases.GetProperty("functions")), Uses, Limits, EngineState.PerWindow, true));
        var window = OpenWindows.Next();
        Assert.Equal(11, engine.CallIn(window, "count")!.GetValue<int>());
        Assert.Equal(12, engine.CallIn(window, "count")!.GetValue<int>());
        OpenWindows.Close(window);
        Assert.Equal(11, engine.CallIn(window, "count")!.GetValue<int>());
    }
}
