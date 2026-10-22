// A question a handler asks (DESIGN 5.14): asked when the handler has
// finished, in Clarity's own dialog; the answer runs the handler it goes to,
// as any handler runs, and cancelling runs nothing.

using Avalonia.Headless.XUnit;
using Avalonia.LogicalTree;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

sealed class AlbumsViewModel(Db db) : WindowViewModel(db)
{
    public List<string> Log { get; } = [];

    public void DeleteClicked() => Handle(() =>
    {
        Ask(Text.Plain("Delete Blue?"), new ConfirmForm(Text.Plain("Delete"), null), _ => Log.Add("deleted"));
        Log.Add("asked");
    });

    public void CopiesClicked() => Handle(() =>
        Ask(Text.Plain("How many?"), new ValueForm(new FieldFormat(FormatKind.Integer), null, 2L), answer =>
        {
            if ((long)answer! < 1)
                throw new RefusedException(Text.Plain("At least one copy"));
            Log.Add($"copies {answer}");
        }));
}

public class AskTests
{
    [AvaloniaFact]
    public void A_question_is_asked_when_the_handler_has_finished_and_its_answer_runs_as_a_handler()
    {
        using var db = Db.Memory();
        var albums = new AlbumsViewModel(db);
        var asked = new List<Question>();
        albums.AskRequested += q =>
        {
            albums.Log.Add("dialog");
            asked.Add(q);
        };
        albums.DeleteClicked();
        Assert.Equal(["asked", "dialog"], albums.Log);
        albums.Answer(asked[0], null);
        Assert.Equal(["asked", "dialog", "deleted"], albums.Log);

        // An answer the handler refuses is an error message, as in any handler.
        albums.CopiesClicked();
        albums.Answer(asked[1], 0L);
        Assert.Equal("At least one copy", albums.Messages.Last().Shown);
        albums.Answer(asked[1], 3L);
        Assert.Equal("copies 3", albums.Log.Last());
    }

    [AvaloniaFact]
    public void The_dialog_holds_the_input_its_form_needs_and_says_the_answer()
    {
        var yes = new AskDialog(new Question(Text.Plain("Delete Blue?"), new ConfirmForm(Text.Plain("Delete"), Text.Plain("Keep")), _ => { }));
        Assert.Null(yes.Input);
        Assert.Equal("Delete", yes.Ok.Content);
        Assert.Equal("Keep", yes.Cancel.Content);
        object? said = "nothing";
        yes.Answered += a => said = a;
        yes.Ok.RaiseEvent(new global::Avalonia.Interactivity.RoutedEventArgs(global::Avalonia.Controls.Button.ClickEvent));
        Assert.Null(said);

        var number = new AskDialog(new Question(Text.Plain("How many?"), new ValueForm(new FieldFormat(FormatKind.Integer), null, 2L), _ => { }));
        Assert.IsType<NumberInput>(number.Input);
        Assert.Equal(2L, number.Value());

        var text = new AskDialog(new Question(Text.Plain("Name?"), new ValueForm(null, null, "Blue"), _ => { }));
        Assert.Equal("Blue", text.Value());

        Choice[] friends = [new(3L, "Ada"), new(5L, "Grace")];
        var choice = new AskDialog(new Question(Text.Plain("Lend it to"), new ValueForm(null, friends, null), _ => { }));
        Assert.IsType<ChoiceInput>(choice.Input);
        Assert.Equal(3L, choice.Value());
        var cancelled = false;
        choice.Cancelled += () => cancelled = true;
        choice.Cancel.RaiseEvent(new global::Avalonia.Interactivity.RoutedEventArgs(global::Avalonia.Controls.Button.ClickEvent));
        Assert.True(cancelled);
    }
}

public class MessageTests
{
    [AvaloniaFact]
    public void A_note_has_a_small_x_that_closes_it_and_a_warning_is_not_a_note()
    {
        var messages = new System.Collections.ObjectModel.ObservableCollection<Message>
        {
            new(Level.Info, Text.Plain("Saved")),
            new(Level.Warning, Text.Plain("Careful")),
        };
        var list = new MessageList { Messages = messages };
        var note = Assert.Single(list.Children);
        Assert.True(note.Classes.Contains("message"));
        var close = note.GetLogicalDescendants().OfType<global::Avalonia.Controls.Button>().Single();
        Assert.Equal("×", close.Content);
        close.RaiseEvent(new global::Avalonia.Interactivity.RoutedEventArgs(global::Avalonia.Controls.Button.ClickEvent));
        Assert.Equal(["Careful"], messages.Select(m => m.Shown));
    }

    [AvaloniaFact]
    public void A_warning_is_said_in_a_dialog_closed_with_ok()
    {
        var said = new SaidDialog(new Message(Level.Error, Text.Plain("Not saved")));
        var done = false;
        said.Done += () => done = true;
        said.Ok.RaiseEvent(new global::Avalonia.Interactivity.RoutedEventArgs(global::Avalonia.Controls.Button.ClickEvent));
        Assert.True(done);
    }
}
