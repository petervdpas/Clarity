// A button's shortcut as the Avalonia KeyTests had it: a second key presses
// the button anywhere in the window, but not while the button is disabled,
// nor once it no longer shows.

namespace Clarity.Ui.Tests.Parts;

public class KeyParityTests
{
    static (Screen Screen, WindowView View, TestHost Host) Shown(string parts)
    {
        var host = new TestHost();
        var spec = WindowMarkup.Read($"<Window name=\"Keys\">{parts}</Window>");
        var view = new WindowView(spec, host);
        var screen = new Screen(view, ThemeLook.Clarity(dark: false)) { Keys = view.KeyPressed };
        screen.Resize(800, 400, 1);
        return (screen, view, host);
    }

    static void Press(Screen screen, Key key, Modifiers held = Modifiers.None) => screen.KeyPressed(new KeyInput(key, held));

    [Fact]
    public void A_disabled_or_hidden_buttons_shortcut_presses_nothing()
    {
        var (screen, view, host) = Shown("""
            <Textbox name="note" at="1,1" span="6" />
            <Button name="save" at="7,1" span="3" caption="Save" key="Ctrl+S" onclick="save" />
            """);
        screen.Focus(((TextBoxPart)view.Parts[0]).Editor);
        host.State("save")!.IsEnabled = false;
        Press(screen, Key.S, Modifiers.Control);
        Assert.Empty(host.Log);
        host.State("save")!.IsEnabled = true;
        host.State("save")!.IsVisible = false;
        Press(screen, Key.S, Modifiers.Control);
        Assert.Empty(host.Log);
        host.State("save")!.IsVisible = true;
        Press(screen, Key.S, Modifiers.Control);
        Assert.Equal(["run save click save"], host.Log);
    }
}
