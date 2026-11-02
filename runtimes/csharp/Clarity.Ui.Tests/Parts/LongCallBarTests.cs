// A long engine call (DESIGN 5.12): while a handler's engine call runs on
// another thread, the window keeps drawing, shows how far the call is with a
// bar and Stop over the window, and takes no other handler. Once the
// handler ended the bar goes.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

/// <summary>A long call makes the whole application wait: these run alone, so no other test's handler meets the wait.</summary>
[CollectionDefinition("LongCallBar", DisableParallelization = true)]
public sealed class LongCallBarCollection;

[Collection("LongCallBar")]
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
        // Escape stops too.
        Assert.True(bar.KeyPressed(new KeyInput(Key.Escape, Modifiers.None)));
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
        LongCall.Waiter = call =>
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
        try
        {
            var handled = model.Handle(() =>
            {
                // As an engine call does: it reports from its own thread and the screen waits.
                var watch = Watching.Current!;
                var call = Task.Run(() =>
                {
                    watch.Progress!(30, 120);
                    while (!watch.Stop.IsCancellationRequested)
                        Thread.Sleep(5);
                    stopped = true;
                });
                watch.Wait!(call);
            });
            Assert.True(handled);
        }
        finally
        {
            LongCall.Waiter = null;
        }
        Assert.NotNull(shown);
        Assert.Equal(0.25, seen);
        Assert.True(stopped);
        Assert.False(otherRan);
        // Once the handler ended, the bar is gone.
        Assert.DoesNotContain(screen.Overlays, o => o.Content is LongCallBar);
    }
}
