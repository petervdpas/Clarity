// A question a handler asks (DESIGN 5.14): asked when the handler has
// finished, in Clarity's own dialog over the window, never the system's; the
// input its form needs, OK and Cancel; the answer runs the handler it goes
// to, as any handler runs, and cancelling runs nothing.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

sealed class AlbumsViewModel() : WindowViewModel(null)
{
    public List<string> Log { get; } = [];

    public override string Title => "Albums";

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

    public void NameClicked() => Handle(() => Ask(Text.Plain("Name?"), new ValueForm(null, null, "Blue"), answer => Log.Add($"named {answer}")));
}

public class AskDialogTests
{
    static (Screen Screen, WindowStack Stack, AlbumsViewModel Model) Shown()
    {
        var stack = new WindowStack(_ => "<Window name=\"Albums\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 500, 1);
        var model = new AlbumsViewModel();
        stack.Push(model);
        Shot.Of(screen, 800, 500);
        return (screen, stack, model);
    }

    static AskDialog Asking(Screen screen) => Assert.IsType<AskDialog>(Assert.Single(screen.Overlays, o => o.Modal).Content);

    static void Click(Screen screen, Element element)
    {
        var (x, y) = (element.Bounds.X + element.Bounds.Width / 2, element.Bounds.Y + element.Bounds.Height / 2);
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    [Fact]
    public void A_question_shows_in_clarity_s_dialog_once_the_handler_has_finished()
    {
        var (screen, _, model) = Shown();
        model.DeleteClicked();
        Assert.Equal(["asked"], model.Log);
        var ask = Asking(screen);
        Assert.Null(ask.Input);
        Assert.Equal("Delete", ask.Ok.Text);
        Assert.Equal("Cancel", ask.Cancel.Text);
        Shot.Of(screen, 800, 500);
        Assert.True(ask.Panel.Width < 800 && ask.Panel.X > 0);

        // OK answers: the dialog goes and the handler the answer goes to runs.
        Click(screen, ask.Ok);
        Assert.Empty(screen.Overlays);
        Assert.Equal(["asked", "deleted"], model.Log);
    }

    [Fact]
    public void Cancel_or_escape_runs_nothing()
    {
        var (screen, stack, model) = Shown();
        model.DeleteClicked();
        Click(screen, Asking(screen).Cancel);
        Assert.Empty(screen.Overlays);
        model.DeleteClicked();
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(screen.Overlays);
        Assert.Equal(["asked", "asked"], model.Log);
        Assert.Same(model, stack.Current);
    }

    [Fact]
    public void A_value_is_typed_in_its_format_and_enter_answers()
    {
        var (screen, _, model) = Shown();
        model.CopiesClicked();
        var ask = Asking(screen);
        var input = Assert.IsType<NumberPart>(ask.Input);
        Assert.Equal(2L, ask.Value());
        // The input has the focus, so the person types at once.
        Assert.True(input.Find<TextEditor>()!.Focused);
        screen.KeyPressed(new KeyInput(Key.End, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Backspace, Modifiers.None));
        screen.TextTyped("3");
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Empty(screen.Overlays);
        Assert.Equal("copies 3", model.Log.Last());
    }

    [Fact]
    public void An_answer_the_handler_refuses_is_an_error_message()
    {
        var (screen, stack, model) = Shown();
        model.CopiesClicked();
        var ask = Asking(screen);
        ask.Answer(0L);
        Assert.Equal("At least one copy", stack.Messages.Last().Shown);
    }

    [Fact]
    public void Text_and_choices_get_their_inputs()
    {
        var text = new AskDialog(new Question(Text.Plain("Name?"), new ValueForm(null, null, "Blue"), _ => { }));
        Assert.IsType<TextBoxPart>(text.Input);
        Assert.Equal("Blue", text.Value());

        Choice[] friends = [new(3L, "Ada"), new(5L, "Grace")];
        var choice = new AskDialog(new Question(Text.Plain("Lend it to"), new ValueForm(null, friends, null), _ => { }));
        Assert.IsType<DropdownPart>(choice.Input);
        Assert.Equal(3L, choice.Value());
        var keep = new AskDialog(new Question(Text.Plain("Lend it to"), new ValueForm(null, friends, 5L), _ => { }));
        Assert.Equal(5L, keep.Value());

        var words = new AskDialog(new Question(Text.Plain("Sure?"), new ConfirmForm(Text.Plain("Yes"), Text.Plain("Keep")), _ => { }));
        Assert.Equal(("Yes", "Keep"), (words.Ok.Text, words.Cancel.Text));
    }

    [Fact]
    public void A_typed_text_is_the_answer()
    {
        var (screen, _, model) = Shown();
        model.NameClicked();
        var ask = Asking(screen);
        screen.KeyPressed(new KeyInput(Key.End, Modifiers.None));
        screen.TextTyped("s");
        Click(screen, ask.Ok);
        Assert.Equal("named Blues", model.Log.Last());
    }
}
