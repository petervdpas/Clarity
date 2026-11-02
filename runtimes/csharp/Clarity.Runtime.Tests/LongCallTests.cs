// How a handler waits for a long engine call: whatever the screen's waiter
// does, a refusal or failure of the call reaches the handler as itself. A long
// call belongs to the windows that show together (a stack, a session): their
// other handlers wait for it, another's run on and see nothing of it.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class LongCallTests
{
    // A handler as generated code runs one: an engine call on another thread,
    // waited for through the handler's watch, its result read after.
    static void EngineLike(Func<object?> work)
    {
        var call = Task.Run(() =>
        {
            Thread.Sleep(20);
            return work();
        });
        Watching.Current!.Wait!(call);
        call.GetAwaiter().GetResult();
    }

    /// <summary>A waiter that ends with call.Wait() throws the AggregateException around the refusal; the window still says the refusal's own text.</summary>
    [Fact]
    public void A_refusal_in_a_waited_call_says_its_own_text()
    {
        var window = new ShelfViewModel(null);
        window.LongCall.Waiter = call => call.Wait();
        window.Handle(() => EngineLike(() => throw new RefusedException(Text.Plain("The game is over"))));
        Assert.Equal("The game is over", Assert.Single(window.Messages).Text.Show());
        Assert.False(window.LongCall.Waiting);
    }

    /// <summary>A failure wrapped in an AggregateException of one says the inner failure's text.</summary>
    [Fact]
    public void A_wrapped_failure_says_the_inner_text()
    {
        var window = new ShelfViewModel(null);
        window.Handle(() => throw new AggregateException(new RefusedException(Text.Plain("Not allowed"))));
        Assert.Equal("Not allowed", Assert.Single(window.Messages).Text.Show());
    }

    // A handler with a long call: it reports 30 of 120 and runs until it is stopped (at most 10 s).
    static bool StoppedCall()
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
    /// While one stack's handler waits for its long call, another stack's
    /// handler runs and sees no progress, and its Stop stops nothing of the
    /// first; a window of the first stack takes no handler. The other stack's
    /// handler runs on a thread of its own, as a test or a session beside it does.
    /// </summary>
    [Fact]
    public void A_long_call_holds_back_only_the_windows_it_belongs_to()
    {
        var waiting = new ShelfViewModel(null);
        var sameStack = new ShelfViewModel(null) { LongCall = waiting.LongCall };
        var other = new ShelfViewModel(null);
        var otherRan = false;
        var sameRan = true;
        (long, long)? otherSaw = (0, 0);
        (long, long)? waitingSaw = null;
        var otherWaiting = true;
        var stoppedByOther = true;
        waiting.LongCall.Waiter = call =>
        {
            var until = DateTime.UtcNow.AddSeconds(10);
            while (waiting.LongCall.Current is null && DateTime.UtcNow < until)
                Thread.Sleep(5);
            waitingSaw = waiting.LongCall.Current;
            var beside = new Thread(() =>
            {
                otherRan = other.Handle(() => { });
                otherSaw = other.LongCall.Current;
                otherWaiting = other.LongCall.Waiting;
                other.LongCall.Stop();
            });
            beside.Start();
            beside.Join();
            Thread.Sleep(50);
            stoppedByOther = call.IsCompleted;
            sameRan = sameStack.Handle(() => { });
            waiting.LongCall.Stop();
            call.Wait(TimeSpan.FromSeconds(10));
        };
        var stopped = false;
        Assert.True(waiting.Handle(() => stopped = StoppedCall()));
        Assert.Equal((30L, 120L), waitingSaw);
        Assert.True(otherRan);
        Assert.Null(otherSaw);
        Assert.False(otherWaiting);
        Assert.False(stoppedByOther);
        Assert.False(sameRan);
        Assert.True(stopped);
        Assert.Null(waiting.LongCall.Current);
        Assert.False(waiting.LongCall.Waiting);
    }

    /// <summary>A report that comes after its handler ended shows no bar that stays.</summary>
    [Fact]
    public void A_report_after_the_handler_ended_shows_nothing()
    {
        var window = new ShelfViewModel(null);
        EngineWatch? kept = null;
        window.Handle(() => kept = Watching.Current);
        kept!.Progress!(5, 10);
        Assert.Null(window.LongCall.Current);
    }
}
