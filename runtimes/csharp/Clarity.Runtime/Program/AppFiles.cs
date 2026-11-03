// What a generated application carries for Clarity.UI to show it (the
// AppFiles of clarity-ui): its windows' and apps' markup, its resolved
// themes, its model as JSON and the files of its resources folder, each
// embedded in its assembly under the name the generated project file gives
// it (Windows/Main.cxaml, Apps/Shop.cxaml, Themes/night-owl.light.json,
// model.json, resources/logo.png), and its title. The desktop hands them to
// the native library; a web server serves them to the library in the
// browser (files.json).

using System.Reflection;
using System.Text;
using System.Text.Json.Nodes;

namespace Clarity.Runtime;

internal sealed class AppFiles
{
    readonly Assembly app;

    AppFiles(Assembly app) => this.app = app;

    /// <summary>Each window's markup by the window's name.</summary>
    public SortedDictionary<string, string> Windows { get; } = new(StringComparer.Ordinal);

    /// <summary>Each app frame's markup by the app's name.</summary>
    public SortedDictionary<string, string> Apps { get; } = new(StringComparer.Ordinal);

    /// <summary>Each theme's JSON by its name and mode (<c>night-owl.light</c>).</summary>
    public SortedDictionary<string, string> Themes { get; } = new(StringComparer.Ordinal);

    /// <summary>The model as JSON; none when the application carries none.</summary>
    public string? Model { get; private set; }

    /// <summary>The names of the resources folder's files (<c>logo.png</c>).</summary>
    public List<string> Resources { get; } = [];

    /// <summary>The title its main window opens with.</summary>
    public string Title { get; private set; } = "";

    const string ResourceFolder = "resources/";

    /// <summary>The files <paramref name="app"/> carries, titled <paramref name="title"/>, else as its model says.</summary>
    public static AppFiles Of(Assembly app, string? title)
    {
        var files = new AppFiles(app);
        foreach (var name in app.GetManifestResourceNames())
        {
            if (files.Named(name, "Windows/", ".cxaml") is { } window)
                files.Windows[window.Name] = window.Text;
            else if (files.Named(name, "Apps/", ".cxaml") is { } frame)
                files.Apps[frame.Name] = frame.Text;
            else if (files.Named(name, "Themes/", ".json") is { } theme)
                files.Themes[theme.Name] = theme.Text;
            else if (name == "model.json")
                files.Model = files.Read(name) is { } bytes ? Encoding.UTF8.GetString(bytes) : null;
            else if (name.StartsWith(ResourceFolder, StringComparison.Ordinal))
                files.Resources.Add(name[ResourceFolder.Length..]);
        }
        files.Resources.Sort(StringComparer.Ordinal);
        files.Title = string.IsNullOrEmpty(title) ? TitleOf(files.Model) ?? app.GetName().Name ?? "Clarity" : title;
        return files;
    }

    /// <summary>The bytes of the resources folder's file <paramref name="name"/>; none when there is no such file.</summary>
    public byte[]? Resource(string name) => Read(ResourceFolder + name);

    /// <summary>The project's title as its model writes it; none when it has none.</summary>
    static string? TitleOf(string? model)
    {
        try
        {
            return model is null ? null : JsonNode.Parse(model)?["project"]?["title"]?.GetValue<string>();
        }
        catch (Exception e) when (e is System.Text.Json.JsonException or InvalidOperationException)
        {
            return null;
        }
    }

    (string Name, string Text)? Named(string resource, string folder, string extension)
    {
        if (!resource.StartsWith(folder, StringComparison.Ordinal) || !resource.EndsWith(extension, StringComparison.Ordinal))
            return null;
        var bytes = Read(resource);
        return bytes is null ? null : (resource[folder.Length..^extension.Length], Encoding.UTF8.GetString(bytes));
    }

    /// <summary>The bytes of the embedded file <paramref name="resource"/>; none when there is no such file.</summary>
    public byte[]? Read(string resource)
    {
        using var stream = app.GetManifestResourceStream(resource);
        if (stream is null)
            return null;
        using var bytes = new MemoryStream();
        stream.CopyTo(bytes);
        return bytes.ToArray();
    }
}
