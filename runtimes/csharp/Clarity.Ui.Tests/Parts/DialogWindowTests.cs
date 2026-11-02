// A dialog (DESIGN 7.9): on the desktop in a window of its own, sized to
// the dialog, as is a target marked `in new window`; where the host opens no
// window of its own (a browser page) a dialog shows over the window as a
// modal overlay, its keys its own. Closing it tells the window that opened
// it, and a dialog asked for gives what it closes with to the window that
// asked.

using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Tests;

sealed class PickFriendViewModel() : WindowViewModel(null)
{
    public override bool IsDialog => true;

    public override string Title => "Pick a friend";

    public int ClosingTimes;

    protected override void OnClose() => ClosingTimes++;

    public void Picked(string friend) => Handle(() => CloseWith(friend));

    RelayCommand? closeWindow;
    public IRelayCommand CloseWindow_Command => closeWindow ??= new RelayCommand(() => Handle(Close));
}

sealed class LenderViewModel() : WindowViewModel(null)
{
    public List<string> Log { get; } = [];
    public PickFriendViewModel? Asked { get; private set; }

    public override string Title => "Lender";

    public int ReturnedTimes;

    protected override void OnReturn() => ReturnedTimes++;

    public void LendClicked() => Handle(() =>
    {
        Asked = new PickFriendViewModel();
        Ask(Asked, answer => Log.Add($"lent to {(string)answer!}"));
    });

    public void OpenApart(WindowViewModel other) => Handle(() => Open(other, newWindow: true));

    public void OpenHere(WindowViewModel other) => Handle(() => Open(other));
}

public class DialogWindowTests
{
    const string Dialog = """
        <Window name="PickFriend">
          <Label name="who" at="1,1" span="12" caption="Who?" />
          <Button name="cancelButton" at="1,2" span="6" caption="Cancel" cancel="true" onclick="close" />
        </Window>
        """;

    static string MarkupOf(WindowViewModel model) => model is PickFriendViewModel ? Dialog : "<Window name=\"Lender\" />";

    static (Screen Screen, WindowStack Stack, LenderViewModel Lender) Shown(Action<WindowViewModel, WindowViewModel>? apart = null)
    {
        var stack = new WindowStack(MarkupOf) { Apart = apart };
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 600, 1);
        var lender = new LenderViewModel();
        stack.Push(lender);
        Shot.Of(screen, 800, 600);
        return (screen, stack, lender);
    }

    [Fact]
    public void Without_windows_of_its_own_a_dialog_shows_over_the_window_sized_to_it()
    {
        var (screen, stack, lender) = Shown();
        lender.LendClicked();
        var overlay = Assert.Single(screen.Overlays);
        Assert.True(overlay.Modal);
        var dialog = Assert.IsType<DialogOverlay>(overlay.Content);
        Assert.Same(lender.Asked, dialog.Model);
        Assert.Equal("Pick a friend", dialog.Title);
        // The window below stays the one in place; the dialog is the last of the windows.
        Assert.Same(lender, stack.Current);
        Assert.Equal([lender, lender.Asked!], stack.Windows);
        Shot.Of(screen, 800, 600);
        var size = WindowStack.DialogSize(dialog.View, lender.Asked!);
        Assert.Equal(size.Width, dialog.Panel.Width);
        Assert.True(dialog.Panel.Height < 600 && dialog.Panel.Height >= 2 * GridLayout.RowHeight);
        Assert.Equal(2, dialog.View.Parts.Count);
    }

    [Fact]
    public void A_dialog_asked_for_gives_what_it_closes_with_to_the_window_that_asked()
    {
        var (screen, stack, lender) = Shown();
        lender.LendClicked();
        lender.Asked!.Picked("Ada");
        Assert.Empty(screen.Overlays);
        Assert.Equal(["lent to Ada"], lender.Log);
        Assert.Equal(1, lender.Asked.ClosingTimes);
        Assert.Equal(1, lender.ReturnedTimes);
        Assert.Equal([lender], stack.Windows);
    }

    [Fact]
    public void Escape_presses_the_dialog_s_cancel_button_and_runs_nothing()
    {
        var (screen, _, lender) = Shown();
        lender.LendClicked();
        Shot.Of(screen, 800, 600);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(screen.Overlays);
        Assert.Empty(lender.Log);
        Assert.Equal(1, lender.ReturnedTimes);
    }

    [Fact]
    public void A_host_with_windows_of_its_own_opens_dialogs_and_new_windows_there()
    {
        var apart = new List<(WindowViewModel Window, WindowViewModel Opener)>();
        var (screen, stack, lender) = Shown((window, opener) => apart.Add((window, opener)));
        lender.LendClicked();
        var target = new LenderViewModel();
        lender.OpenApart(target);
        Assert.Equal([(lender.Asked!, (WindowViewModel)lender), (target, lender)], apart);
        Assert.Empty(screen.Overlays);
        Assert.Equal([lender], stack.Windows);

        // In place is in place, whatever the host.
        var here = new LenderViewModel();
        lender.OpenHere(here);
        Assert.Same(here, stack.Current);
    }

    [Fact]
    public void Without_windows_of_its_own_a_new_window_opens_in_place()
    {
        var (_, stack, lender) = Shown();
        var target = new LenderViewModel();
        lender.OpenApart(target);
        Assert.Same(target, stack.Current);
    }

    [Fact]
    public void A_stack_of_its_own_tells_the_window_that_opened_it_when_it_ends()
    {
        var opener = new LenderViewModel();
        var stack = new WindowStack(MarkupOf) { Opener = opener };
        _ = new Screen(stack, ThemeLook.Clarity(dark: false));
        var dialog = new PickFriendViewModel();
        stack.Push(dialog);
        dialog.CloseWindow_Command.Execute(null);
        Assert.Equal(1, opener.ReturnedTimes);
        Assert.Equal(1, dialog.ClosingTimes);
    }

    [Fact]
    public void A_dialog_is_as_wide_as_it_says_or_the_standard_and_as_high_as_its_rows()
    {
        var spec = WindowMarkup.Read(Dialog);
        var view = new WindowView(spec, new TestHost());
        var size = WindowStack.DialogSize(view, new PickFriendViewModel());
        Assert.Equal(WindowStack.DialogWidth, size.Width);
        Assert.Equal(2 * GridLayout.RowHeight + GridLayout.Gap + 2 * WindowView.ContentPadding, size.Height);
    }
}
