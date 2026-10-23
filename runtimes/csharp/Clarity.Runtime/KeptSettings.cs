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

    /// <summary>Where <paramref name="program"/> keeps its settings on this machine.</summary>
    public static string? File(string program)
    {
        var os = OperatingSystem.IsWindows() ? "windows" : OperatingSystem.IsMacOS() ? "macos" : "linux";
        return FileIn(os, Environment.GetEnvironmentVariable, program);
    }

    /// <summary>The settings <paramref name="program"/> kept; none when it kept none, or they cannot be read.</summary>
    public static JsonObject Read(string program)
    {
        try
        {
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
        var file = File(program) ?? throw new InvalidOperationException("this system names no folder for application settings");
        Directory.CreateDirectory(Path.GetDirectoryName(file)!);
        var written = file + ".new";
        System.IO.File.WriteAllText(written, values.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        System.IO.File.Move(written, file, overwrite: true);
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
