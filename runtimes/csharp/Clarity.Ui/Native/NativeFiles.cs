// What a generated application carries for the library to show it
// (ClarityFiles, the AppFiles of clarity-ui): its windows' and apps' markup,
// its resolved themes, its model as JSON and the files of its resources
// folder, each embedded in its assembly under the name the generated
// project file gives it (Windows/Main.cxaml, Apps/Shop.cxaml,
// Themes/night-owl.light.json, model.json, resources/logo.png), and the
// title and size its main window opens with.

using System.Reflection;
using System.Text;
using System.Text.Json.Nodes;

namespace Clarity.Ui.Native;

internal sealed unsafe class NativeFiles : IDisposable
{
    nint handle;

    public NativeFiles()
    {
        Interop.Check(Lib.FilesNew(out handle), "clarity_files_new");
    }

    public nint Handle => handle;

    /// <summary>The files an application's assembly carries, as the C# generator embeds them.</summary>
    public static NativeFiles Of(Assembly app, string? title, (double Width, double Height)? size)
    {
        var files = new NativeFiles();
        try
        {
            string? model = null;
            foreach (var name in app.GetManifestResourceNames())
            {
                if (Text(app, name, "Windows/", ".cxaml") is { } window)
                    files.Add(Lib.FilesWindow, window.Name, window.Text);
                else if (Text(app, name, "Apps/", ".cxaml") is { } frame)
                    files.Add(Lib.FilesApp, frame.Name, frame.Text);
                else if (Text(app, name, "Themes/", ".json") is { } theme)
                    files.Add(Lib.FilesTheme, theme.Name, theme.Text);
                else if (name == "model.json")
                    model = Read(app, name) is { } bytes ? Encoding.UTF8.GetString(bytes) : null;
                else if (name.StartsWith("resources/", StringComparison.Ordinal) && Read(app, name) is { } resource)
                    files.Resource(name["resources/".Length..], resource);
            }
            if (model is not null)
                files.Model(model);
            files.Title(title ?? TitleOf(model) ?? app.GetName().Name ?? "Clarity");
            if (size is { } s)
                Interop.Check(Lib.FilesSize(files.handle, s.Width, s.Height), "clarity_files_size");
            return files;
        }
        catch
        {
            files.Dispose();
            throw;
        }
    }

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

    static (string Name, string Text)? Text(Assembly app, string resource, string folder, string extension)
    {
        if (!resource.StartsWith(folder, StringComparison.Ordinal) || !resource.EndsWith(extension, StringComparison.Ordinal))
            return null;
        var bytes = Read(app, resource);
        return bytes is null ? null : (resource[folder.Length..^extension.Length], Encoding.UTF8.GetString(bytes));
    }

    static byte[]? Read(Assembly app, string resource)
    {
        using var stream = app.GetManifestResourceStream(resource);
        if (stream is null)
            return null;
        using var bytes = new MemoryStream();
        stream.CopyTo(bytes);
        return bytes.ToArray();
    }

    delegate Status Named(nint files, byte* name, nuint nameLen, byte* text, nuint textLen);

    void Add(Named add, string name, string text)
    {
        var n = Interop.Utf8(name);
        var t = Interop.Utf8(text);
        fixed (byte* np = n)
        fixed (byte* tp = t)
            Interop.Check(add(handle, np, (nuint)n.Length, tp, (nuint)t.Length), "clarity_files");
    }

    /// <summary>A window's markup by the window's name.</summary>
    public void Window(string name, string markup) => Add(Lib.FilesWindow, name, markup);

    /// <summary>The model the program is made with, as JSON.</summary>
    public void Model(string json)
    {
        var j = Interop.Utf8(json);
        fixed (byte* jp = j)
            Interop.Check(Lib.FilesModel(handle, jp, (nuint)j.Length), "clarity_files_model");
    }

    public void Resource(string name, byte[] bytes)
    {
        var n = Interop.Utf8(name);
        fixed (byte* np = n)
        fixed (byte* bp = bytes)
            Interop.Check(Lib.FilesResource(handle, np, (nuint)n.Length, bp, (nuint)bytes.Length), "clarity_files_resource");
    }

    public void Title(string title)
    {
        var t = Interop.Utf8(title);
        fixed (byte* tp = t)
            Interop.Check(Lib.FilesTitle(handle, tp, (nuint)t.Length), "clarity_files_title");
    }

    public void Dispose()
    {
        if (handle == 0)
            return;
        Lib.FilesFree(handle);
        handle = 0;
    }
}
