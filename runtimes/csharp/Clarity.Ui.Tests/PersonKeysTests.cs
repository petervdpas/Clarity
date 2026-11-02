// The keys a person set (DESIGN 5.28) count over the designed ones: a
// window's button by `Window.name`, a command of the window's own frame
// the same, a command of the app's frame by its name. A command the
// person gave another key no longer answers its designed one.

namespace Clarity.Ui.Tests;

public class PersonKeysTests
{
    [Fact]
    public void A_buttons_key_set_by_the_person_runs_it_and_its_designed_key_no_more()
    {
        var host = new TestHost();
        host.PersonKeys["Card.saveButton"] = ["Ctrl+K"];
        var (screen, _, _) = Framed.Window("""<Button at="1,1" span="2" name="saveButton" caption="Save" key="Ctrl+S" onclick="save" />""", host: host, name: "Card");
        Framed.Press(screen, Key.S, Modifiers.Control);
        Assert.Empty(host.Log);
        Framed.Press(screen, Key.K, Modifiers.Control);
        Assert.Equal(["run saveButton click save"], host.Log);
    }

    [Fact]
    public void A_frame_commands_key_is_the_persons_by_its_place()
    {
        var host = new TestHost();
        host.PersonKeys["print"] = ["Ctrl+J"];
        host.PersonKeys["Card.find"] = ["F3"];
        var (screen, _, _) = Framed.Window(
            """<Toolbar name="own"><Item name="find" caption="Find" onclick="find" /></Toolbar>""",
            """<Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /></Item></Menu>""",
            host, name: "Card");
        Framed.Press(screen, Key.P, Modifiers.Control);
        Assert.Empty(host.Log);
        Framed.Press(screen, Key.J, Modifiers.Control);
        Framed.Press(screen, Key.F3);
        Assert.Equal(["run print click print", "run find click find"], host.Log);
    }

    [Fact]
    public void A_menu_shows_the_key_the_person_set()
    {
        var host = new TestHost();
        host.PersonKeys["print"] = ["Ctrl+J"];
        var (_, view, _) = Framed.Window("", """<Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /><Item name="quit" caption="Quit" key="Ctrl+Q" onclick="quit" /></Item></Menu>""", host);
        var entries = view.Frame.EntriesOf(view.Frame.Shown.Menu!.Value.Items[0].Items);
        Assert.Equal(["Ctrl+J", "Ctrl+Q"], entries.Select(e => e.Key));
    }
}
