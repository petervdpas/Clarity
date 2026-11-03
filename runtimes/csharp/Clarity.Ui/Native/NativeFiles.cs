// What a generated application carries for the library to show it
// (ClarityFiles; the runtime's AppFiles reads them from its assembly), and
// the size its main window opens with.

using System.Reflection;
using Clarity.Runtime;

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
            var carried = AppFiles.Of(app, title);
            foreach (var (name, markup) in carried.Windows)
                files.Add(Lib.FilesWindow, name, markup);
            foreach (var (name, markup) in carried.Apps)
                files.Add(Lib.FilesApp, name, markup);
            foreach (var (name, theme) in carried.Themes)
                files.Add(Lib.FilesTheme, name, theme);
            foreach (var name in carried.Resources)
                if (carried.Resource(name) is { } resource)
                    files.Resource(name, resource);
            if (carried.Model is { } model)
                files.Model(model);
            files.Title(carried.Title);
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
