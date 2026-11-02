// What the screen does for every element in its tree: one inside a hidden
// element draws nothing and takes neither Tab nor the pointer; elements are
// found by their type; an element that animates gets a frame drawn on each
// turn; a press on what an overlay opened from is no press outside it. Also
// the opener of addresses each system gets, and a part state's own look.

using System.Runtime.InteropServices;

namespace Clarity.Ui.Tests;

public class ScreenTreeTests
{
    [Fact]
    public void An_element_inside_a_hidden_part_takes_no_tab_and_draws_nothing()
    {
        var host = new TestHost();
        var (screen, grid, parts) = Made.Window("""
            <Textbox name="first" at="1,1" span="4" />
            <Panel name="box" at="1,2" span="6" rows="3" caption="Box"><Textbox name="inner" at="1,1" span="4" /></Panel>
            <Textbox name="last" at="1,5" span="4" />
            """, host);
        host.State("box")!.IsVisible = false;
        var inner = grid.Find<TextBoxPart>(p => p.Spec.Name == "inner")!;
        Assert.False(inner.ShowsInTree);
        Assert.True(((TextBoxPart)parts[0]).Editor.ShowsInTree);
        screen.Focus(((TextBoxPart)parts[0]).Editor);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Same(((TextBoxPart)parts[2]).Editor, screen.Focused);
        // Nothing of the panel is drawn: its place is the window's background.
        using var pixels = Shot.Of(screen, 800, 400);
        var at = inner.Bounds;
        Assert.Equal(screen.Look.Color("background"), pixels.PixelAt((int)(at.X + 4), (int)(at.Y + at.Height / 2)));
        Assert.IsNotType<TextEditor>(screen.At(at.X + 4, at.Y + at.Height / 2));
    }

    [Fact]
    public void A_busy_indicator_asks_for_frames_while_it_shows()
    {
        var host = new TestHost();
        var (screen, _, _) = Made.Window("""<Busy name="busy" at="1,1" span="4" caption="Loading" />""", host);
        using (Shot.Of(screen, 800, 400)) { }
        Assert.Equal(1, screen.Animating);
        host.State("busy")!.IsVisible = false;
        using (Shot.Of(screen, 800, 400)) { }
        Assert.Equal(0, screen.Animating);
    }

    [Fact]
    public void A_press_on_what_an_overlay_opened_from_goes_to_it()
    {
        var host = new TestHost { Values = { ["size"] = "S" }, Offered = { ["size"] = [new("S", "Small"), new("L", "Large")] } };
        var (screen, _, parts) = Made.Window("""<Dropdown name="size" at="1,1" span="4" bind="size" />""", host);
        var dropdown = (DropdownPart)parts[0];
        var (x, y) = (dropdown.Bounds.X + 10, dropdown.Bounds.Y + 10);
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
        Assert.NotNull(dropdown.List);
        // A second click on its own box closes the list, and the box keeps the focus.
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
        Assert.Null(dropdown.List);
        Assert.Same(dropdown, screen.Focused);
    }

    [Fact]
    public void Each_system_opens_an_address_with_its_own_program()
    {
        Assert.Equal(("xdg-open", "https://example.org", false), Flat(SystemAddress.CommandFor(OSPlatform.Linux, "https://example.org")));
        Assert.Equal(("open", "mailto:ada@example.org", false), Flat(SystemAddress.CommandFor(OSPlatform.OSX, "mailto:ada@example.org")));
        Assert.Equal(("https://example.org", "", true), Flat(SystemAddress.CommandFor(OSPlatform.Windows, "https://example.org")));
        // Only what a link opens is passed on.
        Assert.Null(SystemAddress.CommandFor(OSPlatform.Linux, "file:///etc/passwd"));
        Assert.Null(SystemAddress.CommandFor(OSPlatform.Windows, "calc.exe"));
    }

    static (string, string, bool) Flat(SystemAddress.Command? c) => (c!.Program, string.Join(" ", c.Arguments), c.Shell);

    [Fact]
    public void A_parts_state_look_is_only_what_the_state_sets()
    {
        var look = ThemeLook.Clarity(dark: false);
        var pressed = look.PartOwn("button", "pressed");
        Assert.Equal(look.Color("accent_soft"), pressed.Background);
        Assert.Null(pressed.Radius);
        Assert.Same(LookValues.None, look.PartOwn("button", null));
    }
}
