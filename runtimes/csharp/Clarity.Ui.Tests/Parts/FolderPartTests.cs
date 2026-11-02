// The folder part (X2): a folder setting's path, typed (taken on leaving or
// Enter, only a differing text a change) or chosen with the system's folder
// picker through its button, named in Clarity's words; read-only, and where
// nothing picks a folder, there is no button.

using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class FolderPartTests
{
    const string Markup = """<Folder at="1,1" span="12" name="export" bind="settings.exportFolder" onchange="moved" onenter="came" onleave="went" />""";

    static (Screen Screen, FolderPart Part, FilePartHost Host) Make(string markup = Markup, string? folder = "/mnt/photos")
    {
        var host = new FilePartHost();
        host.Values["settings.exportFolder"] = folder;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<FolderPart>(parts[0]), host);
    }

    static void Type(Screen screen, TextEditor editor, string text)
    {
        screen.Focus(editor);
        editor.SelectAll();
        screen.KeyPressed(new KeyInput(Key.Backspace, Modifiers.None));
        screen.TextTyped(text);
    }

    [Fact]
    public void A_typed_folder_is_taken_on_leaving_or_Enter_when_it_differs()
    {
        var (screen, part, host) = Make();
        Assert.Equal("/mnt/photos", part.Editor.Text);
        Type(screen, part.Editor, "/mnt/photos");
        screen.Focus(null);
        Assert.Equal(["run export enter came", "run export leave went"], host.Log);
        host.Log.Clear();
        Type(screen, part.Editor, "/srv/export");
        // Typing alone writes nothing: only the focus coming in is told.
        Assert.Equal(["run export enter came"], host.Log);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal("/srv/export", host.Values["settings.exportFolder"]);
        Assert.Equal(["run export enter came", "write export settings.exportFolder /srv/export", "run export change moved"], host.Log);
        host.Log.Clear();
        screen.Focus(null);
        Assert.Equal(["run export leave went"], host.Log);
    }

    [Fact]
    public void A_chosen_folder_is_taken_at_once()
    {
        var (screen, part, host) = Make();
        host.Picker.NextFolder = "/home/ada/Pictures";
        var button = part.ChooseButton;
        screen.PointerPressed(button.Bounds.X + button.Bounds.Width / 2, button.Bounds.Y + button.Bounds.Height / 2, PointerButton.Left);
        screen.PointerReleased(button.Bounds.X + button.Bounds.Width / 2, button.Bounds.Y + button.Bounds.Height / 2, PointerButton.Left);
        Assert.Equal("/home/ada/Pictures", host.Values["settings.exportFolder"]);
        Assert.Equal("/home/ada/Pictures", part.Editor.Text);
        Assert.Equal($"folder {Locale.Translate("chooseFolder", "Choose a folder")}", Assert.Single(host.Picker.Asked));
        Assert.Equal(["write export settings.exportFolder /home/ada/Pictures", "run export change moved"], host.Log);
        // The same folder again is no change.
        part.Chosen("/home/ada/Pictures");
        Assert.Equal(2, host.Log.Count);
        // Choosing none leaves it.
        host.Picker.NextFolder = null;
        button.Press();
        Assert.Equal(2, host.Log.Count);
    }

    [Fact]
    public void Its_button_is_named_in_Clarity_words()
    {
        var (_, part, _) = Make();
        Assert.Equal(Locale.Translate("chooseFolder", "Choose a folder"), part.ChooseName);
        Assert.Equal("...", part.ChooseButton.Text);
    }

    [Fact]
    public void A_read_only_folder_takes_no_chosen_folder_and_has_no_button()
    {
        var (_, part, host) = Make("""<Folder at="1,1" span="12" name="export" bind="settings.exportFolder" readonly="true" />""");
        Assert.False(part.ChooseButton.IsShown);
        part.Chosen("/elsewhere");
        Assert.Equal("/mnt/photos", host.Values["settings.exportFolder"]);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void Where_nothing_picks_a_folder_it_is_only_typed()
    {
        var host = new FilePartHost();
        host.Picker.Folders = false;
        var (_, _, parts) = Made.Window(Markup, host);
        Assert.False(Assert.IsType<FolderPart>(parts[0]).ChooseButton.IsShown);
    }

    [Fact]
    public void An_unbound_folder_keeps_its_text_in_its_state()
    {
        var (screen, part, host) = Make("""<Folder at="1,1" span="12" name="export" />""");
        Type(screen, part.Editor, "/tmp/out");
        screen.Focus(null);
        Assert.Equal("/tmp/out", host.State("export")!.Text);
        Assert.Equal(["export"], host.Edits);
        host.State("export")!.Text = "/var/x";
        host.Change();
        Assert.Equal("/var/x", part.Value);
    }

    [Fact]
    public void A_value_set_from_elsewhere_shows()
    {
        var (_, part, host) = Make();
        host.Values["settings.exportFolder"] = "/data";
        host.Change();
        Assert.Equal("/data", part.Editor.Text);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void It_draws()
    {
        var (screen, _, _) = Make();
        using var shot = Shot.Of(screen, 800, 60);
        Assert.Equal(60, shot.Height);
    }
}
