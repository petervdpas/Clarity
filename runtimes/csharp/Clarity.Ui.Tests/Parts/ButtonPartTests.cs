// The buttons: a button runs its click from the pointer, Enter and Space;
// it is drawn in the theme's button look with its hover and pressed looks
// and the focus ring; a disabled one does nothing. A toggle stays pressed,
// bound or in its state; a button group joins its buttons; a split button
// opens its menu from its arrow.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class ButtonPartTests
{
    static void Click(Screen screen, double x, double y)
    {
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    static void Click(Screen screen, Element part) => Click(screen, part.Bounds.X + part.Bounds.Width / 2, part.Bounds.Y + part.Bounds.Height / 2);

    static bool Near(SKColor a, SKColor b, int by = 10) =>
        Math.Abs(a.Red - b.Red) <= by && Math.Abs(a.Green - b.Green) <= by && Math.Abs(a.Blue - b.Blue) <= by;

    [Fact]
    public void A_click_runs_its_handlers()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Button at="1,1" span="2" name="seven" caption="7" onclick="digit(7)" />""", host);
        Assert.IsType<ButtonPart>(parts[0]);
        Click(screen, parts[0]);
        Assert.Equal(["run seven click digit(7)"], host.Log);
    }

    [Fact]
    public void Enter_and_Space_click_the_focused_button()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Button at="1,1" span="2" name="go" onclick="go" />""", host);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Same(parts[0], screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Space, Modifiers.None));
        Assert.Equal(["run go click go", "run go click go"], host.Log);
    }

    [Fact]
    public void A_disabled_button_does_nothing_and_takes_no_focus()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Button at="1,1" span="2" name="go" onclick="go" />""", host);
        host.State("go")!.IsEnabled = false;
        Click(screen, parts[0]);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Empty(host.Log);
        Assert.Null(screen.Focused);
    }

    [Fact]
    public void Its_caption_is_the_projects_text_or_what_code_sets()
    {
        var host = new TestHost();
        host.Texts["saveCaption"] = "Opslaan";
        var (_, _, parts) = Made.Window("""<Button at="1,1" span="2" name="save" caption="@saveCaption" />""", host);
        var button = Assert.IsType<ButtonPart>(parts[0]);
        Assert.Equal("Opslaan", button.Caption);
        host.State("save")!.CaptionText = Text.Of("Bewaar");
        Assert.Equal("Bewaar", button.Caption);
    }

    [Fact]
    public void It_is_drawn_in_the_themes_button_look_hovered_and_pressed()
    {
        var host = new TestHost();
        var (screen, _, _) = Made.Window("""<Button at="1,1" span="2" caption="" />""", host);
        var look = screen.Look;
        using (var plain = Shot.Of(screen, 800, 400))
            Assert.True(Near(look.Part("button", null).Background!.Value, plain.PixelAt(6, 18)));
        screen.PointerMoved(6, 18);
        using (var hovered = Shot.Of(screen, 800, 400))
            Assert.True(Near(look.Part("button", "hover").Border!.Value, hovered.PixelAt(0, 18), 40));
        screen.PointerPressed(6, 18, PointerButton.Left);
        using (var pressed = Shot.Of(screen, 800, 400))
            Assert.True(Near(look.Part("button", "pressed").Background!.Value, pressed.PixelAt(6, 18)));
    }

    [Fact]
    public void The_focused_button_shows_the_focus_ring_and_a_disabled_one_is_dimmed()
    {
        var host = new TestHost();
        var (screen, _, _) = Made.Window("""<Button at="1,1" span="2" name="go" caption="" />""", host);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        using (var focused = Shot.Of(screen, 800, 400))
            Assert.True(Near(screen.Look.Color("focus"), focused.PixelAt(1, 18), 40));
        screen.Focus(null);
        SKColor enabled;
        using (var shot = Shot.Of(screen, 800, 400))
            enabled = shot.PixelAt(0, 18);
        host.State("go")!.IsEnabled = false;
        using var dimmed = Shot.Of(screen, 800, 400);
        Assert.NotEqual(enabled, dimmed.PixelAt(0, 18));
    }

    [Fact]
    public void A_bound_toggle_stays_pressed_and_tells_its_change()
    {
        var host = new TestHost();
        host.Values["style.bold"] = false;
        var (screen, _, parts) = Made.Window("""<Toggle at="1,1" span="2" name="bold" bind="style.bold" onchange="restyle" />""", host);
        var toggle = Assert.IsType<TogglePart>(parts[0]);
        Click(screen, toggle);
        Assert.True(toggle.IsOn);
        Assert.Equal(["write bold style.bold True", "run bold change restyle"], host.Log);
        Click(screen, toggle);
        Assert.Equal(false, host.Values["style.bold"]);
    }

    [Fact]
    public void An_unbound_toggle_keeps_its_state_and_what_code_sets_is_no_change()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Toggle at="1,1" span="2" name="pin" onchange="pinned" />""", host);
        var toggle = Assert.IsType<TogglePart>(parts[0]);
        host.State("pin")!.IsChecked = true;
        Assert.True(toggle.IsOn);
        Assert.Empty(host.Log);
        Click(screen, toggle);
        Assert.False(host.State("pin")!.IsChecked);
        Assert.Equal(["run pin change pinned"], host.Log);
    }

    [Fact]
    public void A_read_only_toggle_is_not_pressed()
    {
        var host = new TestHost();
        host.Values["v"] = false;
        var (screen, _, parts) = Made.Window("""<Toggle at="1,1" span="2" name="pin" bind="v" readonly="true" onchange="pinned" />""", host);
        Click(screen, parts[0]);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_button_group_joins_its_buttons_with_only_its_outer_corners_round()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""
            <Buttons at="1,1" span="4">
              <Button at="1,1" span="2" name="left" onclick="goLeft" />
              <Toggle at="3,1" span="2" name="right" />
            </Buttons>
            """, host);
        var group = Assert.IsType<ButtonsPart>(parts[0]);
        var pushes = group.Children[0].Children.Cast<PressPart>().ToList();
        Assert.Equal(2, pushes.Count);
        Assert.Equal(pushes[0].Bounds.Right, pushes[1].Bounds.X, 3);
        Assert.Equal(group.Bounds.Right, pushes[1].Bounds.Right, 3);
        Assert.Equal((RoundCorners.Left, RoundCorners.Right), (pushes[0].Corners, pushes[1].Corners));
        Click(screen, pushes[0]);
        Click(screen, pushes[1]);
        Assert.Equal("run left click goLeft", host.Log[0]);
        Assert.True(host.State("right")!.IsChecked);
    }

    [Fact]
    public void A_split_button_runs_its_click_and_opens_its_menu_from_the_arrow()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""
            <Button at="1,1" span="4" name="save" split="true" onclick="save">
              <Menu><Item caption="Save as draft" onclick="saveDraft" /><Item kind="separator" /><Item caption="Save a copy" onclick="saveCopy(1)" /></Menu>
            </Button>
            """, host);
        var button = Assert.IsType<ButtonPart>(parts[0]);
        Click(screen, button.Bounds.X + 10, 18);
        Assert.Equal(["run save click save"], host.Log);
        Click(screen, button.Arrow.X + 10, 18);
        var menu = Assert.Single(screen.Overlays);
        Assert.True(menu.Place.Y >= button.Bounds.Bottom);
        Click(screen, menu.Place.X + 10, menu.Place.Y + 1 + DropdownList.Line * 1.5);
        Assert.Equal(["run save click save", "run save click saveCopy(1)"], host.Log);
        Assert.Empty(screen.Overlays);
        Assert.Null(button.Menu);
    }
}
