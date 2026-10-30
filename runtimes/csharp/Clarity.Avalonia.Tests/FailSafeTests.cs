// The runtime fails safe: a handler that hits an error nobody foresaw says
// so as an error message and the window keeps working, and a control whose
// drawing fails shows why in its place instead of ending the application.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Threading;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

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

    [AvaloniaFact]
    public void A_control_whose_drawing_fails_shows_why_and_the_window_stays()
    {
        var control = new Border { Name = "dialCanvas", Width = 200, Height = 100 };
        var window = new Window { Width = 200, Height = 100, Content = control };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        string? failed = null;
        using (var context = new global::Avalonia.Media.Imaging.RenderTargetBitmap(new PixelSize(200, 100)).CreateDrawingContext())
            failed = FailSafe.Render(control, context, () => throw new InvalidOperationException("no drawing"));
        Assert.Equal("no drawing", failed);
        using (var context = new global::Avalonia.Media.Imaging.RenderTargetBitmap(new PixelSize(200, 100)).CreateDrawingContext())
            Assert.Null(FailSafe.Render(control, context, () => { }));
        window.Close();
    }
}
