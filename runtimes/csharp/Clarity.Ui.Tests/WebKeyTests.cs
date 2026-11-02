// A browser's names of keys are Clarity.UI's keys, as the desktop's are.

using Clarity.Ui.Browser;

namespace Clarity.Ui.Tests;

public class WebKeyTests
{
    [Theory]
    [InlineData("Tab", Key.Tab)]
    [InlineData("Enter", Key.Enter)]
    [InlineData(" ", Key.Space)]
    [InlineData("ArrowLeft", Key.Left)]
    [InlineData("PageDown", Key.PageDown)]
    [InlineData("F1", Key.F1)]
    [InlineData("F12", Key.F12)]
    [InlineData("a", Key.A)]
    [InlineData("Z", Key.Z)]
    [InlineData("7", Key.D7)]
    [InlineData("+", Key.Plus)]
    [InlineData("-", Key.Minus)]
    [InlineData("*", Key.Multiply)]
    [InlineData("/", Key.Slash)]
    [InlineData(".", Key.Period)]
    [InlineData(",", Key.Comma)]
    [InlineData("=", Key.Equal)]
    [InlineData("F13", Key.Other)]
    [InlineData("Dead", Key.Other)]
    [InlineData("é", Key.Other)]
    public void A_browser_key_is_the_same_key(string named, Key key) => Assert.Equal(key, WebKeys.Of(named));
}
