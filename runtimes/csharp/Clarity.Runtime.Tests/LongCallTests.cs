// How a handler waits for a long engine call: whatever the screen's waiter
// does, a refusal or failure of the call reaches the handler as itself.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

// While a call is waited for the application takes no other handler, for
// every window: these tests run alone so other tests' handlers still run.
[CollectionDefinition(nameof(LongCallTests), DisableParallelization = true)]
public sealed class LongCallAlone;

[Collection(nameof(LongCallTests))]
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
        var was = LongCall.Waiter;
        LongCall.Waiter = call => call.Wait();
        try
        {
            window.Handle(() => EngineLike(() => throw new RefusedException(Text.Plain("The game is over"))));
        }
        finally
        {
            LongCall.Waiter = was;
        }
        Assert.Equal("The game is over", Assert.Single(window.Messages).Text.Show());
        Assert.False(LongCall.Waiting);
    }

    /// <summary>A failure wrapped in an AggregateException of one says the inner failure's text.</summary>
    [Fact]
    public void A_wrapped_failure_says_the_inner_text()
    {
        var window = new ShelfViewModel(null);
        window.Handle(() => throw new AggregateException(new RefusedException(Text.Plain("Not allowed"))));
        Assert.Equal("Not allowed", Assert.Single(window.Messages).Text.Show());
    }
}
