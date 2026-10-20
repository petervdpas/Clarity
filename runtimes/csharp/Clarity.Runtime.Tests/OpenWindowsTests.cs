// Every window that opens has a number of its own, and what belongs to one
// open window (an engine's state per window) hears when it closes.

using Xunit;

namespace Clarity.Runtime.Tests;

public class OpenWindowsTests
{
    [Fact]
    public void EveryOpenWindowHasItsOwnNumber()
    {
        var (first, second) = (OpenWindows.Next(), OpenWindows.Next());
        Assert.True(first > 0 && second > 0 && first != second);
    }

    [Fact]
    public void AClosedWindowIsReportedToWhatListens()
    {
        var closed = new List<long>();
        void Listen(long instance) => closed.Add(instance);
        OpenWindows.Closed += Listen;
        try
        {
            var window = OpenWindows.Next();
            OpenWindows.Close(window);
            Assert.Contains(window, closed);
        }
        finally
        {
            OpenWindows.Closed -= Listen;
        }
    }
}
