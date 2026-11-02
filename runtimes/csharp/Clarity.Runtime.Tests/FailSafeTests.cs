// The runtime fails safe: a handler that hits an error nobody foresaw says
// so as an error message and the window keeps working (LANGUAGE.md, Fail
// safe; Avalonia's FailSafeTests).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

sealed class FailingViewModel(Db db) : WindowViewModel(db)
{
    public List<string> Log { get; } = [];

    public bool Broken() => Handle(() =>
    {
        List<long> none = [];
        Log.Add($"never {none[3]}");
    });

    public bool Fine() => Handle(() => Log.Add("fine"));
}

public class FailSafeTests
{
    [Fact]
    public void A_handler_that_fails_says_why_and_the_window_keeps_working()
    {
        using var db = Sqlite.Memory();
        var window = new FailingViewModel(db);
        Assert.False(window.Broken());
        var said = Assert.Single(window.Messages);
        Assert.Equal(Level.Error, said.Level);
        Assert.Contains("out of range", said.Text.Show());
        Assert.True(window.Fine());
        Assert.Equal(["fine"], window.Log);
    }
}
