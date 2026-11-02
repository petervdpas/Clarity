// A window's life as the desktop host leans on it: a yes or no question
// focuses OK; the stack ends when its last window closes, not before; the
// system's close closes the dialogs over the window first, each hearing its
// on close (the top window and its opener: CheckFixHostsTests); and a stack
// that is closed (or off its screen) no longer hears language changes.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

/// <summary>A window that counts its on close and its returns, and opens a dialog over itself.</summary>
sealed class CountingViewModel() : WindowViewModel(null)
{
    public override string Title => "Counting";

    public int ClosingTimes;
    public int ReturnedTimes;
    public PickFriendViewModel? Asked;

    protected override void OnClose() => ClosingTimes++;

    protected override void OnReturn() => ReturnedTimes++;

    public void LendClicked() => Handle(() =>
    {
        Asked = new PickFriendViewModel();
        Ask(Asked, _ => { });
    });
}

public class CheckFixWindowTests
{
    const string Dialog = """
        <Window name="PickFriend">
          <Label name="who" at="1,1" span="12" caption="Who?" />
          <Button name="adaButton" at="1,2" span="6" caption="Ada" onclick="close" />
        </Window>
        """;

    static string MarkupOf(WindowViewModel model) => model is PickFriendViewModel ? Dialog : "<Window name=\"Counting\" />";

    [Fact]
    public void A_yes_or_no_question_focuses_OK_and_Enter_answers()
    {
        var stack = new WindowStack(_ => "<Window name=\"Albums\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 500, 1);
        var model = new AlbumsViewModel();
        stack.Push(model);
        model.DeleteClicked();
        var ask = Assert.IsType<AskDialog>(Assert.Single(screen.Overlays, o => o.Modal).Content);
        Assert.Null(ask.Input);
        Assert.Same(ask.Ok, ask.First);
        Assert.Same(ask.Ok, screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal(["asked", "deleted"], model.Log);
    }

    [Fact]
    public void The_stack_ends_when_its_last_window_closes_and_not_before()
    {
        var stack = new WindowStack(MarkupOf);
        _ = new Screen(stack, ThemeLook.Clarity(dark: false));
        var ended = 0;
        stack.Ended += () => ended++;
        var below = new CountingViewModel();
        var top = new CountingViewModel();
        stack.Push(below);
        stack.Push(top);
        stack.Close(top);
        Assert.Equal(0, ended);
        Assert.Same(below, stack.Current);
        Assert.Equal(1, below.ReturnedTimes);
        stack.Close(below);
        Assert.Equal(1, ended);
        Assert.Equal(1, top.ClosingTimes);
        Assert.Equal(1, below.ClosingTimes);
    }

    [Fact]
    public void The_systems_close_closes_the_dialogs_over_the_window_first_each_hearing_on_close()
    {
        var opener = new CountingViewModel();
        var stack = new WindowStack(MarkupOf) { Opener = opener };
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 500, 1);
        var below = new CountingViewModel();
        var top = new CountingViewModel();
        stack.Push(below);
        stack.Push(top);
        top.LendClicked();
        Assert.Single(screen.Overlays, o => o.Modal);

        stack.Closing();

        Assert.Equal(1, top.Asked!.ClosingTimes);
        Assert.DoesNotContain(screen.Overlays, o => o.Modal);
        Assert.Equal(1, top.ClosingTimes);
        Assert.Equal(1, opener.ReturnedTimes);
    }

    [Collection("Language")]
    public class Language
    {
        static string? Lookup(string key, CultureInfo culture) => (key, culture.TwoLetterISOLanguageName) switch
        {
            ("hello", "nl") => "Hallo",
            ("hello", _) => "Hello",
            _ => null,
        };

        const string Greet = """<Window name="Greet"><Label name="hi" at="1,1" span="6" caption="@hello" /></Window>""";

        [Fact]
        public void A_closed_window_no_longer_hears_language_changes()
        {
            Locale.Start("en", ["en", "nl"], null, Lookup, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
            try
            {
                // Closed: its stack is disposed, as the desktop host disposes a closed window's.
                var closed = new WindowStack(_ => Greet);
                var closedScreen = new Screen(closed, ThemeLook.Clarity(dark: false));
                closedScreen.Resize(800, 400, 1);
                closed.Push(new CountingViewModel());
                var closedView = closed.View!;
                closed.Dispose();
                // Off its screen: the screen is disposed, so the stack leaves it.
                var gone = new WindowStack(_ => Greet);
                var goneScreen = new Screen(gone, ThemeLook.Clarity(dark: false));
                goneScreen.Resize(800, 400, 1);
                gone.Push(new CountingViewModel());
                var goneView = gone.View!;
                goneScreen.Dispose();
                // Still shown: it is made again in the new language.
                var open = new WindowStack(_ => Greet);
                var openScreen = new Screen(open, ThemeLook.Clarity(dark: false));
                openScreen.Resize(800, 400, 1);
                open.Push(new CountingViewModel());
                var openView = open.View!;

                Locale.Choose("nl");

                Assert.Same(closedView, closed.View);
                Assert.Same(goneView, gone.View);
                Assert.NotSame(openView, open.View);
                Assert.Equal("Hallo", open.View!.Parts[0].Host.Text("@hello"));
            }
            finally
            {
                Locale.Choose("en");
                Locale.Reset();
            }
        }
    }
}
