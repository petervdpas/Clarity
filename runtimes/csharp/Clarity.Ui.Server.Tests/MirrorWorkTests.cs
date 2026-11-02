// What the browser's mirror does beside sending commands: while a handler's
// long call runs on the server it shows the bar of how far it is, whose Stop
// stops the call there; a change the person made that a canvas draws has
// the server draw it anew; and each request carries the language and the
// formats the page shows, which the server's handler runs in. Through the
// server in the same process, as the browser reaches it.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using Clarity.Ui.Browser;
using Xunit;

namespace Clarity.Ui.Server.Tests;

[Collection("page")]
public class MirrorWorkTests
{
    static Screen OnScreen(MirrorStack mirror)
    {
        var screen = new Screen(mirror, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 600, 1);
        return screen;
    }

    /// <summary>Waits for <paramref name="met"/>, at most ten seconds.</summary>
    static async Task Until(Func<bool> met)
    {
        var until = DateTime.UtcNow.AddSeconds(10);
        while (!met() && DateTime.UtcNow < until)
            await Task.Delay(10, TestContext.Current.CancellationToken);
        Assert.True(met());
    }

    [Fact]
    public async Task The_page_shows_the_bar_of_a_long_call_and_its_Stop_stops_the_call_on_the_server()
    {
        var link = new InProcess(() => new CrunchViewModel(null)) { Threaded = true };
        var mirror = await MirrorStack.Start(link);
        var screen = OnScreen(mirror);
        var crunch = Assert.IsType<CrunchViewModel>(mirror.Current);
        crunch.CrunchCommand.Execute(null);

        // The server said how far the call is: the bar shows it over the window.
        await Until(() => mirror.CallBar is not null);
        var bar = mirror.CallBar!;
        Assert.Contains(screen.Overlays, o => o.Content == bar);
        Assert.Equal("25%", bar.Said);
        Assert.Contains("progress", link.Sent);

        // Stop stops the call on the server; the reply comes, and the bar goes.
        bar.Stop.Press();
        Assert.True(bar.Stopped);
        await mirror.Idle.WaitAsync(TimeSpan.FromSeconds(10), TestContext.Current.CancellationToken);
        Assert.Contains("stop", link.Sent);
        Assert.True(crunch.Stopped);
        Assert.Null(mirror.CallBar);
        Assert.DoesNotContain(screen.Overlays, o => o.Content == bar);
    }

    [Fact]
    public async Task A_quick_handler_shows_no_bar()
    {
        var link = new InProcess(() => new CrunchViewModel(null)) { Threaded = true };
        var mirror = await MirrorStack.Start(link);
        OnScreen(mirror);
        ((CrunchViewModel)mirror.Current!).SteppedCommand.Execute(null);
        await mirror.Idle;
        Assert.Null(mirror.CallBar);
        Assert.Equal(1, ((CrunchViewModel)mirror.Current!).Steps);
    }

    [Fact]
    public async Task A_name_typed_in_a_bound_box_is_drawn_on_the_canvas_by_the_server()
    {
        var link = new InProcess(() => new SketchViewModel(null));
        var mirror = await MirrorStack.Start(link);
        OnScreen(mirror);
        var sketch = Assert.IsType<SketchViewModel>(mirror.Current);
        var box = mirror.View!.Parts.First(p => p.Spec.Name == "nameBox");
        ((ViewModelHost)box.Host).Write("nameBox", "name", "Ada");
        await Until(() => link.Commands.Contains(nameof(WindowViewModel.DrawAnew)));
        await mirror.Idle;
        var words = Assert.Single(sketch.Field.Drawing.OfType<Shape.Words>());
        Assert.Equal("Ada", Locale.Show(words.Text));
    }

    [Fact]
    public async Task Two_pages_in_two_languages_each_get_their_own_language_from_the_server()
    {
        var dutch = new Dictionary<string, string> { ["hello"] = "Hallo" };
        var english = new Dictionary<string, string> { ["hello"] = "Hello" };
        Locale.Start("en", ["nl"], null, (key, culture) => (culture.Name == "nl" ? dutch : english).GetValueOrDefault(key), (_, value) => value, System.Globalization.CultureInfo.InvariantCulture);
        try
        {
            var server = new WebServer(new Sessions(() => new HelloViewModel(null)), "", typeof(InProcess).Assembly);
            var dutchLink = new InProcess(server);
            var englishLink = new InProcess(server);
            var dutchPage = await MirrorStack.Start(dutchLink);
            var englishPage = await MirrorStack.Start(englishLink);
            using (Locale.For("nl", "nl-NL"))
            {
                ((HelloViewModel)dutchPage.Current!).HelloCommand.Execute(null);
                await dutchPage.Idle;
            }
            using (Locale.For("en", "en-US"))
            {
                ((HelloViewModel)englishPage.Current!).HelloCommand.Execute(null);
                await englishPage.Idle;
            }
            Assert.Equal("nl", dutchLink.Requests[^1]["locale"]!["language"]!.GetValue<string>());
            Assert.Equal("Hallo 2,5", ((HelloViewModel)dutchPage.Current!).Said);
            Assert.Equal("Hello 2.5", ((HelloViewModel)englishPage.Current!).Said);
            // The server's own language stays.
            Assert.Null(Locale.Language);
        }
        finally
        {
            Locale.Reset();
        }
    }
}
