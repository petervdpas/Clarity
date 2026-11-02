// The menu bar's access keys, as desktop programs have them: each top item
// has a letter of its caption (its first letter, else the next one no item
// before it took), underlined while Alt is held; Alt with that letter opens
// its menu. Alt pressed and let go alone keeps the letters shown, and then
// the letter alone opens the menu; any other key, or Escape, ends that. The
// same cases as the Rust version's tests/looks_menu_access.rs.

namespace Clarity.Ui.Tests;

public class FrameAccessTests
{
    const string Menu = """
        <Menu name="mainMenu">
          <Item caption="File"><Item caption="Open" onclick="open" /></Item>
          <Item caption="View"><Item caption="Zoom" onclick="zoom" /></Item>
          <Item caption="Format"><Item caption="Bold" onclick="bold" /></Item>
          <Item caption="Help" onclick="help" />
        </Menu>
        """;

    static (Screen Screen, WindowView View, TestHost Host) Shown()
    {
        var shown = Framed.Window(Menu);
        shown.Screen.KeysUp = shown.View.KeyReleased;
        Framed.Draw(shown.Screen);
        return shown;
    }

    static List<List<string>> OpenCaptions(WindowView view) => [.. view.Frame.Menus.Lists.Select(l => l.Entries.Select(e => e.Caption).ToList())];

    static void Release(Screen screen, Key key, Modifiers held = Modifiers.None) => screen.KeyReleased(new KeyInput(key, held));

    [Fact]
    public void Each_top_item_has_its_first_free_letter() =>
        Assert.Equal([0, 0, 1, 0, null, null], FrameRules.AccessKeys(["File", "View", "Format", "Help", "", "Fv"]));

    [Fact]
    public void Alt_with_a_letter_opens_that_menu()
    {
        var (screen, view, _) = Shown();
        Framed.Press(screen, Key.Alt, Modifiers.Alt);
        Framed.Press(screen, Key.V, Modifiers.Alt);
        Assert.Equal([["Zoom"]], OpenCaptions(view));
        Release(screen, Key.Alt);
        // Format's letter is its o: File took the F.
        Framed.Press(screen, Key.Escape);
        Framed.Press(screen, Key.O, Modifiers.Alt);
        Assert.Equal([["Bold"]], OpenCaptions(view));
    }

    [Fact]
    public void A_top_item_without_items_runs_as_a_click()
    {
        var (screen, _, host) = Shown();
        Framed.Press(screen, Key.H, Modifiers.Alt);
        Assert.Equal(["run  click help"], host.Log);
    }

    [Fact]
    public void Alt_held_shows_the_letters_and_alt_alone_keeps_them_for_a_plain_letter()
    {
        var (screen, view, _) = Shown();
        var bar = view.Frame.MenuBar!;
        Assert.False(bar.AccessShown);
        Framed.Press(screen, Key.Alt, Modifiers.Alt);
        Assert.True(bar.AccessShown);
        Framed.Draw(screen);
        Release(screen, Key.Alt);
        Assert.True(bar.AccessShown, "Alt alone keeps them shown");
        Framed.Press(screen, Key.F);
        Assert.Equal([["Open"]], OpenCaptions(view));
        Assert.False(bar.AccessShown);
    }

    [Fact]
    public void Another_key_after_alt_alone_ends_the_letters()
    {
        var (screen, view, _) = Shown();
        Framed.Press(screen, Key.Alt, Modifiers.Alt);
        Release(screen, Key.Alt);
        Framed.Press(screen, Key.Escape);
        Assert.False(view.Frame.MenuBar!.AccessShown);
        Framed.Press(screen, Key.F);
        Assert.Empty(view.Frame.Menus.Lists);
    }

    [Fact]
    public void Alt_used_with_a_letter_shows_no_letters_after()
    {
        var (screen, view, _) = Shown();
        Framed.Press(screen, Key.Alt, Modifiers.Alt);
        Framed.Press(screen, Key.F, Modifiers.Alt);
        Release(screen, Key.Alt);
        Assert.False(view.Frame.MenuBar!.AccessShown);
    }
}
