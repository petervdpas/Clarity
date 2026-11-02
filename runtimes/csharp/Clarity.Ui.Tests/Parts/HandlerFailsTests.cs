// A handler that fails in a way nobody foresaw (FailSafeTests of Avalonia):
// the window says why in an error message, in Clarity's dialog, and goes on
// working: the next handler runs.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

sealed class FailingViewModel() : WindowViewModel(null)
{
    public override string Title => "Failing";

    public int Ran;

    public void Fail() => Handle(() => throw new InvalidOperationException("the disk is gone"));

    public void Work() => Handle(() => Ran++);
}

public class HandlerFailsTests
{
    [Fact]
    public void A_handler_that_fails_says_why_and_the_window_keeps_working()
    {
        var stack = new WindowStack(_ => "<Window name=\"Failing\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false)) { Keys = stack.KeyPressed };
        screen.Resize(800, 500, 1);
        var model = new FailingViewModel();
        stack.Push(model);
        model.Fail();
        stack.View!.Tick();
        var said = Assert.IsType<SaidDialog>(Assert.Single(screen.Overlays).Content);
        Assert.Equal(Level.Error, said.Message.Level);
        Assert.Contains("the disk is gone", said.Message.Shown);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Empty(screen.Overlays);
        model.Work();
        Assert.Equal(1, model.Ran);
    }
}
