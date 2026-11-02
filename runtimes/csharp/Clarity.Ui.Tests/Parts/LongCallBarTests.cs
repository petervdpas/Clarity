// A long engine call (DESIGN 5.12): while a handler's engine call runs on
// another thread, the window keeps drawing, shows how far the call is with a
// bar and Stop over the window, and takes no other handler. Once the
// handler ended the bar goes.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class LongCallBarTests
{
    [Fact]
    public void The_bar_shows_how_far_and_stop_stops()
    {
        var stopped = 0;
        var bar = new LongCallBar(() => (30, 120), () => stopped++);
        var screen = new Screen(bar, ThemeLook.Clarity(dark: false));
        Shot.Of(screen, 600, 400);
        Assert.Equal(0.25, bar.Fraction);
        Assert.Equal("25%", bar.Said);
        Assert.Equal("Stop", bar.Stop.Text);
        var stop = bar.Stop.Bounds;
        screen.PointerPressed(stop.X + 4, stop.Y + 4, PointerButton.Left);
        screen.PointerReleased(stop.X + 4, stop.Y + 4, PointerButton.Left);
        Assert.Equal(1, stopped);
        // Stop disables itself once pressed (Avalonia's Messages.cs): neither a press nor Escape stops again.
        Assert.False(bar.Stop.EnabledInTree);
        screen.PointerPressed(stop.X + 4, stop.Y + 4, PointerButton.Left);
        screen.PointerReleased(stop.X + 4, stop.Y + 4, PointerButton.Left);
        Assert.True(bar.KeyPressed(new KeyInput(Key.Escape, Modifiers.None)));
        Assert.Equal(1, stopped);
        // Escape stops a call whose Stop was not pressed.
        var again = new LongCallBar(() => (1, 2), () => stopped++);
        Assert.True(again.KeyPressed(new KeyInput(Key.Escape, Modifiers.None)));
        Assert.Equal(2, stopped);
    }

    [Fact]
    public void A_call_that_says_nothing_yet_shows_an_empty_bar()
    {
        var bar = new LongCallBar(() => null, () => { });
        Assert.Equal(0, bar.Fraction);
        Assert.Equal("", bar.Said);
    }

    [Fact]
    public void A_long_call_shows_the_bar_over_the_window_and_stops()
    {
        var stack = new WindowStack(_ => "<Window name=\"Albums\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 500, 1);
        var model = new AlbumsViewModel();
        stack.Push(model);
        LongCallBar? shown = null;
        var otherRan = true;
        var seen = 0.0;
        var stopped = false;
        stack.LongCall.Waiter = call =>
        {
            // The screen waits its own way: here, until the bar shows what the call said.
            var until = DateTime.UtcNow.AddSeconds(10);
            while (DateTime.UtcNow < until && (shown = screen.Overlays.Select(o => o.Content).OfType<LongCallBar>().FirstOrDefault()) is not { Fraction: > 0 })
                Thread.Sleep(5);
            otherRan = model.Handle(() => { });
            if (shown is not null)
            {
                seen = shown.Fraction;
                var stop = shown.Stop.Bounds;
                screen.PointerPressed(stop.X + 4, stop.Y + 4, PointerButton.Left);
                screen.PointerReleased(stop.X + 4, stop.Y + 4, PointerButton.Left);
            }
            call.Wait(TimeSpan.FromSeconds(10));
        };
        var handled = model.Handle(() => stopped = LongHandler());
        Assert.True(handled);
        Assert.NotNull(shown);
        Assert.Equal(0.25, seen);
        Assert.True(stopped);
        Assert.False(otherRan);
        // Once the handler ended, the bar is gone.
        Assert.DoesNotContain(screen.Overlays, o => o.Content is LongCallBar);
    }

    // As an engine call does: it reports from its own thread and the screen waits, until it is stopped (at most 10 s).
    static bool LongHandler()
    {
        var watch = Watching.Current!;
        var stopped = false;
        var call = Task.Run(() =>
        {
            watch.Progress!(30, 120);
            var until = DateTime.UtcNow.AddSeconds(10);
            while (!watch.Stop.IsCancellationRequested && DateTime.UtcNow < until)
                Thread.Sleep(5);
            stopped = watch.Stop.IsCancellationRequested;
        });
        watch.Wait!(call);
        return stopped;
    }

    /// <summary>
    /// Two window stacks (two desktop programs' or two tabs'): a long call in
    /// one shows its bar only there, the other's handlers run on, and the
    /// other's Stop stops nothing of it. A desktop window of its own shares
    /// its opener's call.
    /// </summary>
    [Fact]
    public void A_long_call_shows_and_waits_only_in_its_own_stack()
    {
        var first = new WindowStack(_ => "<Window name=\"Albums\" />");
        var firstScreen = new Screen(first, ThemeLook.Clarity(dark: false));
        firstScreen.Resize(800, 500, 1);
        var second = new WindowStack(_ => "<Window name=\"Albums\" />");
        var secondScreen = new Screen(second, ThemeLook.Clarity(dark: false));
        secondScreen.Resize(800, 500, 1);
        var waiting = new AlbumsViewModel();
        var other = new AlbumsViewModel();
        first.Push(waiting);
        second.Push(other);
        Assert.Same(first.LongCall, waiting.LongCall);
        Assert.Same(second.LongCall, other.LongCall);
        Assert.NotSame(first.LongCall, second.LongCall);
        Assert.Same(first.LongCall, new WindowStack(_ => "<Window name=\"Albums\" />") { Opener = waiting }.LongCall);
        var otherRan = false;
        var barBeside = true;
        var barHere = false;
        var stoppedByOther = true;
        first.LongCall.Waiter = call =>
        {
            var until = DateTime.UtcNow.AddSeconds(10);
            while (DateTime.UtcNow < until && !firstScreen.Overlays.Any(o => o.Content is LongCallBar))
                Thread.Sleep(5);
            barHere = firstScreen.Overlays.Any(o => o.Content is LongCallBar);
            otherRan = other.Handle(() => { });
            barBeside = secondScreen.Overlays.Any(o => o.Content is LongCallBar);
            second.LongCall.Stop();
            Thread.Sleep(50);
            stoppedByOther = call.IsCompleted;
            first.LongCall.Stop();
            call.Wait(TimeSpan.FromSeconds(10));
        };
        var stopped = false;
        Assert.True(waiting.Handle(() => stopped = LongHandler()));
        Assert.True(barHere);
        Assert.True(otherRan);
        Assert.False(barBeside);
        Assert.False(stoppedByOther);
        Assert.True(stopped);
        Assert.DoesNotContain(firstScreen.Overlays, o => o.Content is LongCallBar);
    }
}
