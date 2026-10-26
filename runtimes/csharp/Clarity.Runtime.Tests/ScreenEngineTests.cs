// Engines beside the screen on Avalonia (DESIGN 5.12): Wasmtime in the
// application's process, one instance per canvas, its frames stopped by the
// epoch when they run past the limit (conformance/screen-engines.json).

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class ScreenEngineTests
{
    static readonly string Folder = Path.Combine(AppContext.BaseDirectory, "conformance");
    static readonly JsonElement Cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(Folder, "screen-engines.json"))).RootElement;
    static byte[] Screen => File.ReadAllBytes(Path.Combine(Folder, "engines", Cases.GetProperty("engine").GetString()!));
    static string Name => Cases.GetProperty("name").GetString()!;

    static ScreenEngine Load(bool opens, uint seconds = 5, byte[]? bytes = null) =>
        ScreenEngine.Load(Name, bytes ?? Screen, [], opens, new EngineLimits(seconds, 256));

    [Fact]
    public void Each_frame_gives_the_canvas_s_shapes()
    {
        foreach (var run in Cases.GetProperty("frames").EnumerateArray())
        {
            using var engine = Load(run.GetProperty("opens").GetBoolean());
            var ticks = run.GetProperty("ticks").EnumerateArray().Select(t => t.GetDouble()).ToArray();
            var draws = run.GetProperty("draws").EnumerateArray().ToArray();
            for (var i = 0; i < ticks.Length; i++)
            {
                var drawn = engine.Frame(ticks[i], []);
                Assert.True(JsonNode.DeepEquals(JsonNode.Parse(draws[i].GetRawText()), drawn), $"{run.GetProperty("name")}, frame {i}: {drawn.ToJsonString()}");
            }
        }
    }

    [Fact]
    public void A_frame_that_fails_or_runs_too_long_ends_the_engine()
    {
        foreach (var fails in Cases.GetProperty("fails").EnumerateArray())
        {
            using var engine = Load(true, fails.GetProperty("seconds").GetUInt32());
            foreach (var ms in fails.GetProperty("ticks").EnumerateArray())
                engine.Frame(ms.GetDouble(), []);
            var e = Assert.Throws<EngineException>(() => engine.Frame(fails.GetProperty("tick").GetDouble(), []));
            Assert.Equal(fails.GetProperty("fails").GetString(), e.Message);
            // Ended, it draws no more.
            Assert.Equal(e.Message, Assert.Throws<EngineException>(() => engine.Frame(16, [])).Message);
        }
    }

    /// <summary>An input as the conformance file writes it.</summary>
    static ScreenInput InputOf(JsonElement i) =>
        i.TryGetProperty("pointer", out var kind)
            ? new ScreenInput.Pointer(kind.GetString()!, i.GetProperty("x").GetInt64(), i.GetProperty("y").GetInt64())
            : new ScreenInput.Key(i.GetProperty("key").GetString()!, i.GetProperty("name").GetString()!);

    [Fact]
    public void Pointer_and_keys_reach_the_engine_before_its_tick()
    {
        foreach (var run in Cases.GetProperty("input").EnumerateArray())
        {
            using var engine = Load(run.GetProperty("opens").GetBoolean());
            Assert.Equal(new Hears(true, true), engine.Hears);
            var i = 0;
            foreach (var frame in run.GetProperty("frames").EnumerateArray())
            {
                var drawn = engine.Frame(frame.GetProperty("tick").GetDouble(), frame.GetProperty("input").EnumerateArray().Select(InputOf).ToList());
                Assert.True(JsonNode.DeepEquals(JsonNode.Parse(frame.GetProperty("draws").GetRawText()), drawn), $"{run.GetProperty("name")}, frame {i++}: {drawn.ToJsonString()}");
            }
        }
    }

    [Fact]
    public void Moves_in_a_row_keep_only_the_last()
    {
        foreach (var c in Cases.GetProperty("queued").EnumerateArray())
        {
            var waiting = new List<ScreenInput>();
            foreach (var given in c.GetProperty("given").EnumerateArray())
                ScreenInput.Queue(waiting, InputOf(given));
            Assert.Equal(c.GetProperty("sent").EnumerateArray().Select(InputOf), waiting);
        }
    }

    [Fact]
    public void An_engine_beside_the_screen_exports_its_frame_loop()
    {
        var test = File.ReadAllBytes(Path.Combine(Folder, "engines", "test.wasm"));
        var e = Assert.Throws<EngineException>(() => ScreenEngine.Load("Test", test, ["clock", "random", "log", "language"], false, new EngineLimits(5, 256)));
        Assert.Equal("engine Test lacks clarity_tick, which an engine beside the screen needs", e.Message);
    }
}
