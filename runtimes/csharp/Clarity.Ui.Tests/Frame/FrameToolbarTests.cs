// A window's toolbar: its buttons in a row, each running its handler or
// opening the window it targets; the `theme` and `language` targets open
// their choices below the button. The toolbar and each button hide and
// grey as their part state says, and a button's key runs it.

namespace Clarity.Ui.Tests;

public class FrameToolbarTests
{
    const string Toolbar = """
        <Toolbar name="tools">
          <Item name="save" caption="Save" icon="save" key="Ctrl+S" onclick="save" />
          <Item kind="separator" />
          <Item name="users" caption="Users" opens="Users" />
          <Item caption="Language" opens="language" />
        </Toolbar>
        """;

    static Box Button(WindowView view, string caption) => view.Frame.Toolbar!.Placed.First(p => p.Item.Text("caption") == caption).Box;

    [Fact]
    public void Its_buttons_sit_in_a_row_and_run_when_clicked()
    {
        var (screen, view, host) = Framed.Window(Toolbar);
        host.Icons["save"] = "M5 5h14v14H5z";
        Framed.Draw(screen);
        var placed = view.Frame.Toolbar!.Placed;
        Assert.Equal(4, placed.Count);
        Assert.True(placed[1].Box.X > placed[0].Box.Right && placed[2].Box.X > placed[1].Box.Right);
        Framed.Click(screen, Button(view, "Save"));
        Framed.Click(screen, Button(view, "Users"));
        Assert.Equal(["run save click save", "run users click Users"], host.Log);
    }

    [Fact]
    public void The_language_button_opens_its_choices_below_it()
    {
        var (screen, view, host) = Framed.Window(Toolbar);
        var chosen = new List<string>();
        view.Frame.Language = new FramePicker(() => [("en", "English", false), ("de", "Deutsch", true)], chosen.Add);
        Framed.Draw(screen);
        var button = Button(view, "Language");
        Framed.Click(screen, button);
        var list = view.Frame.Menus.Lists.Single();
        Assert.Equal(button.Bottom + 1, list.Bounds.Y);
        Assert.Equal([("English", false), ("Deutsch", true)], list.Entries.Select(e => (e.Caption, e.Checked)));
        Framed.Click(screen, list.RowOf(0));
        Assert.Equal(["en"], chosen);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_hidden_button_leaves_the_row_and_a_disabled_one_takes_no_click()
    {
        var (screen, view, host) = Framed.Window(Toolbar);
        host.State("users")!.IsVisible = false;
        host.State("save")!.IsEnabled = false;
        Framed.Draw(screen);
        Assert.DoesNotContain(view.Frame.Toolbar!.Placed, p => p.Item.Text("name") == "users");
        Framed.Click(screen, Button(view, "Save"));
        Framed.Press(screen, Key.S, Modifiers.Control);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_disabled_toolbar_takes_no_click_and_a_hidden_one_no_room()
    {
        var (screen, view, host) = Framed.Window(Toolbar);
        host.State("tools")!.IsEnabled = false;
        Framed.Draw(screen);
        Framed.Click(screen, Button(view, "Save"));
        Framed.Press(screen, Key.S, Modifiers.Control);
        Assert.Empty(host.Log);
        host.State("tools")!.IsVisible = false;
        Framed.Draw(screen);
        Assert.Equal(0, view.Frame.Body.Y);
    }

    [Fact]
    public void A_buttons_key_runs_it()
    {
        var (screen, _, host) = Framed.Window(Toolbar);
        Framed.Press(screen, Key.S, Modifiers.Control);
        Assert.Equal(["run save click save"], host.Log);
    }
}

public class FrameTooltipTests
{
    [Fact]
    public void A_toolbar_and_a_ribbon_button_say_their_caption_and_keys()
    {
        var host = new TestHost();
        host.PersonKeys["Main.find"] = ["F3"];
        var (screen, view, _) = Framed.Window("""
            <Toolbar name="tools"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /><Item name="find" caption="Find" key="Ctrl+F" onclick="find" /></Toolbar>
            <Ribbon name="r"><Tab title="Home" name="home"><Group title="G"><Item name="paste" caption="Paste" onclick="paste" /></Group></Tab></Ribbon>
            """, host: host);
        Framed.Draw(screen);
        var tools = view.Frame.Toolbar!;
        var print = tools.Placed[0].Box;
        Assert.Equal("Print (Ctrl+P)", tools.TooltipAt(print.X + 2, print.Y + 2));
        var find = tools.Placed[1].Box;
        Assert.Equal("Find (F3)", tools.TooltipAt(find.X + 2, find.Y + 2));
        var paste = view.Frame.Ribbon!.Buttons[0].Box;
        Assert.Equal("Paste", view.Frame.Ribbon.TooltipAt(paste.X + 2, paste.Y + 2));
        // The screen shows it after the pointer rests.
        var now = new DateTime(2026, 11, 2);
        screen.Now = () => now;
        screen.PointerMoved(print.X + 2, print.Y + 2);
        screen.WakeDue(now + Screen.TipPause);
        Assert.Equal("Print (Ctrl+P)", screen.Tip?.Text);
    }
}
