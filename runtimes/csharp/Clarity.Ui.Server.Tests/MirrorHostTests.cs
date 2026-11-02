// The browser's mirror beyond what Avalonia's tests asked: a server that
// cannot be reached is said as an error and the window goes on, and a
// window's view is made once and shown again as it comes back on top.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using Clarity.Ui.Browser;
using Xunit;

namespace Clarity.Ui.Server.Tests;

/// <summary>A link to sessions in this process that can go down, as a server that stopped.</summary>
sealed class Breaking(ISessionLink inner) : ISessionLink
{
    public bool Down { get; set; }

    public Task<JsonObject> Send(JsonObject request) =>
        Down ? throw new HttpRequestException("Connection refused (127.0.0.1:8080)") : inner.Send(request);

    public Task<JsonObject> Markup() => inner.Markup();
}

[Collection("page")]
public class MirrorHostTests
{
    [Fact]
    public async Task A_server_that_cannot_be_reached_is_said_and_the_window_goes_on()
    {
        var link = new Breaking(new InProcess(() => new ShelfViewModel(null)));
        var mirror = await MirrorStack.Start(link);
        var main = (ShelfViewModel)mirror.Current!;
        link.Down = true;
        main.GreetCommand.Execute(null);
        await mirror.Idle;
        var said = Assert.Single(mirror.Messages);
        Assert.Equal(Level.Error, said.Level);
        Assert.Contains("Connection refused", Locale.Show(said.Text));
        Assert.Same(main, mirror.Current);
        // The server back: the next command runs there.
        link.Down = false;
        main.GreetCommand.Execute(null);
        await mirror.Idle;
        Assert.Same(main, mirror.Current);
    }

    [Fact]
    public async Task A_window_s_view_is_made_once_and_shown_again_when_it_is_back_on_top()
    {
        var mirror = await MirrorStack.Start(new InProcess(() => new ShelfViewModel(null)));
        var screen = new Screen(mirror, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 600, 1);
        var first = mirror.View;
        ((ShelfViewModel)mirror.Current!).DetailCommand.Execute(null);
        await mirror.Idle;
        Assert.NotSame(first, mirror.View);
        ((BookViewModel)mirror.Current!).BackCommand.Execute(null);
        await mirror.Idle;
        Assert.Same(first, mirror.View);
    }
}
