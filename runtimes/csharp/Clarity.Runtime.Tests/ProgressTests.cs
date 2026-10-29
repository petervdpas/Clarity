// Progress of a long engine call and Stop, against the cases every host
// shares (conformance/progress.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class ProgressTests
{
    static readonly JsonElement Cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "progress.json"))).RootElement;
    static readonly byte[] Long = File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "conformance", "engines", "long.wasm"));

    static string[] Names(JsonElement list) => list.EnumerateArray().Select(e => e.GetString()!).ToArray();

    static Engine Load(string[] uses)
    {
        var limits = Cases.GetProperty("limits");
        return Engine.Load(Cases.GetProperty("name").GetString()!, Long, new EngineDeclaration(Names(Cases.GetProperty("functions")), uses, new EngineLimits(limits.GetProperty("seconds").GetUInt32(), limits.GetProperty("megabytes").GetUInt32())));
    }

    static List<(long, long)> Pairs(JsonElement list) => list.EnumerateArray().Select(p => (p[0].GetInt64(), p[1].GetInt64())).ToList();

    static object?[] Args(JsonElement list) => list.EnumerateArray().Select(a => (object?)a.GetInt64()).ToArray();

    [Fact]
    public void A_long_call_reports_how_far_it_is()
    {
        var engine = Load(Names(Cases.GetProperty("uses")));
        foreach (var c in Cases.GetProperty("reported").EnumerateArray())
        {
            var reports = new List<(long, long)>();
            var got = engine.CallWatched(0, c.GetProperty("function").GetString()!, new EngineWatch((done, total) => reports.Add((done, total))), Args(c.GetProperty("args")));
            Assert.Equal(c.GetProperty("gives").GetInt64(), got!.GetValue<long>());
            Assert.Equal(Pairs(c.GetProperty("progress")), reports);
        }
    }

    [Fact]
    public void Stop_ends_a_long_call()
    {
        var engine = Load(Names(Cases.GetProperty("uses")));
        foreach (var c in Cases.GetProperty("stopped").EnumerateArray())
        {
            var stopAt = c.GetProperty("stopAt").GetInt64();
            var reports = new List<(long, long)>();
            using var stop = new CancellationTokenSource();
            if (stopAt == 0)
                stop.Cancel();
            var watch = new EngineWatch((done, total) =>
            {
                lock (reports)
                    reports.Add((done, total));
                if (done == stopAt)
                    stop.Cancel();
            }, stop.Token);
            var error = Assert.Throws<EngineException>(() => engine.CallWatched(0, c.GetProperty("function").GetString()!, watch, Args(c.GetProperty("args"))));
            Assert.Equal(c.GetProperty("fails").GetString(), error.Message);
            var first = Pairs(c.GetProperty("firstReports"));
            lock (reports)
                Assert.Equal(first, reports.Take(first.Count));
        }
    }

    [Fact]
    public void Progress_is_a_service_the_engine_must_use()
    {
        var refused = Cases.GetProperty("refused");
        var error = Assert.Throws<EngineException>(() => Load(Names(refused.GetProperty("uses"))));
        Assert.Equal(refused.GetProperty("message").GetString(), error.Message);
    }
}
