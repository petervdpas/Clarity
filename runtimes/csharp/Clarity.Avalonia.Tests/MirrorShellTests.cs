// The browser's side of a server that shows Avalonia windows in browsers
// (DESIGN 7.6): the mirror shell shows a mirror of each window the session
// keeps, sends what the person does, and follows what the server answers.
// Here it talks to the sessions in the same process, through JSON text as
// it would cross the wire.

using System.Text.Json.Nodes;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// <summary>The sessions, reached as a browser reaches them: requests and replies as JSON text.</summary>
sealed class InProcess(Sessions sessions) : ISessionLink
{
    public List<string> Sent { get; } = [];

    public Task<JsonObject> Send(JsonObject request)
    {
        Sent.Add(request["do"]!.GetValue<string>());
        var reply = sessions.Answer(JsonNode.Parse(request.ToJsonString())!.AsObject());
        return Task.FromResult(JsonNode.Parse(reply.ToJsonString())!.AsObject());
    }
}

/// <summary>A browser page that keeps what it was asked to do.</summary>
sealed class FakePage : IBrowserPage
{
    public List<string> Did { get; } = [];
    public Dictionary<string, string> Stored { get; } = [];

    public void SetTitle(string title) => Did.Add($"title {title}");

    public void Download(string name, byte[] data) => Did.Add($"download {name} {data.Length}");

    public void Show(string name, byte[] data) => Did.Add($"show {name} {data.Length}");

    public string? Read(string key) => Stored.GetValueOrDefault(key);

    public void Write(string key, string value) => Stored[key] = value;

    public int Play(byte[] file, double volume, bool repeat)
    {
        Did.Add($"play {file.Length} {volume} {repeat}");
        return Did.Count;
    }

    public void Stop(int sound) => Did.Add($"stop {sound}");
}

public class MirrorShellTests
{
    static Control View(WindowViewModel model) => new TextBlock { Text = model.GetType().Name, DataContext = model };

    static async Task<(MirrorShell Shell, InProcess Link)> Started(Db db)
    {
        var link = new InProcess(new Sessions(() => new ShelfViewModel(db)));
        var shell = await MirrorShell.Start(link, View);
        return (shell, link);
    }

    [AvaloniaFact]
    public async Task It_shows_a_mirror_of_the_main_window_the_session_opened()
    {
        using var db = Db.Memory();
        var (shell, _) = await Started(db);
        var main = Assert.IsType<ShelfViewModel>(shell.Current);
        Assert.True(main.IsMirror);
        Assert.Equal(["opened"], main.Log);
        Assert.Equal("ShelfViewModel", Assert.IsType<TextBlock>(shell.Shown).Text);
    }

    [AvaloniaFact]
    public async Task A_command_runs_on_the_server_with_what_the_person_typed()
    {
        using var db = Db.Memory();
        var (shell, link) = await Started(db);
        var main = (ShelfViewModel)shell.Current!;
        main.Name.Text = "Ada";
        main.GreetCommand.Execute(null);
        await shell.Idle;
        Assert.Equal("run", link.Sent[^1]);
        Assert.Equal("Hello Ada", Assert.Single(shell.Messages).Text.Template);
    }

    [AvaloniaFact]
    public async Task Windows_open_and_close_as_the_server_says()
    {
        using var db = Db.Memory();
        var (shell, _) = await Started(db);
        ((ShelfViewModel)shell.Current!).DetailCommand.Execute(null);
        await shell.Idle;
        var book = Assert.IsType<BookViewModel>(shell.Current);
        Assert.Equal("BookViewModel", Assert.IsType<TextBlock>(shell.Shown).Text);
        book.BackCommand.Execute(null);
        await shell.Idle;
        var main = Assert.IsType<ShelfViewModel>(shell.Current);
        Assert.Equal(["opened", "back"], main.Log);
    }

    [AvaloniaFact]
    public async Task A_dialog_shows_over_the_window_and_gives_back_its_value()
    {
        using var db = Db.Memory();
        var (shell, link) = await Started(db);
        ((ShelfViewModel)shell.Current!).PickCommand.Execute(null);
        await shell.Idle;
        var pick = Assert.IsType<PickViewModel>(Assert.Single(shell.Dialogs));
        pick.ChooseCommand.Execute("blue");
        await shell.Idle;
        Assert.Empty(shell.Dialogs);
        Assert.Contains("picked blue", ((ShelfViewModel)shell.Current!).Log);
        // Closed by the person, a dialog gives nothing back.
        ((ShelfViewModel)shell.Current!).PickCommand.Execute(null);
        await shell.Idle;
        await shell.CloseDialog(shell.Dialogs[0]);
        Assert.Equal("close", link.Sent[^1]);
        Assert.Empty(shell.Dialogs);
    }

    [AvaloniaFact]
    public async Task A_picture_or_a_file_is_fetched_from_the_server()
    {
        using var db = Db.Memory();
        var (shell, link) = await Started(db);
        ((ShelfViewModel)shell.Current!).DetailCommand.Execute(null);
        await shell.Idle;
        var book = Assert.IsType<BookViewModel>(shell.Current);
        Assert.Equal([1, 2, 3], await book.CoverPicture());
        Assert.Equal("fetch", link.Sent[^1]);
        Assert.Equal([4], await book.CoverFile("a.png"));
        Assert.Null(await book.CoverFile("b.png"));
    }

    [AvaloniaFact]
    public async Task A_question_is_asked_in_the_browser_and_answered_on_the_server()
    {
        using var db = Db.Memory();
        var (shell, _) = await Started(db);
        ((ShelfViewModel)shell.Current!).RemoveCommand.Execute(null);
        await shell.Idle;
        var question = Assert.IsType<Question>(shell.Asking);
        Assert.Equal("Remove it?", question.Text.Template);
        question.Answered(null);
        await shell.Idle;
        Assert.Null(shell.Asking);
        Assert.Contains("removed", ((ShelfViewModel)shell.Current!).Log);
    }
}

public class BrowserPageTests
{
    [AvaloniaFact]
    public async Task The_tab_takes_the_title_of_the_window_on_top()
    {
        using var db = Db.Memory();
        var page = new FakePage();
        BrowserPage.Current = page;
        try
        {
            var shell = await MirrorShell.Start(new InProcess(new Sessions(() => new ShelfViewModel(db))), m => new TextBlock());
            Assert.Equal("title Shelf", page.Did[^1]);
            ((ShelfViewModel)shell.Current!).DetailCommand.Execute(null);
            await shell.Idle;
            Assert.Equal("title Book", page.Did[^1]);
        }
        finally
        {
            BrowserPage.Current = null;
        }
    }

    [AvaloniaFact]
    public void A_sound_plays_through_the_page()
    {
        var page = new FakePage();
        var output = new PageSoundOutput(page);
        var playing = output.Play([1, 2, 3], 0.5f, true);
        playing.Dispose();
        Assert.Equal(["play 3 0.5 True", "stop 1"], page.Did);
    }

    [AvaloniaFact]
    public void A_file_is_downloaded_or_shown_in_a_tab_in_the_browser()
    {
        var page = new FakePage();
        BrowserPage.Current = page;
        try
        {
            var input = new FileInput();
            input.Take("rules.pdf", [1, 2, 3]);
            string? said = null;
            input.CopySaved += where => said = where;
            input.SaveCopy();
            input.OpenIt();
            Assert.Equal(["download rules.pdf 3", "show rules.pdf 3"], page.Did);
            Assert.Equal("rules.pdf", said);
        }
        finally
        {
            BrowserPage.Current = null;
        }
    }
}

[Collection("settings")]
public class MirrorSettingsTests
{
    [AvaloniaFact]
    public async Task The_mirror_sends_the_persons_settings_and_takes_back_what_a_handler_changed()
    {
        using var db = Db.Memory();
        var person = new JsonObject { ["speed"] = 3 };
        var sessions = new Sessions(() => new FasterViewModel(db)) { Settings = Prefs.Hooks };
        var shell = await MirrorShell.Start(new InProcess(sessions), m => new TextBlock(), settings: (() => person, sent => person = sent));
        ((FasterViewModel)shell.Current!).FasterCommand.Execute(null);
        await shell.Idle;
        Assert.Equal(3, ((FasterViewModel)shell.Current!).Seen);
        Assert.Equal(4, person["speed"]!.GetValue<int>());
    }
}
