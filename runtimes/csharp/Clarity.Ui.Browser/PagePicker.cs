// The file dialogs of a browser page (DESIGN 7.9): a file to open comes from
// the page's file input, its bytes brought into the program; a copy is saved
// as the browser's download; a file is shown in a tab of its own. A page
// cannot choose a folder of the computer: a folder is typed there.

using Clarity.Runtime;

namespace Clarity.Ui.Browser;

public sealed class PagePicker(IBrowserPage page, Func<string, Task<ChosenFile?>> open) : IFilePicker
{
    public bool PicksFolders => false;

    public Task<ChosenFile?> OpenFile(string title, FileFilter? filter = null) => open(Accept(filter));

    /// <summary>The download of the copy: where it went is the browser's choice, so its name is said.</summary>
    public Task<string?> SaveFile(string title, string name, byte[] bytes)
    {
        page.Download(name, bytes);
        return Task.FromResult<string?>(name);
    }

    public Task<string?> ChooseFolder(string title) => Task.FromResult<string?>(null);

    public void Show(string name, byte[] bytes) => page.Show(name, bytes);

    /// <summary>A filter as a file input's accept: <c>*.png</c> is <c>.png</c>; every file, or none, accepts anything.</summary>
    public static string Accept(FileFilter? filter) =>
        filter is null || filter.Patterns.Any(p => p is "*" or "*.*")
            ? ""
            : string.Join(',', filter.Patterns.Where(p => p.StartsWith("*.") && p.Length > 2).Select(p => p[1..]));

    /// <summary>
    /// The kind a shown file is given in its tab, by its extension: kinds a
    /// tab only shows (PDF, pictures, plain text); anything else, a web page
    /// or a drawing that could run script in the page's name included, is
    /// given as data the browser downloads.
    /// </summary>
    public static string TypeOf(string name) =>
        Path.GetExtension(name).ToLowerInvariant() switch
        {
            ".pdf" => "application/pdf",
            ".png" => "image/png",
            ".jpg" or ".jpeg" => "image/jpeg",
            ".gif" => "image/gif",
            ".webp" => "image/webp",
            ".bmp" => "image/bmp",
            ".txt" or ".csv" or ".json" or ".md" or ".log" => "text/plain",
            ".mp3" => "audio/mpeg",
            ".wav" => "audio/wav",
            ".mp4" => "video/mp4",
            _ => "application/octet-stream",
        };
}
