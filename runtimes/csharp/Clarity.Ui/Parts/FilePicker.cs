// The system's file dialogs as the parts use them: choose a file to open,
// choose where a copy of a file is saved, choose a folder, and open a file
// in the program the system uses for its kind. The desktop host opens the
// system's dialogs, the browser host the page's (a file input, a download).
// A part asks its window for them (IPartHost.Files); the window has the
// one its host made (AppScreen's files), or one that picks nothing. Tests
// give a picker of their own; no test opens a dialog.

namespace Clarity.Ui;

/// <summary>A file the person chose: its name as chosen and its bytes.</summary>
public sealed record ChosenFile(string Name, byte[] Bytes);

/// <summary>The kinds of file a dialog offers: their name in the person's language and their patterns (<c>*.png</c>).</summary>
public sealed record FileFilter(string Name, IReadOnlyList<string> Patterns);

/// <summary>The system's file dialogs, and the program that opens a file.</summary>
public interface IFilePicker
{
    /// <summary>Whether it can choose a folder; a browser cannot, and a folder is typed there.</summary>
    bool PicksFolders { get; }

    /// <summary>The file the person chose to open, titled <paramref name="title"/>, of the kinds of <paramref name="filter"/>; none when they chose none.</summary>
    Task<ChosenFile?> OpenFile(string title, FileFilter? filter = null);

    /// <summary>Saves <paramref name="bytes"/> where the person chose, <paramref name="name"/> suggested; where it went, or none when they chose nowhere.</summary>
    Task<string?> SaveFile(string title, string name, byte[] bytes);

    /// <summary>The folder the person chose, titled <paramref name="title"/>; none when they chose none.</summary>
    Task<string?> ChooseFolder(string title);

    /// <summary>Opens a file in the program the system uses for its kind; in a browser, in a tab of its own.</summary>
    void Show(string name, byte[] bytes);
}

/// <summary>The picker of a host that has none: it chooses nothing and opens nothing.</summary>
public sealed class NoFilePicker : IFilePicker
{
    public bool PicksFolders => false;

    public Task<ChosenFile?> OpenFile(string title, FileFilter? filter = null) => Task.FromResult<ChosenFile?>(null);

    public Task<string?> SaveFile(string title, string name, byte[] bytes) => Task.FromResult<string?>(null);

    public Task<string?> ChooseFolder(string title) => Task.FromResult<string?>(null);

    public void Show(string name, byte[] bytes) { }
}

/// <summary>How a part reads the stored file or picture of a record's field (<c>task.attachment</c>).</summary>
public static class RecordFetch
{
    /// <summary>The record a field is of; none for a path without a dot.</summary>
    public static object? Record(IPartHost host, string? bind) =>
        bind?.IndexOf('.') is { } dot and > 0 && dot < bind.Length - 1 ? host.Read(bind[..dot]) : null;

    /// <summary>The bytes of a field's file: new ones as they are, a stored one or one of the store by its path fetched through the window (IPartHost.Fetch); none for none.</summary>
    public static async Task<byte[]?> Bytes(IPartHost host, string? bind, Clarity.Runtime.Picture? content)
    {
        if (content is null || bind is null)
            return content?.Bytes;
        if (content.Bytes is { } bytes)
            return bytes;
        return await host.Fetch(bind, content.FilePath);
    }
}
