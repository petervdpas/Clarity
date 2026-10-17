// What the person chose with the `language` and `theme` click targets, kept
// for the next start in a small file of the application's own, as the web
// renderer keeps it in local storage.

using System.Text.Json;

namespace Clarity.Avalonia;

public static class Remembered
{
    /// <summary>The folder of the choices file; none keeps choices for this run only.</summary>
    public static string? Folder { get; set; }

    /// <summary>Keeps choices in the person's application data, in a folder named after the application.</summary>
    public static void Use(string application) =>
        Folder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), application);

    static readonly Dictionary<string, string> forRun = [];

    static string? File => Folder is null ? null : Path.Combine(Folder, "choices.json");

    static Dictionary<string, string> Read()
    {
        if (File is not { } file)
            return new(forRun);
        try
        {
            if (System.IO.File.Exists(file))
                return JsonSerializer.Deserialize<Dictionary<string, string>>(System.IO.File.ReadAllText(file)) ?? [];
        }
        catch (Exception e) when (e is IOException or JsonException or UnauthorizedAccessException)
        {
            // A damaged or unreadable file: the application starts as designed.
        }
        return [];
    }

    public static string? Get(string key) => Read().GetValueOrDefault(key);

    public static void Set(string key, string value)
    {
        if (File is not { } file)
        {
            forRun[key] = value;
            return;
        }
        var all = Read();
        all[key] = value;
        try
        {
            Directory.CreateDirectory(Folder!);
            System.IO.File.WriteAllText(file, JsonSerializer.Serialize(all));
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            // The choice holds for this run.
        }
    }
}
