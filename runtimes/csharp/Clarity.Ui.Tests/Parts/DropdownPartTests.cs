// The plain dropdown: a closed box showing its choice's label, which opens
// its list over the window below it (above when there is no room below).
// Picking another choice is a change; the one it has, closing the list and
// what code sets are not. Closed, Up and Down pick the choice before or
// after and a typed letter jumps to a choice; Enter, Alt+Down and F4 open
// the list, whose Up, Down, Enter and Escape work it. The settings language
// and theme offer the project's languages and themes.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class DropdownPartTests
{
    static TestHost Owners()
    {
        var host = new TestHost();
        host.Values["task.owner"] = 1L;
        host.Offered["task.owner"] = [new(null, ""), new(1L, "Ada"), new(2L, "Grace")];
        return host;
    }

    const string Owner = """<Dropdown at="1,1" span="4" name="owner" bind="task.owner" onchange="ownerChanged" />""";

    static void Click(Screen screen, double x, double y)
    {
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    static void Press(Screen screen, Key key) => screen.KeyPressed(new KeyInput(key, Modifiers.None));

    [Fact]
    public void It_shows_its_choices_label_and_what_code_sets_is_no_change()
    {
        var host = Owners();
        var (_, _, parts) = Made.Window(Owner, host);
        var dropdown = Assert.IsType<DropdownPart>(parts[0]);
        Assert.Equal("Ada", dropdown.Shown);
        host.Values["task.owner"] = 2;
        host.Change();
        Assert.Equal("Grace", dropdown.Shown);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_click_opens_its_list_below_it_and_a_pick_changes_it()
    {
        var host = Owners();
        var (screen, _, parts) = Made.Window(Owner, host);
        var dropdown = (DropdownPart)parts[0];
        Click(screen, 20, 18);
        var list = Assert.Single(screen.Overlays);
        Assert.True(list.Place.Y >= dropdown.Bounds.Bottom);
        Assert.Equal(1, dropdown.Highlighted);
        Click(screen, list.Place.X + 10, list.Place.Y + 1 + DropdownList.Line * 2.5);
        Assert.Empty(screen.Overlays);
        Assert.Equal(["write owner task.owner 2", "run owner change ownerChanged"], host.Log);
        Assert.Same(dropdown, screen.Focused);
    }

    [Fact]
    public void Picking_the_choice_it_has_or_a_press_outside_is_no_change()
    {
        var host = Owners();
        var (screen, _, _) = Made.Window(Owner, host);
        Click(screen, 20, 18);
        var list = screen.Overlays[0].Place;
        Click(screen, list.X + 10, list.Y + 1 + DropdownList.Line * 1.5);
        Click(screen, 20, 18);
        Click(screen, 700, 300);
        Assert.Empty(screen.Overlays);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void Up_Down_Enter_and_Escape_work_its_list()
    {
        var host = Owners();
        var (screen, _, parts) = Made.Window(Owner, host);
        var dropdown = (DropdownPart)parts[0];
        Press(screen, Key.Tab);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.Alt));
        Assert.NotNull(dropdown.List);
        Press(screen, Key.Down);
        Press(screen, Key.Escape);
        Assert.Null(dropdown.List);
        Assert.Empty(host.Log);
        Press(screen, Key.Enter);
        Press(screen, Key.Up);
        Assert.Equal(0, dropdown.Highlighted);
        Press(screen, Key.Enter);
        Assert.Null(dropdown.List);
        Assert.Null(host.Values["task.owner"]);
        Assert.Equal("", dropdown.Shown);
        Assert.Equal(["write owner task.owner ", "run owner change ownerChanged"], host.Log);
    }

    /// As Avalonia's ComboBox: closed, Up and Down change the choice at once, and a typed letter jumps to one.
    [Fact]
    public void Closed_Up_and_Down_pick_the_choice_beside_it_and_a_letter_jumps_to_one()
    {
        var host = Owners();
        var (screen, _, parts) = Made.Window(Owner, host);
        var dropdown = (DropdownPart)parts[0];
        Press(screen, Key.Tab);
        Press(screen, Key.Down);
        Assert.Null(dropdown.List);
        Assert.Equal(2L, host.Values["task.owner"]);
        // At the end Down stays.
        Press(screen, Key.Down);
        Assert.Equal(["write owner task.owner 2", "run owner change ownerChanged"], host.Log);
        Press(screen, Key.Up);
        Assert.Equal(1L, host.Values["task.owner"]);
        host.Log.Clear();
        screen.TextTyped("g");
        Assert.Equal(2L, host.Values["task.owner"]);
        Assert.Equal("Grace", dropdown.Shown);
        // While the list is open a letter marks, and Enter picks.
        Press(screen, Key.F4);
        screen.TextTyped("a");
        Assert.Equal(1, dropdown.Highlighted);
        Press(screen, Key.Enter);
        Assert.Equal(1L, host.Values["task.owner"]);
    }

    [Fact]
    public void Choices_that_come_after_the_value_do_not_clear_it()
    {
        var host = new TestHost();
        host.Values["task.owner"] = 2L;
        var (_, _, parts) = Made.Window(Owner, host);
        var dropdown = (DropdownPart)parts[0];
        Assert.Equal("", dropdown.Shown);
        host.Offered["task.owner"] = [new(null, ""), new(1L, "Ada"), new(2L, "Grace")];
        host.Change();
        Assert.Equal("Grace", dropdown.Shown);
        Assert.Equal(2L, host.Values["task.owner"]);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void Its_list_opens_above_it_when_there_is_no_room_below()
    {
        var host = Owners();
        var (screen, _, parts) = Made.Window("""<Dropdown at="1,8" span="4" name="owner" bind="task.owner" />""", host);
        Click(screen, 20, parts[0].Bounds.Y + 18);
        Assert.True(screen.Overlays[0].Place.Bottom <= parts[0].Bounds.Y);
    }

    [Fact]
    public void A_disabled_or_read_only_dropdown_does_not_open()
    {
        var host = Owners();
        var (screen, _, parts) = Made.Window("""
            <Dropdown at="1,1" span="4" name="first" bind="task.owner" readonly="true" />
            <Dropdown at="1,2" span="4" name="second" bind="task.owner" />
            """, host);
        host.State("second")!.IsEnabled = false;
        Click(screen, 20, parts[0].Bounds.Y + 18);
        Click(screen, 20, parts[1].Bounds.Y + 18);
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void The_language_and_theme_settings_offer_the_projects_own()
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window("""
            <Dropdown at="1,1" span="4" bind="settings.language" />
            <Dropdown at="1,2" span="4" bind="settings.theme" />
            """, host);
        Assert.Equal(Choice.LanguagesOffered, ((DropdownPart)parts[0]).Choices);
        Assert.Equal(Choice.ThemesOffered, ((DropdownPart)parts[1]).Choices);
    }
}
