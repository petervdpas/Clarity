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
