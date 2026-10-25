// Keys set by the person, as every renderer sets them (conformance/shortcut-keys.json).

using System.Text.Json;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class ShortcutKeyCaseTests
{
    static readonly JsonElement Cases =
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "shortcut-keys.json"))).RootElement;

    static IReadOnlyList<string> Strings(JsonElement list) => [.. list.EnumerateArray().Select(e => e.GetString()!)];

    static readonly KeyRules.Command[] Commands =
        [.. Cases.GetProperty("commands").EnumerateArray().Select(c => new KeyRules.Command(c.GetProperty("id").GetString()!, Strings(c.GetProperty("layers")), Strings(c.GetProperty("keys"))))];

    static Dictionary<string, IReadOnlyList<string>> Choices(JsonElement e) =>
        e.EnumerateObject().ToDictionary(p => p.Name, p => Strings(p.Value));

    static string Shown(IReadOnlyDictionary<string, IReadOnlyList<string>> choices) =>
        string.Join("; ", choices.OrderBy(p => p.Key, StringComparer.Ordinal).Select(p => $"{p.Key}={string.Join(",", p.Value)}"));

    [Fact]
    public void Assign_as_every_renderer_does()
    {
        foreach (var c in Cases.GetProperty("assign").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var choices = Choices(c.GetProperty("choices"));
            var (id, key) = (c.GetProperty("id").GetString()!, c.GetProperty("key").GetString()!);
            var expect = c.GetProperty("expect");
            var (next, swapped) = KeyRules.Assign(Commands, choices, id, key);
            Assert.True(Shown(Choices(expect.GetProperty("choices"))) == Shown(next), $"{name}: {Shown(next)}");
            Assert.True(expect.GetProperty("swapped").ValueKind == JsonValueKind.Null ? swapped is null : swapped == expect.GetProperty("swapped").GetString(), $"{name}: {swapped}");
            Assert.Equal(Strings(expect.GetProperty("also")), KeyRules.AlsoHas(Commands, choices, id, key));
        }
    }

    [Fact]
    public void Reset_as_every_renderer_does()
    {
        foreach (var c in Cases.GetProperty("reset").EnumerateArray())
        {
            var next = KeyRules.Reset(Commands, Choices(c.GetProperty("choices")), c.GetProperty("id").GetString()!);
            Assert.True(Shown(Choices(c.GetProperty("expect"))) == Shown(next), $"{c.GetProperty("name").GetString()}: {Shown(next)}");
        }
    }

    [Fact]
    public void Refused_as_every_renderer_does()
    {
        foreach (var c in Cases.GetProperty("refused").EnumerateArray())
        {
            var key = c[0].GetString()!;
            Assert.True((c[1].ValueKind == JsonValueKind.Null ? null : c[1].GetString()) == KeyRules.Refused(key), key);
        }
    }
}
