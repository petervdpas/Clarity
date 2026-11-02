// A window's menu bar (DESIGN 5.16): a press opens a menu below its
// caption, an item runs its handler or opens the window it targets, a sub
// menu opens beside its item, separators and keys show. A named item hides,
// greys and shows a check mark as its part state says (conformance/overlays.json
// `item`); the `language` and `theme` targets pick in place from their
// choices, the one shown checked. An item's key runs it while its menu is
// closed, unless code hid or disabled it.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class FrameMenuTests
{
    const string Menu = """
        <Menu name="mainMenu">
          <Item caption="File">
            <Item caption="Open" key="Ctrl+O" onclick="open" />
            <Item kind="separator" />
            <Item caption="Recent">
              <Item caption="Shop.db" onclick="openRecent" />
            </Item>
            <Item name="wrap" caption="Wrap lines" onclick="toggleWrap" />
            <Item name="customers" caption="Customers" opens="Customers" />
          </Item>
          <Item caption="View">
            <Item caption="Language" opens="language" />
            <Item caption="Theme" opens="theme" />
          </Item>
          <Item caption="Help" onclick="help" />
        </Menu>
        """;

    static (Screen Screen, WindowView View, TestHost Host) Shown() => Framed.Window(Menu);

    static Box Top(WindowView view, string caption) => view.Frame.MenuBar!.Placed.First(p => p.Item.Text("caption") == caption).Box;

    static FrameMenuList Open(Screen screen, WindowView view, string caption)
    {
        Framed.Draw(screen);
        Framed.Click(screen, Top(view, caption));
        return view.Frame.Menus.Lists[^1];
    }

    static void Pick(Screen screen, FrameMenuList list, string caption) =>
        Framed.Click(screen, list.RowOf(list.Entries.ToList().FindIndex(e => e.Caption == caption)));

    [Fact]
    public void A_press_opens_a_menu_below_its_caption_and_another_closes_it()
    {
        var (screen, view, _) = Shown();
        var list = Open(screen, view, "File");
        Assert.Single(screen.Overlays);
        Assert.Equal(Top(view, "File").Bottom + 1, list.Bounds.Y);
        Assert.Equal(["Open", "", "Recent", "Wrap lines", "Customers"], list.Entries.Select(e => e.Caption));
        Assert.True(list.Entries[1].Separator);
        Assert.Equal("Ctrl+O", list.Entries[0].Key);
        Framed.Click(screen, Top(view, "File"));
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void An_item_runs_its_handler_and_the_menu_closes()
    {
        var (screen, view, host) = Shown();
        Pick(screen, Open(screen, view, "File"), "Open");
        Assert.Equal(["run  click open"], host.Log);
        Assert.Empty(screen.Overlays);
        Assert.False(view.Frame.Menus.IsOpen);
    }

    [Fact]
    public void An_item_opens_the_window_it_targets()
    {
        var (screen, view, host) = Shown();
        Pick(screen, Open(screen, view, "File"), "Customers");
        Assert.Equal(["run customers click Customers"], host.Log);
    }

    [Fact]
    public void A_top_item_without_items_runs_as_a_click()
    {
        var (screen, view, host) = Shown();
        Framed.Draw(screen);
        Framed.Click(screen, Top(view, "Help"));
        Assert.Equal(["run  click help"], host.Log);
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void A_sub_menu_opens_beside_its_item()
    {
        var (screen, view, host) = Shown();
        var list = Open(screen, view, "File");
        var recent = list.RowOf(2);
        screen.PointerMoved(recent.X + 10, recent.Y + 5);
        Assert.Equal(2, view.Frame.Menus.Lists.Count);
        var sub = view.Frame.Menus.Lists[1];
        Assert.True(sub.Bounds.X >= list.Bounds.Right - 4);
        Pick(screen, sub, "Shop.db");
        Assert.Equal(["run  click openRecent"], host.Log);
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void While_a_menu_is_open_the_pointer_over_another_opens_it()
    {
        var (screen, view, _) = Shown();
        Open(screen, view, "File");
        var viewMenu = Top(view, "View");
        screen.PointerMoved(viewMenu.X + 5, viewMenu.Y + 5);
        Assert.Single(screen.Overlays);
        Assert.Equal(["Language", "Theme"], view.Frame.Menus.Lists[0].Entries.Select(e => e.Caption));
    }

    [Fact]
    public void The_arrow_keys_and_Enter_pick_an_item_and_Escape_closes()
    {
        var (screen, view, host) = Shown();
        Open(screen, view, "File");
        Framed.Press(screen, Key.Down);
        Framed.Press(screen, Key.Enter);
        Assert.Equal(["run  click open"], host.Log);
        Open(screen, view, "File");
        Framed.Press(screen, Key.Escape);
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void A_press_outside_closes_every_menu()
    {
        var (screen, view, host) = Shown();
        var list = Open(screen, view, "File");
        screen.PointerMoved(list.RowOf(2).X + 10, list.RowOf(2).Y + 5);
        Assert.Equal(2, screen.Overlays.Count);
        screen.PointerPressed(900, 500, PointerButton.Left);
        Assert.Empty(screen.Overlays);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_named_item_shows_and_takes_clicks_as_its_state_says()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "overlays.json"))).RootElement;
        foreach (var c in file.GetProperty("item").EnumerateArray())
        {
            var (screen, view, host) = Shown();
            var part = c.GetProperty("part");
            if (part.TryGetProperty("visible", out var visible))
                host.State("wrap")!.IsVisible = visible.GetBoolean();
            if (part.TryGetProperty("enabled", out var enabled))
                host.State("wrap")!.IsEnabled = enabled.GetBoolean();
            var list = Open(screen, view, "File");
            var at = list.Entries.ToList().FindIndex(e => e.Caption == "Wrap lines");
            Assert.Equal(c.GetProperty("shown").GetBoolean(), at >= 0);
            if (at >= 0)
                Framed.Click(screen, list.RowOf(at));
            Assert.Equal(c.GetProperty("clicks").GetBoolean(), host.Log.Contains("run wrap click toggleWrap"));
        }
    }

    [Fact]
    public void A_checked_item_shows_its_check_mark()
    {
        var (screen, view, host) = Shown();
        host.State("wrap")!.IsChecked = true;
        var list = Open(screen, view, "File");
        Assert.True(list.Entries.Single(e => e.Caption == "Wrap lines").Checked);
        Assert.False(list.Entries.Single(e => e.Caption == "Open").Checked);
    }

    [Fact]
    public void The_language_item_picks_in_place_the_one_shown_checked()
    {
        var (screen, view, host) = Shown();
        var chosen = new List<string>();
        view.Frame.Language = new FramePicker(() => [("en", "English", true), ("nl", "Nederlands", false)], chosen.Add);
        var list = Open(screen, view, "View");
        var language = list.RowOf(0);
        screen.PointerMoved(language.X + 10, language.Y + 5);
        var choices = view.Frame.Menus.Lists[1];
        Assert.Equal([("English", true), ("Nederlands", false)], choices.Entries.Select(e => (e.Caption, e.Checked)));
        Pick(screen, choices, "Nederlands");
        Assert.Equal(["nl"], chosen);
        Assert.Empty(screen.Overlays);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void The_theme_item_picks_the_theme_through_the_runtime()
    {
        var (screen, view, _) = Shown();
        Themes.Start([new ThemeOffer("clarity", "Clarity"), new ThemeOffer("night", "Night")]);
        var list = Open(screen, view, "View");
        screen.PointerMoved(list.RowOf(1).X + 10, list.RowOf(1).Y + 5);
        var choices = view.Frame.Menus.Lists[1];
        Assert.Equal([("Clarity", true), ("Night", false)], choices.Entries.Select(e => (e.Caption, e.Checked)));
        Pick(screen, choices, "Night");
        Assert.Equal("night", Themes.Current);
    }

    [Fact]
    public void An_items_key_runs_it_while_its_menu_is_closed()
    {
        var (screen, _, host) = Shown();
        Framed.Press(screen, Key.O, Modifiers.Control);
        Assert.Equal(["run  click open"], host.Log);
    }

    [Fact]
    public void What_code_hid_or_disabled_does_not_answer_its_key()
    {
        var (screen, _, host) = Framed.Window("""
            <Menu name="mainMenu">
              <Item caption="File">
                <Item name="open" caption="Open" key="Ctrl+O" onclick="open" />
                <Item name="save" caption="Save" key="Ctrl+S" onclick="save" />
              </Item>
            </Menu>
            """);
        host.State("open")!.IsEnabled = false;
        host.State("save")!.IsVisible = false;
        Framed.Press(screen, Key.O, Modifiers.Control);
        Framed.Press(screen, Key.S, Modifiers.Control);
        host.State("save")!.IsVisible = true;
        host.State("mainMenu")!.IsVisible = false;
        Framed.Press(screen, Key.S, Modifiers.Control);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_hidden_menu_bar_takes_no_room_and_a_disabled_one_no_press()
    {
        var (screen, view, host) = Shown();
        host.State("mainMenu")!.IsEnabled = false;
        Framed.Draw(screen);
        Framed.Click(screen, Top(view, "Help"));
        Assert.Empty(host.Log);
        host.State("mainMenu")!.IsVisible = false;
        Framed.Draw(screen);
        Assert.Equal(0, view.Frame.Body.Y);
    }

    [Fact]
    public void An_apps_menu_shows_in_a_window_without_its_own_and_its_keys_answer()
    {
        var (screen, view, host) = Framed.Window("", Menu);
        Framed.Draw(screen);
        Assert.NotNull(view.Frame.MenuBar);
        Assert.True(view.Frame.Shown.Menu!.Inherited);
        Framed.Press(screen, Key.O, Modifiers.Control);
        Assert.Equal(["run  click open"], host.Log);
    }
}
