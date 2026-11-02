// A language chosen while a part has the focus (seen in the noughts example's
// preferences): the window made again in the new language gives the focus
// back to that part, so the next Tab goes on from it. The language is the
// whole application's, so this runs with the other language tests.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

[Collection("Language")]
public class LanguageFocusTests
{
    [Fact]
    public void A_language_change_keeps_the_focus_on_its_part()
    {
        Locale.Start("en", ["en", "nl"], null, (_, _) => null, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
        try
        {
            var stack = new WindowStack(_ => """<Window name="Greet"><Textbox name="first" at="1,1" span="6" /><Textbox name="second" at="1,2" span="6" /><Button name="go" at="1,3" span="6" caption="Go" /></Window>""");
            var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
            screen.Resize(800, 400, 1);
            stack.Push(new GreetViewModel());
            screen.MoveFocus();
            screen.MoveFocus();
            Assert.Equal("second", NameOf(screen.Focused));

            Locale.Choose("nl");

            Assert.Equal("second", NameOf(screen.Focused));
            screen.MoveFocus();
            Assert.Equal("go", NameOf(screen.Focused));
        }
        finally
        {
            Locale.Choose("en");
            Locale.Reset();
        }
    }

    static string? NameOf(Element? element)
    {
        for (var e = element; e is not null; e = e.Parent)
            if (e is Part { Spec.Name: { } name })
                return name;
        return null;
    }
}
