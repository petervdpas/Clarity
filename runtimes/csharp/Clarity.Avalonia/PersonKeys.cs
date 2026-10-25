// The person's keys (DESIGN 5.28), where the project file says `keys
// editable`: by command, only where they differ from the design's, kept in
// the settings file as `keys`, beside the settings. A command of the app's
// frame is known by its name, a window's own by `Window.name`.

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public static class PersonKeys
{
    static string? program;

    /// <summary>The person's keys by command.</summary>
    public static IReadOnlyDictionary<string, IReadOnlyList<string>> Choices { get; private set; } = new Dictionary<string, IReadOnlyList<string>>();

    /// <summary>The keys changed: the commands take them at once.</summary>
    public static event Action? Changed;

    /// <summary>Takes back what <paramref name="name"/> kept, and keeps every change for it from now on.</summary>
    public static void Start(string name)
    {
        program = name;
        Choices = Read(KeptSettings.Read(name)["keys"]);
        Changed?.Invoke();
    }

    /// <summary>Forgets the person's keys and keeps nothing more.</summary>
    public static void Stop()
    {
        program = null;
        Choices = new Dictionary<string, IReadOnlyList<string>>();
        Changed?.Invoke();
    }

    /// <summary>The person set their keys: the commands take them, and the settings file keeps them.</summary>
    public static void Set(IReadOnlyDictionary<string, IReadOnlyList<string>> choices)
    {
        Choices = new Dictionary<string, IReadOnlyList<string>>(choices);
        if (program is { } name)
        {
            var kept = KeptSettings.Read(name);
            kept["keys"] = Json();
            KeptSettings.Keep(name, kept);
        }
        Changed?.Invoke();
    }

    /// <summary>The keys a command answers: the person's, or the one its design gives.</summary>
    public static IReadOnlyList<string> Of(string id, string? designed) =>
        Choices.TryGetValue(id, out var keys) ? keys : designed is null ? [] : [designed];

    /// <summary>The person's keys as the settings file keeps them.</summary>
    public static JsonObject Json()
    {
        var keys = new JsonObject();
        foreach (var (id, list) in Choices)
            keys[id] = new JsonArray([.. list.Select(k => (JsonNode)JsonValue.Create(k))]);
        return keys;
    }

    /// <summary>What the file kept, as lists of keys only.</summary>
    static Dictionary<string, IReadOnlyList<string>> Read(JsonNode? kept)
    {
        var found = new Dictionary<string, IReadOnlyList<string>>();
        if (kept is not JsonObject keys)
            return found;
        foreach (var (id, value) in keys)
        {
            if (value is JsonArray list && list.All(k => k is JsonValue v && v.TryGetValue<string>(out _)))
                found[id] = [.. list.Select(k => k!.GetValue<string>())];
        }
        return found;
    }
}
