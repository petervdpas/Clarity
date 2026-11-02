// The file part (U1c): a record's file name or that it has none, a chosen
// file and a removed one as changes, the bytes of a stored file fetched only
// to open it or save a copy, and its buttons by what may be done; with the
// names a chosen file is kept under (conformance/record-files.json). A
// picker of the test's own stands for the system's dialogs.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

/// <summary>The system's dialogs as a test gives them: what it chooses next, and what it was asked.</summary>
sealed class FilePartPicker : IFilePicker
{
    public ChosenFile? NextFile;
    public string? NextFolder;
    public string? SavedTo = "/home/ada/Downloads/copy";
    public bool Folders = true;
    public readonly List<string> Asked = [];
    public readonly List<(string Name, byte[] Bytes)> Shown = [];
    public readonly List<(string Name, byte[] Bytes)> Saved = [];
    public FileFilter? Filter;

    public bool PicksFolders => Folders;

    public Task<ChosenFile?> OpenFile(string title, FileFilter? filter = null)
    {
        Asked.Add($"open {title}");
        Filter = filter;
        return Task.FromResult(NextFile);
    }

    public Task<string?> SaveFile(string title, string name, byte[] bytes)
    {
        Asked.Add($"save {title} {name}");
        Saved.Add((name, bytes));
        return Task.FromResult(SavedTo);
    }

    public Task<string?> ChooseFolder(string title)
    {
        Asked.Add($"folder {title}");
        return Task.FromResult(NextFolder);
    }

    public void Show(string name, byte[] bytes) => Shown.Add((name, bytes));
}

/// <summary>A host with the test's picker, and the change handlers it ran.</summary>
class FilePartHost : TestHost
{
    public readonly FilePartPicker Picker = new();

    public FilePartHost() => Files = Picker;
}

public class FilePartTests
{
    static readonly JsonElement RecordFilesCases = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "record-files.json"))).RootElement;

    const string Markup = """<File at="1,1" span="12" name="doc" bind="task.attachment" onchange="attached" />""";

    static (Screen Screen, FilePart Part, FilePartHost Host) Make(RecordFile? file = null, string markup = Markup)
    {
        var host = new FilePartHost();
        host.Values["task"] = new object();
        host.Values["task.attachment"] = file;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<FilePart>(parts[0]), host);
    }

    static void Click(Screen screen, Element button)
    {
        var (x, y) = (button.Bounds.X + button.Bounds.Width / 2, button.Bounds.Y + button.Bounds.Height / 2);
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    [Fact]
    public void It_shows_the_files_name_or_that_there_is_none()
    {
        var (_, part, host) = Make();
        Assert.Equal(Locale.Translate("noFile", "No file"), part.Shown);
        Assert.True(part.IsEmpty);
        host.Values["task.attachment"] = new RecordFile("manual.pdf", Picture.Stored);
        host.Change();
        Assert.Equal("manual.pdf", part.Shown);
        Assert.False(part.IsEmpty);
    }

    [Fact]
    public void Open_and_save_show_only_with_a_file_and_remove_is_usable_only_then()
    {
        var (_, part, host) = Make();
        Assert.False(part.OpenButton.IsShown);
        Assert.False(part.SaveButton.IsShown);
        Assert.True(part.ChooseButton.IsShown);
        Assert.False(part.RemoveButton.Focusable);
        host.Values["task.attachment"] = new RecordFile("manual.pdf", Picture.Stored);
        host.Change();
        Assert.True(part.OpenButton.IsShown);
        Assert.True(part.SaveButton.IsShown);
        Assert.True(part.RemoveButton.Focusable);
        // Clarity's own words name them.
        Assert.Equal(Locale.Translate("chooseFile", "Choose a file"), part.ChooseButton.Text);
        Assert.Equal(Locale.Translate("openFile", "Open"), part.OpenButton.Text);
        Assert.Equal(Locale.Translate("saveFileCopy", "Save a copy"), part.SaveButton.Text);
        Assert.Equal(Locale.Translate("removeFile", "Remove the file"), part.RemoveButton.Text);
    }

    [Fact]
    public void Choosing_and_removing_are_changes()
    {
        var (screen, part, host) = Make();
        host.Picker.NextFile = new ChosenFile("C:\\docs\\manual.pdf", [1, 2, 3]);
        Click(screen, part.ChooseButton);
        var file = Assert.IsType<RecordFile>(host.Values["task.attachment"]);
        Assert.Equal("manual.pdf", file.Name);
        Assert.Equal(Picture.New([1, 2, 3]), file.Content);
        Assert.Equal(["write doc task.attachment " + file, "run doc change attached"], host.Log);
        host.Log.Clear();
        Click(screen, part.RemoveButton);
        Assert.Null(host.Values["task.attachment"]);
        Assert.Equal(["write doc task.attachment ", "run doc change attached"], host.Log);
        host.Log.Clear();
        // Removing when there is none is no change, nor is a choice the person left.
        part.Remove();
        host.Picker.NextFile = null;
        Click(screen, part.ChooseButton);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_read_only_file_part_neither_chooses_nor_removes()
    {
        var (_, part, host) = Make(new RecordFile("manual.pdf", Picture.Stored), """<File at="1,1" span="12" name="doc" bind="task.attachment" readonly="true" />""");
        Assert.False(part.ChooseButton.IsShown);
        Assert.False(part.RemoveButton.IsShown);
        Assert.True(part.OpenButton.IsShown);
        part.Take("other.pdf", [4]);
        part.Remove();
        Assert.Empty(host.Log);
        // Code makes one read-only through its state.
        var (_, other, _) = Make();
        Assert.True(other.ChooseButton.IsShown);
        other.State!.IsReadOnly = true;
        Assert.False(other.ChooseButton.IsShown);
    }

    [Fact]
    public void A_chosen_file_is_kept_under_the_last_part_of_its_name()
    {
        var run = 0;
        foreach (var c in RecordFilesCases.GetProperty("storePaths").EnumerateArray())
        {
            var (_, part, host) = Make();
            var chosen = c.GetProperty("chosen").GetString()!;
            var bytes = System.Text.Encoding.UTF8.GetBytes(c.GetProperty("text").GetString()!);
            part.Take(chosen, bytes);
            var file = Assert.IsType<RecordFile>(host.Values["task.attachment"]);
            Assert.Equal(RecordFiles.NameOf(c.GetProperty("path").GetString()!), file.Name);
            run++;
        }
        Assert.True(run >= 6);
    }

    [Fact]
    public async Task A_stored_file_is_fetched_only_when_asked_for()
    {
        var fetched = 0;
        var (screen, part, host) = Make(new RecordFile("manual.pdf", Picture.Stored));
        host.Fetcher = (bind, path) =>
        {
            Assert.Equal("task.attachment", bind);
            if (path is not null)
                return Task.FromResult<byte[]?>(path == "ab/c.pdf" ? [7] : null);
            fetched++;
            return Task.FromResult<byte[]?>([9]);
        };
        Shot.Of(screen, 800, 60);
        Assert.Equal(0, fetched);
        Assert.Equal([9], await part.Bytes());
        Assert.Equal(1, fetched);
        host.Values["task.attachment"] = new RecordFile("c.pdf", Picture.InStore("ab/c.pdf"));
        host.Change();
        Assert.Equal([7], await part.Bytes());
        host.Values["task.attachment"] = new RecordFile("new.pdf", Picture.New([5]));
        host.Change();
        Assert.Equal([5], await part.Bytes());
    }

    [Fact]
    public void Opening_and_saving_a_copy_are_no_changes()
    {
        var (screen, part, host) = Make(new RecordFile("manual.pdf", Picture.Stored));
        host.Fetcher = (_, _) => Task.FromResult<byte[]?>([9]);
        string? saved = null;
        part.CopySaved += place => saved = place;
        Click(screen, part.OpenButton);
        Assert.Equal("manual.pdf", Assert.Single(host.Picker.Shown).Name);
        Assert.Equal([9], host.Picker.Shown[0].Bytes);
        Click(screen, part.SaveButton);
        Assert.Equal("manual.pdf", Assert.Single(host.Picker.Saved).Name);
        Assert.Equal("/home/ada/Downloads/copy", saved);
        // The person hears where it really went, among the window's messages.
        Assert.Equal([$"Info {Locale.Translate("fileCopySavedAt", "A copy is saved as {place}").Replace("{place}", "/home/ada/Downloads/copy")}"], host.Said);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_saved_copy_says_where_it_went_the_chosen_place_or_the_Downloads_folder()
    {
        var (screen, part, host) = Make(new RecordFile("manual.pdf", Picture.New([1])));
        host.Picker.SavedTo = "/srv/elsewhere/manual.pdf";
        Click(screen, part.SaveButton);
        Assert.Contains("/srv/elsewhere/manual.pdf", Assert.Single(host.Said));
        // A browser's download has only the name: it is in the Downloads folder.
        Assert.Equal(Locale.Translate("fileCopySaved", "A copy is in your Downloads folder"), FilePart.SavedWhere("manual.pdf"));
    }

    [Fact]
    public void Its_buttons_read_choose_open_save_a_copy_remove_and_a_long_name_ends_in_an_ellipsis()
    {
        var (screen, part, _) = Make(new RecordFile(new string('n', 300) + ".pdf", Picture.New([1])));
        var order = new[] { part.ChooseButton, part.OpenButton, part.SaveButton, part.RemoveButton };
        for (var i = 1; i < order.Length; i++)
            Assert.True(order[i - 1].Bounds.Right <= order[i].Bounds.X, $"button {i} stands after the one before");
        Assert.Equal([part.ChooseButton, part.OpenButton, part.SaveButton, part.RemoveButton], screen.TabOrder().Where(order.Contains).ToList());
        // The name the part draws is cut off inside its place, its end shown as dots.
        using var shot = Shot.Of(screen, 800, 60);
        var drawn = part.DrawnName;
        Assert.EndsWith("\u2026", drawn);
        Assert.True(drawn.Length < part.Shown.Length);
        Assert.StartsWith(drawn[..^1], part.Shown);
        Assert.True(screen.Fonts.Measure(drawn, "text", screen.Look.Size("text"), false) <= part.NameBox.Width - 2 * screen.Look.Padding);
    }

    [Fact]
    public void A_fetch_that_fails_opens_nothing()
    {
        var (_, part, host) = Make(new RecordFile("manual.pdf", Picture.Stored));
        host.Values["TaskAttachmentPicture"] = (Func<Task<byte[]?>>)(() => Task.FromException<byte[]?>(new IOException("gone")));
        part.OpenButton.Press();
        Assert.Empty(host.Picker.Shown);
    }

    [Fact]
    public void It_draws_with_and_without_a_file()
    {
        var (screen, _, host) = Make();
        using (Shot.Of(screen, 800, 60)) { }
        host.Values["task.attachment"] = new RecordFile("manual.pdf", Picture.Stored);
        host.Change();
        using var shot = Shot.Of(screen, 800, 60);
        Assert.Equal(800, shot.Width);
    }
}
