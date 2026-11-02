// The dialog that says a warning or an error shows its text and OK, as the
// shared web screen's does: no heading of its own, whose word would not be
// in the project's languages; the colored edge says the level.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class SaidDialogTests
{
    [Fact]
    public void It_says_its_text_and_ok_without_a_heading()
    {
        var dialog = new SaidDialog(new Message(Level.Error, Text.Plain("The game is over")));
        var screen = new Screen(dialog, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 400, 1);
        Shot.Of(screen, 800, 400);
        // Its padding, one line of text, its padding, OK and its padding.
        Assert.Equal(20 + 20 + 20 + GridLayout.RowHeight + 20, dialog.Panel.Height);
    }
}
