// The browser's side of a program served to browsers (DESIGN 7.6, 7.9): the
// mirror stack shows a mirror of each window the session keeps, drawn by
// Clarity.UI from the markup the server sends, sends what the person does,
// and follows what the server answers. Here it talks to the server in the
// same process, through JSON text as it would cross the wire.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using Clarity.Ui.Browser;
using Xunit;

namespace Clarity.Ui.Server.Tests;

/// <summary>A browser page that keeps what it was asked to do.</summary>
sealed class FakePage : IBrowserPage
{
    public List<string> Did { get; } = [];

    public void SetTitle(string title) => Did.Add($"title {title}");

    public void Download(string name, byte[] data) => Did.Add($"download {name}");

    public void Show(string name, byte[] data) => Did.Add($"show {name}");

    public string? Read(string key) => null;

    public void Write(string key, string value) { }

    public int Play(byte[] file, double volume, bool repeat) => 0;

    public void Stop(int sound) { }
}

[Collection("page")]
public class MirrorStackTests
{
    static async Task<(MirrorStack Mirror, InProcess Link)> Started()
    {
        var link = new InProcess(() => new ShelfViewModel(null));
        return (await MirrorStack.Start(link), link);
    }

    /// <summary>The mirror on a screen, laid out, as the page shows it.</summary>
    static Screen OnScreen(MirrorStack mirror)
    {
        var screen = new Screen(mirror, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 600, 1);
        return screen;
    }

    [Fact]
    public async Task It_shows_a_mirror_of_the_main_window_the_session_opened_drawn_from_the_servers_markup()
    {
        var (mirror, link) = await Started();
        var main = Assert.IsType<ShelfViewModel>(mirror.Current);
        Assert.True(main.IsMirror);
        Assert.Same(mirror, main.Remote);
        // Its on open ran on the server, once: the mirror took its state.
        Assert.Equal(["opened"], main.Log);
        Assert.Equal(["start"], link.Sent);
        var view = Assert.IsType<WindowView>(mirror.View);
        Assert.Contains(view.Parts, p => p.Spec.Name == "greetButton");
        Assert.Contains("<Button name=\"greetButton\"", mirror.Carried("Windows/Shelf.cxaml"));
    }

    [Fact]
    public async Task A_command_runs_on_the_server_with_what_the_person_typed()
    {
        var (mirror, link) = await Started();
        var main = (ShelfViewModel)mirror.Current!;
        main.Name.Text = "Ada";
        main.GreetCommand.Execute(null);
        await mirror.Idle;
        Assert.Equal("run", link.Sent[^1]);
        Assert.Equal("Hello Ada", Assert.Single(mirror.Messages).Text.Template);
    }

    [Fact]
    public async Task A_button_pressed_on_the_screen_sends_its_command()
    {
        var (mirror, link) = await Started();
        var screen = OnScreen(mirror);
        var button = mirror.View!.Parts.First(p => p.Spec.Name == "greetButton");
        var at = button.Bounds;
        screen.PointerPressed(at.X + at.Width / 2, at.Y + at.Height / 2, PointerButton.Left);
        screen.PointerReleased(at.X + at.Width / 2, at.Y + at.Height / 2, PointerButton.Left);
        await mirror.Idle;
        Assert.Equal(["Greet"], link.Commands);
    }

    [Fact]
    public async Task Windows_open_and_close_as_the_server_says()
    {
        var (mirror, _) = await Started();
        OnScreen(mirror);
        ((ShelfViewModel)mirror.Current!).DetailCommand.Execute(null);
        await mirror.Idle;
        var book = Assert.IsType<BookViewModel>(mirror.Current);
        Assert.Contains(mirror.View!.Parts, p => p.Spec.Name == "backButton");
        book.BackCommand.Execute(null);
        await mirror.Idle;
        var main = Assert.IsType<ShelfViewModel>(mirror.Current);
        Assert.Equal(["opened", "back"], main.Log);
        Assert.Contains(mirror.View!.Parts, p => p.Spec.Name == "greetButton");
    }

    [Fact]
    public async Task A_dialog_shows_over_the_window_and_gives_back_its_value()
    {
        var (mirror, link) = await Started();
        OnScreen(mirror);
        ((ShelfViewModel)mirror.Current!).PickCommand.Execute(null);
        await mirror.Idle;
        var pick = Assert.IsType<PickViewModel>(Assert.Single(mirror.Dialogs));
        var dialog = Assert.IsType<WindowView>(mirror.ViewOf(pick));
        Assert.IsType<DialogOverlay>(dialog.Parent);
        pick.ChooseCommand.Execute("blue");
        await mirror.Idle;
        Assert.Empty(mirror.Dialogs);
        Assert.Null(mirror.ViewOf(pick));
        Assert.Contains("picked blue", ((ShelfViewModel)mirror.Current!).Log);
        // Closed by the person, a dialog gives nothing back.
        ((ShelfViewModel)mirror.Current!).PickCommand.Execute(null);
        await mirror.Idle;
        await mirror.CloseDialog(mirror.Dialogs[0]);
        Assert.Equal("close", link.Sent[^1]);
        Assert.Empty(mirror.Dialogs);
    }

    [Fact]
    public async Task A_picture_or_a_file_is_fetched_from_the_server()
    {
        var (mirror, link) = await Started();
        ((ShelfViewModel)mirror.Current!).DetailCommand.Execute(null);
        await mirror.Idle;
        var book = Assert.IsType<BookViewModel>(mirror.Current);
        Assert.Equal([1, 2, 3], await book.CoverPicture());
        Assert.Equal("fetch", link.Sent[^1]);
        Assert.Equal([4], await book.CoverFile("a.png"));
        Assert.Null(await book.CoverFile("b.png"));
    }

    [Fact]
    public async Task A_question_is_asked_in_the_browser_and_answered_on_the_server()
    {
        var (mirror, _) = await Started();
        var screen = OnScreen(mirror);
        ((ShelfViewModel)mirror.Current!).RemoveCommand.Execute(null);
        await mirror.Idle;
        var question = Assert.IsType<Question>(mirror.Asking);
        Assert.Equal("Remove it?", question.Text.Template);
        var ask = Assert.IsType<AskDialog>(screen.Overlays[^1].Content);
        ask.Answered!(null);
        await mirror.Idle;
        Assert.Null(mirror.Asking);
        Assert.Contains("removed", ((ShelfViewModel)mirror.Current!).Log);
    }

    [Fact]
    public async Task What_a_handler_copied_goes_on_the_pages_clipboard()
    {
        var (mirror, _) = await Started();
        var screen = OnScreen(mirror);
        var main = (ShelfViewModel)mirror.Current!;
        main.Name.Text = "Ada";
        main.CopyNameCommand.Execute(null);
        await mirror.Idle;
        Assert.Equal("Name: Ada", screen.Clipboard.GetText());
    }

    [Fact]
    public async Task A_command_the_server_refuses_says_why_and_the_window_goes_on()
    {
        var (mirror, _) = await Started();
        mirror.Command(mirror.Current!, "Nothing", []);
        await mirror.Idle;
        var said = Assert.Single(mirror.Messages);
        Assert.Equal(Level.Error, said.Level);
        Assert.Contains("Nothing", said.Text.Template);
        Assert.IsType<ShelfViewModel>(mirror.Current);
    }

    /// <summary>
    /// A timer that ticks faster than the server answers sends no backlog:
    /// a tick that waits unsent takes the place of the one before it.
    /// </summary>
    [Fact]
    public async Task Ticks_waiting_for_a_slow_server_go_as_one()
    {
        var (mirror, link) = await Started();
        var main = (ShelfViewModel)mirror.Current!;
        link.Gate = new TaskCompletionSource();
        main.SteppedCommand.Execute(null);
        main.SteppedCommand.Execute(null);
        main.SteppedCommand.Execute(null);
        main.SteppedCommand.Execute(null);
        link.Gate.SetResult();
        await mirror.Idle;
        Assert.Equal(["Stepped", "Stepped"], link.Commands);
        Assert.Equal(2, main.Steps);
    }

    [Fact]
    public async Task What_the_window_does_as_it_opens_and_closes_stays_on_the_server()
    {
        var (mirror, link) = await Started();
        mirror.Command(mirror.Current!, nameof(WindowViewModel.Opened), []);
        mirror.Command(mirror.Current!, nameof(WindowViewModel.Returned), []);
        await mirror.Idle;
        Assert.Equal(["start"], link.Sent);
    }

    [Fact]
    public async Task The_tab_takes_the_title_of_the_window_on_top()
    {
        var page = new FakePage();
        BrowserPage.Current = page;
        try
        {
            var (mirror, _) = await Started();
            Assert.Equal("title Shelf", page.Did[^1]);
            ((ShelfViewModel)mirror.Current!).DetailCommand.Execute(null);
            await mirror.Idle;
            Assert.Equal("title Book", page.Did[^1]);
        }
        finally
        {
            BrowserPage.Current = null;
        }
    }

    [Fact]
    public async Task The_mirror_sends_the_persons_settings_and_takes_back_what_a_handler_changed()
    {
        var person = new JsonObject { ["speed"] = 3 };
        var sessions = new Sessions(() => new FasterViewModel(null)) { Settings = Prefs.Hooks };
        var mirror = await MirrorStack.Start(new InProcess(new WebServer(sessions, "", typeof(InProcess).Assembly)), (() => person, sent => person = sent));
        ((FasterViewModel)mirror.Current!).FasterCommand.Execute(null);
        await mirror.Idle;
        Assert.Equal(3, ((FasterViewModel)mirror.Current!).Seen);
        Assert.Equal(4, person["speed"]!.GetValue<int>());
    }
}
