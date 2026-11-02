// A window's frame by keyboard and pointer, as Avalonia had it: Tab reaches
// the ribbon's tab strip (Left and Right open its tabs) and on into the
// buttons of the open tab; toolbar and ribbon buttons take the focus, show
// the focus ring and answer Enter and Space; the sidebar's fold and fill
// buttons too, and they say what they do. Folding a sidebar leaves whether
// it fills the window as it was; a double click anywhere on a ribbon but a
// button folds it; the sidebar's edge shows the resize pointer. F10 or Alt
// alone gives the menu bar the focus: Left and Right walk its menus, Down
// opens one, Escape leaves it.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class FrameKeyboardTests
{
    const string Ribbon = """
        <Ribbon name="mainRibbon" collapsible="true">
          <Tab title="Home" name="home">
            <Group title="Clipboard" name="clip">
              <Item name="paste" caption="Paste" large="true" onclick="paste" />
              <Item name="cut" caption="Cut" onclick="cut" />
            </Group>
          </Tab>
          <Tab title="Insert" name="insert">
            <Group title="Pictures"><Item caption="Picture" onclick="picture" /></Group>
          </Tab>
          <Tab title="View" name="view">
            <Group title="Zoom"><Item caption="Zoom" onclick="zoom" /></Group>
          </Tab>
        </Ribbon>
        """;

    const string Toolbar = """
        <Toolbar name="tools">
          <Item name="save" caption="Save" onclick="save" />
          <Item kind="separator" />
          <Item name="print" caption="Print" onclick="print" />
          <Item name="users" caption="Users" onclick="users" />
          <Item name="mail" caption="Mail" onclick="mail" />
        </Toolbar>
        """;

    const string Nav = """
        <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="4">
          <Button name="find" at="1,1" span="4" caption="Find" onclick="find" />
        </Sidebar>
        """;

    const string Menu = """
        <Menu name="mainMenu">
          <Item caption="File"><Item caption="Open" onclick="open" /><Item caption="Recent"><Item caption="Shop.db" onclick="recent" /></Item></Item>
          <Item name="edit" caption="Edit"><Item caption="Undo" onclick="undo" /></Item>
          <Item caption="View"><Item caption="Zoom" onclick="zoom" /></Item>
          <Item caption="Help" onclick="help" />
        </Menu>
        """;

    static string Caption(Element? e) => e switch
    {
        FrameButton b => b.Item.Text("caption") ?? "",
        RibbonTabStrip => "tabs",
        SidebarButton s => s.Fills ? "fill" : "fold",
        Part p => p.Spec.Name ?? "",
        _ => e?.GetType().Name ?? "none",
    };

    static bool Near(SKColor a, SKColor b) => Math.Abs(a.Red - b.Red) + Math.Abs(a.Green - b.Green) + Math.Abs(a.Blue - b.Blue) < 60;

    [Fact]
    public void The_ribbons_tab_strip_takes_the_focus_and_the_arrows_open_its_tabs()
    {
        var (screen, view, host) = Framed.Window(Ribbon);
        Framed.Draw(screen);
        var ribbon = view.Frame.Ribbon!;
        Framed.Press(screen, Key.Tab);
        Assert.Equal("tabs", Caption(screen.Focused));
        Framed.Press(screen, Key.Right);
        Assert.Equal(2, host.State("mainRibbon")!.Tab);
        // A hidden tab is passed over.
        host.State("insert")!.IsVisible = false;
        Framed.Draw(screen);
        Framed.Press(screen, Key.Left);
        Assert.Equal(1, ribbon.ShownTab);
        Framed.Press(screen, Key.Right);
        Assert.Equal(3, ribbon.ShownTab);
        // The last tab stays the last.
        Framed.Press(screen, Key.Right);
        Assert.Equal(3, ribbon.ShownTab);
        Framed.Press(screen, Key.Left);
        Framed.Draw(screen);
        // Tab goes on into the open tab's buttons.
        Framed.Press(screen, Key.Tab);
        Assert.Equal("Paste", Caption(screen.Focused));
        Framed.Press(screen, Key.Tab);
        Assert.Equal("Cut", Caption(screen.Focused));
        Framed.Press(screen, Key.Enter);
        Assert.Equal(["run cut click cut"], host.Log);
    }

    [Fact]
    public void Toolbar_buttons_are_reached_with_Tab_and_pressed_with_Enter_and_Space()
    {
        var (screen, view, host) = Framed.Window(Toolbar);
        host.State("users")!.IsEnabled = false;
        host.State("print")!.IsVisible = false;
        Framed.Draw(screen);
        Assert.Equal(["Save", "Mail"], screen.TabOrder().Select(Caption));
        Framed.Press(screen, Key.Tab);
        Assert.Equal("Save", Caption(screen.Focused));
        Framed.Press(screen, Key.Enter);
        Framed.Press(screen, Key.Tab);
        Framed.Press(screen, Key.Space);
        Assert.Equal(["run save click save", "run mail click mail"], host.Log);
    }

    [Fact]
    public void A_focused_frame_button_shows_the_focus_ring_in_the_themes_color()
    {
        var (screen, view, _) = Framed.Window(Toolbar);
        Framed.Draw(screen);
        var box = view.Frame.Toolbar!.Placed[0].Box;
        var (x, y) = ((int)box.X, (int)(box.Y + box.Height / 2));
        var focus = screen.Look.Color("focus");
        using (var before = Framed.Draw(screen))
            Assert.False(Near(focus, before.PixelAt(x, y)));
        Framed.Press(screen, Key.Tab);
        using var after = Framed.Draw(screen);
        Assert.True(Near(focus, after.PixelAt(x, y)), $"{after.PixelAt(x, y)} is not near {focus}");
    }

    [Fact]
    public void The_sidebars_buttons_are_reached_by_keyboard_and_say_what_they_do()
    {
        var (screen, view, host) = Framed.Window(Nav);
        Framed.Draw(screen);
        var nav = view.Frame.Left!;
        // As Avalonia's header: the fill button, then the fold button, then the sidebar's parts.
        Assert.Equal(["fill", "fold", "find"], screen.TabOrder().Select(Caption));
        var fill = nav.FillButton!.Value;
        var fold = nav.FoldButton!.Value;
        Assert.Equal("Fill the window", nav.Buttons.Single(b => b.Fills).TooltipAt(fill.X + 2, fill.Y + 2));
        Assert.Equal("Fold the sidebar away", nav.Buttons.Single(b => !b.Fills).TooltipAt(fold.X + 2, fold.Y + 2));
        Framed.Press(screen, Key.Tab);
        Framed.Press(screen, Key.Enter);
        Assert.True(host.State("nav")!.IsMaximized);
        Framed.Draw(screen);
        fill = nav.FillButton!.Value;
        Assert.Equal("Give the window back", nav.Buttons.Single(b => b.Fills).TooltipAt(fill.X + 2, fill.Y + 2));
        Framed.Press(screen, Key.Space);
        Assert.False(host.State("nav")!.IsMaximized);
        Framed.Press(screen, Key.Tab);
        Framed.Press(screen, Key.Enter);
        Assert.True(host.State("nav")!.IsCollapsed);
        Framed.Draw(screen);
        fold = nav.FoldButton!.Value;
        Assert.Equal("Show the sidebar", nav.Buttons.Single(b => !b.Fills).TooltipAt(fold.X + 2, fold.Y + 2));
        // The screen's tooltip finds it too.
        Assert.Equal("Show the sidebar", screen.At(fold.X + 2, fold.Y + 2)!.TooltipAt(fold.X + 2, fold.Y + 2));
    }

    [Fact]
    public void Folding_a_sidebar_leaves_whether_it_fills_the_window_as_it_was()
    {
        var (screen, view, host) = Framed.Window(Nav);
        Framed.Draw(screen);
        var nav = view.Frame.Left!;
        nav.Fill(true);
        nav.Fold(true);
        Assert.True(host.State("nav")!.IsMaximized);
        Assert.False(nav.Maximized);
        Framed.Draw(screen);
        Assert.Equal(FrameRules.StripWidth, nav.Bounds.Width);
        nav.Fold(false);
        Framed.Draw(screen);
        Assert.True(nav.Maximized);
        Assert.Equal(1000, nav.Bounds.Width);
    }

    [Fact]
    public void A_double_click_anywhere_on_the_ribbon_folds_it_but_on_a_button_only_presses_it()
    {
        var (screen, view, host) = Framed.Window(Ribbon);
        Framed.Draw(screen);
        var ribbon = view.Frame.Ribbon!;
        // On a button: it runs, the ribbon stays open.
        Framed.Click(screen, ribbon.Buttons[1].Box, clicks: 2);
        Assert.Equal(["run cut click cut"], host.Log);
        Assert.False(ribbon.Collapsed);
        // On the groups, away from the buttons: it folds.
        Framed.Click(screen, new Box(ribbon.Bounds.Right - 40, ribbon.Bounds.Y + FrameRibbon.TabsHeight + 30, 10, 10), clicks: 2);
        Assert.True(host.State("mainRibbon")!.IsCollapsed);
        Framed.Draw(screen);
        // On the strip beside the tabs: it unfolds.
        Framed.Click(screen, new Box(ribbon.Bounds.Right - 40, ribbon.Bounds.Y + 5, 10, 10), clicks: 2);
        Assert.False(host.State("mainRibbon")!.IsCollapsed);
    }

    [Fact]
    public void The_sidebars_edge_shows_the_resize_pointer()
    {
        var (screen, view, _) = Framed.Window(Nav);
        Framed.Draw(screen);
        var nav = view.Frame.Left!;
        var edge = nav.Edge!.Value;
        var (x, y) = (edge.X + edge.Width / 2, 300.0);
        Assert.Equal(Cursor.ResizeLeftRight, nav.CursorAt(x, y));
        screen.PointerMoved(x, y);
        Assert.Equal(Cursor.ResizeLeftRight, screen.Cursor);
        Assert.Null(nav.CursorAt(nav.Bounds.X + 40, y));
        nav.Fold(true);
        Framed.Draw(screen);
        Assert.Null(nav.CursorAt(nav.Bounds.Right - 2, y));
    }

    [Fact]
    public void The_menu_bar_takes_the_focus_and_the_arrows_walk_its_menus()
    {
        var (screen, view, host) = Framed.Window(Menu + """<Button name="b" at="1,1" span="3" caption="B" onclick="b" />""");
        Framed.Draw(screen);
        var bar = view.Frame.MenuBar!;
        var button = view.Find<Part>(p => p.Spec.Name == "b")!;
        screen.Focus(button);
        host.State("edit")!.IsEnabled = false;
        Assert.True(bar.FocusFirst());
        Assert.Same(bar, screen.Focused);
        Assert.Equal("File", bar.Marked!.Text("caption"));
        // Right and Left move the marked item, past the disabled one, round the ends.
        Framed.Press(screen, Key.Right);
        Assert.Equal("View", bar.Marked!.Text("caption"));
        Framed.Press(screen, Key.Right);
        Framed.Press(screen, Key.Right);
        Assert.Equal("File", bar.Marked!.Text("caption"));
        Framed.Press(screen, Key.Left);
        Assert.Equal("Help", bar.Marked!.Text("caption"));
        // Escape leaves the bar, the focus back where it was.
        Framed.Press(screen, Key.Escape);
        Assert.Same(button, screen.Focused);
        Assert.Null(bar.Marked);
        // Down opens the marked menu with its first item marked.
        bar.FocusFirst();
        Framed.Press(screen, Key.Down);
        var file = view.Frame.Menus.Lists.Single();
        Assert.Equal(("Open", 0), (file.Entries[0].Caption, file.Marked));
        Assert.Same(file, screen.Focused);
        // In the open menu, Right goes to the next top menu, Left back.
        Framed.Press(screen, Key.Right);
        Assert.Equal(["Zoom"], view.Frame.Menus.Lists.Single().Entries.Select(e => e.Caption));
        Assert.Equal(0, view.Frame.Menus.Lists.Single().Marked);
        Framed.Press(screen, Key.Left);
        Assert.Equal("Open", view.Frame.Menus.Lists.Single().Entries[0].Caption);
        // Right on an item with a sub menu opens that instead.
        Framed.Press(screen, Key.Down);
        Framed.Press(screen, Key.Right);
        Assert.Equal(2, view.Frame.Menus.Lists.Count);
        Framed.Press(screen, Key.Left);
        // Left from the first menu goes round to Help, which has no menu: the bar takes the focus with it marked.
        Framed.Press(screen, Key.Left);
        Assert.Empty(view.Frame.Menus.Lists);
        Assert.Same(bar, screen.Focused);
        Assert.Equal("Help", bar.Marked!.Text("caption"));
        // Enter on an item without a menu runs it and leaves the bar.
        Framed.Press(screen, Key.Enter);
        Assert.Equal(["run  click help"], host.Log);
        Assert.Same(button, screen.Focused);
    }

    [Fact]
    public void A_menu_bar_that_shows_nothing_does_not_take_the_focus()
    {
        var (screen, view, host) = Framed.Window(Menu);
        host.State("mainMenu")!.IsVisible = false;
        Framed.Draw(screen);
        Assert.False(view.Frame.MenuBar!.FocusFirst());
        Assert.Null(screen.Focused);
        // Tab does not reach a menu bar that is not in use.
        host.State("mainMenu")!.IsVisible = true;
        Framed.Draw(screen);
        Assert.DoesNotContain((Element)view.Frame.MenuBar, screen.TabOrder());
    }
}
