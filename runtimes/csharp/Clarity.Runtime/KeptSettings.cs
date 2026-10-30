// The settings each person keeps for themselves (DESIGN 5.17), the same
// as on every stack (conformance/settings.json): one JSON file per
// application in the folder for application settings of the person's
// system, and of what was kept only what still fits its setting is taken
// back. The generated Settings describe their settings and hold the values.

using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;

namespace Clarity.Runtime;

/// <summary>
/// A setting as a kept value is checked against it: its type, its lowest
/// and highest number, and the values it allows (an enum's, the languages,
/// the themes).
/// </summary>
public sealed record Setting(string Type, string? Min = null, string? Max = null, IReadOnlyList<string>? Values = null);

public static partial class KeptSettings
{
    /// <summary>
    /// Where settings are kept instead of a file (DESIGN 7.6): in a browser,
    /// its own storage, by a key; none for the file. A store keeps no secret:
    /// a browser has no protected store, so the person gives them again.
    /// </summary>
    public static (Func<string, string?> Read, Action<string, string> Write)? Store { get; set; }

    static string StoreKey(string program) => $"settings:{program}";

    /// <summary>Where <paramref name="program"/> keeps its settings on system <paramref name="os"/>, with <paramref name="env"/> giving the environment; null when it names no folder for them.</summary>
    public static string? FileIn(string os, Func<string, string?> env, string program)
    {
        string? Set(string name) => env(name) is { Length: > 0 } value ? value : null;
        var folder = os switch
        {
            "windows" => Set("APPDATA"),
            "macos" => Set("HOME") is { } home ? $"{home}/Library/Application Support" : null,
            _ => Set("XDG_CONFIG_HOME") ?? (Set("HOME") is { } home ? $"{home}/.config" : null),
        };
        return folder is null ? null : $"{folder}/{program}/settings.json";
    }

    /// <summary>This machine's system, as the cases name it.</summary>
    public static string Os => OperatingSystem.IsWindows() ? "windows" : OperatingSystem.IsMacOS() ? "macos" : "linux";

    /// <summary>Where <paramref name="program"/> keeps its settings on this machine.</summary>
    public static string? File(string program) => FileIn(Os, Environment.GetEnvironmentVariable, program);

    /// <summary>
    /// The folder of file store <paramref name="store"/> when a folder setting names it, on
    /// system <paramref name="os"/>: the kept value when it is text that is not empty, else
    /// <paramref name="fallback"/> (the setting's default), else a folder named after the
    /// store; a relative folder is in <paramref name="settingsFolder"/>, the folder of the
    /// settings file.
    /// </summary>
    public static string StoreFolderIn(string os, string settingsFolder, string store, JsonNode? kept, string fallback)
    {
        var chosen = kept is JsonValue v && v.TryGetValue<string>(out var text) && text.Length > 0 ? text : fallback;
        if (chosen.Length == 0)
            chosen = store;
        var absolute = os == "windows"
            ? chosen.StartsWith(@"\\", StringComparison.Ordinal) || chosen.StartsWith("//", StringComparison.Ordinal)
                || (chosen.Length > 2 && char.IsAsciiLetter(chosen[0]) && chosen[1] == ':' && chosen[2] is '/' or '\\')
            : chosen.StartsWith('/');
        return absolute ? chosen : $"{settingsFolder}/{chosen}";
    }

    /// <summary>The settings <paramref name="program"/> kept; none when it kept none, or they cannot be read.</summary>
    public static JsonObject Read(string program)
    {
        try
        {
            if (Store is { } store)
                return store.Read(StoreKey(program)) is { } text && JsonNode.Parse(text) is JsonObject stored ? stored : [];
            return File(program) is { } file && System.IO.File.Exists(file) && JsonNode.Parse(System.IO.File.ReadAllText(file)) is JsonObject kept ? kept : [];
        }
        catch (Exception e) when (e is IOException or JsonException or UnauthorizedAccessException)
        {
            return [];
        }
    }

    /// <summary>Keeps <paramref name="values"/> as the settings of <paramref name="program"/>, written whole and then put in place.</summary>
    public static void Keep(string program, JsonObject values)
    {
        if (Store is { } store)
        {
            store.Write(StoreKey(program), values.ToJsonString());
            return;
        }
        var file = File(program) ?? throw new InvalidOperationException("this system names no folder for application settings");
        Directory.CreateDirectory(Path.GetDirectoryName(file)!);
        var written = file + ".new";
        // Plain text, as the other runtimes write it: a key's `Ctrl+K` stays as typed.
        System.IO.File.WriteAllText(written, values.ToJsonString(new JsonSerializerOptions { WriteIndented = true, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping }));
        System.IO.File.Move(written, file, overwrite: true);
    }

    /// <summary>The settings <paramref name="program"/> kept, with its secrets (DESIGN 5.27): each one named that the
    /// system's protected store holds, beside the file's values.</summary>
    public static JsonObject Read(string program, IReadOnlyCollection<string> secrets)
    {
        var values = Read(program);
        foreach (var name in secrets)
        {
            values.Remove(name);
            if (Store is null && Secrets.Get(program, name) is { } secret)
                values[name] = secret;
        }
        return values;
    }

    /// <summary>Keeps <paramref name="values"/> as the settings of <paramref name="program"/>, those named in
    /// <paramref name="secrets"/> in the system's protected store and never in the file.</summary>
    public static void Keep(string program, JsonObject values, IReadOnlyCollection<string> secrets)
    {
        var plain = (JsonObject)values.DeepClone();
        foreach (var name in secrets)
        {
            if (!plain.TryGetPropertyValue(name, out var value))
                continue;
            plain.Remove(name);
            if (Store is null)
                Secrets.Set(program, name, value?.GetValueKind() == JsonValueKind.String ? value.GetValue<string>() : "");
        }
        Keep(program, plain);
    }

    /// <summary>Whether a kept value fits its setting.</summary>
    public static bool Fits(Setting setting, JsonNode? value)
    {
        if (value is not JsonValue v)
            return false;
        bool Within(decimal n) =>
            (setting.Min is null || n >= decimal.Parse(setting.Min, CultureInfo.InvariantCulture))
            && (setting.Max is null || n <= decimal.Parse(setting.Max, CultureInfo.InvariantCulture));
        var kind = v.GetValueKind();
        string? Text() => kind == JsonValueKind.String ? v.GetValue<string>() : null;
        return setting.Type switch
        {
            "integer" or "duration" => kind == JsonValueKind.Number && v.TryGetValue<long>(out var n) && Within(n),
            "decimal" or "money" => Text() is { } t && Number().IsMatch(t) && Within(decimal.Parse(t, CultureInfo.InvariantCulture)),
            "bool" => kind is JsonValueKind.True or JsonValueKind.False,
            "date" => Text() is { } t && Date().IsMatch(t),
            "datetime" => Text() is { } t && Moment().IsMatch(t),
            "enum" or "language" or "theme" => Text() is { } t && (setting.Values ?? []).Contains(t),
            "color" => Text() is { } t && KeptColor().IsMatch(t),
            // A reference is kept as the record's id.
            "ref" => kind == JsonValueKind.Number && v.TryGetValue<long>(out var id) && id > 0,
            _ => Text() is not null,
        };
    }

    /// <summary>The kept values a start takes back: only of settings there are, and only those that fit.</summary>
    public static JsonObject Taken(IReadOnlyDictionary<string, Setting> settings, JsonObject kept)
    {
        var taken = new JsonObject();
        foreach (var (name, setting) in settings)
            if (kept.TryGetPropertyValue(name, out var value) && Fits(setting, value))
                taken[name] = value!.DeepClone();
        return taken;
    }

    [GeneratedRegex(@"^-?\d+(\.\d+)?$")]
    private static partial Regex Number();

    [GeneratedRegex(@"^\d{4}-\d{2}-\d{2}$")]
    private static partial Regex Date();

    [GeneratedRegex(@"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?$")]
    private static partial Regex Moment();

    [GeneratedRegex("^(#[0-9a-f]{6})?$")]
    private static partial Regex KeptColor();
}
