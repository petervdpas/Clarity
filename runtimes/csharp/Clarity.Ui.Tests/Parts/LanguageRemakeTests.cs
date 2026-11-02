// A language chosen while windows are open (LocaleTests of Avalonia): every
// window's view is made again in it, and the title follows. The language is
// the whole application's, so this runs alone.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

[CollectionDefinition("Language", DisableParallelization = true)]
public sealed class LanguageCollection;

sealed class GreetViewModel() : WindowViewModel(null)
{
    public override string Title => Locale.Translate("greetTitle");
}

[Collection("Language")]
public class LanguageRemakeTests
{
    [Fact]
    public void ALanguageChangeRebuildsTheViewsOfEveryWindow()
    {
        string? Lookup(string key, CultureInfo culture) => (key, culture.TwoLetterISOLanguageName) switch
        {
            ("hello", "nl") => "Hallo",
            ("hello", _) => "Hello",
            ("greetTitle", "nl") => "Groet",
            ("greetTitle", _) => "Greeting",
            _ => null,
        };
        Locale.Start("en", ["en", "nl"], null, Lookup, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
        try
        {
            var stack = new WindowStack(_ => """<Window name="Greet"><Label name="hi" at="1,1" span="6" caption="@hello" /></Window>""");
            var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
            screen.Resize(800, 400, 1);
            var below = new GreetViewModel();
            var top = new GreetViewModel();
            stack.Push(below);
            stack.Push(top);
            var before = stack.View!;
            var titles = new List<string>();
            stack.TitleChanged += () => titles.Add(stack.Current!.Title);
            Assert.Equal("Hello", before.Parts[0].Host.Text("@hello"));

            Locale.Choose("nl");

            Assert.NotSame(before, stack.View);
            Assert.Equal("Hallo", stack.View!.Parts[0].Host.Text("@hello"));
            Assert.Equal(["Groet"], titles);
            // The window below is made again too: it shows in Dutch once it is on top.
            stack.Close(top);
            Assert.Same(below, stack.Current);
            Assert.Equal("Hallo", stack.View!.Parts[0].Host.Text("@hello"));
        }
        finally
        {
            Locale.Choose("en");
            Locale.Reset();
        }
    }
}
