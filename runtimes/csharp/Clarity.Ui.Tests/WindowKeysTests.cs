// The window's own keys, which the screen asks before the part with the
// focus (Screen.Keys): a part's or a menu item's shortcut, Enter for the
// default button and Escape for the cancel button, as the shared screen
// does them (runtimes/web/src/kinds/Button.svelte, screen/shortcut.ts).

namespace Clarity.Ui.Tests;

public class WindowKeysTests
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
    public void A_shortcut_matches_its_modifiers_exactly_and_leaves_typing_alone()
    {
        Assert.True(Shortcuts.Matches("Ctrl+S", new KeyInput(Key.S, Modifiers.Control), typing: true));
        Assert.False(Shortcuts.Matches("Ctrl+S", new KeyInput(Key.S, Modifiers.None), typing: false));
        Assert.False(Shortcuts.Matches("Ctrl+S", new KeyInput(Key.S, Modifiers.Control | Modifiers.Shift), typing: false));
        Assert.True(Shortcuts.Matches("Ctrl+Shift+F2", new KeyInput(Key.F2, Modifiers.Control | Modifiers.Shift), typing: false));
        Assert.True(Shortcuts.Matches("7", new KeyInput(Key.D7, Modifiers.None), typing: false));
        Assert.False(Shortcuts.Matches("7", new KeyInput(Key.D7, Modifiers.None), typing: true));
        Assert.True(Shortcuts.Matches("F5", new KeyInput(Key.F5, Modifiers.None), typing: true));
        var (modifiers, key) = Shortcuts.Parts("Ctrl++");
        Assert.Equal(["Ctrl"], modifiers);
        Assert.Equal("+", key);
    }

    [Theory]
    [InlineData("+", Key.Plus)]
    [InlineData("-", Key.Minus)]
    [InlineData("*", Key.Multiply)]
    [InlineData("/", Key.Slash)]
    [InlineData(".", Key.Period)]
    [InlineData(",", Key.Comma)]
    [InlineData("=", Key.Equal)]
    [InlineData(";", Key.Semicolon)]
    [InlineData("F2", Key.F2)]
    [InlineData("Up", Key.Up)]
    [InlineData("§", Key.Other)]
    public void A_shortcuts_key_is_clarity_uis_key(string written, Key key) => Assert.Equal(key, Shortcuts.KeyOf(written));

    [Fact]
    public void A_symbol_shortcut_matches_from_the_keyboard_or_the_number_pad()
    {
        // The plus is Shift and = on many keyboards, the number pad's plus has no Shift: both are Plus.
        Assert.True(Shortcuts.Matches("Ctrl++", new KeyInput(Key.Plus, Modifiers.Control | Modifiers.Shift), typing: false));
        Assert.True(Shortcuts.Matches("Ctrl++", new KeyInput(Key.Plus, Modifiers.Control), typing: true));
        Assert.True(Shortcuts.Matches("Ctrl+-", new KeyInput(Key.Minus, Modifiers.Control), typing: false));
        Assert.True(Shortcuts.Matches("Ctrl+.", new KeyInput(Key.Period, Modifiers.Control), typing: false));
        Assert.True(Shortcuts.Matches("Ctrl+,", new KeyInput(Key.Comma, Modifiers.Control), typing: false));
        Assert.False(Shortcuts.Matches("Ctrl++", new KeyInput(Key.Minus, Modifiers.Control), typing: false));
        // A symbol with Ctrl is no plus without it, and a typed symbol is typing in an input.
        Assert.False(Shortcuts.Matches("+", new KeyInput(Key.Plus, Modifiers.Control), typing: false));
        Assert.True(Shortcuts.Matches("+", new KeyInput(Key.Plus, Modifiers.Shift), typing: false));
        Assert.False(Shortcuts.Matches("+", new KeyInput(Key.Plus, Modifiers.None), typing: true));
        Assert.True(Shortcuts.Matches("*", new KeyInput(Key.Multiply, Modifiers.None), typing: false));
    }

    [Fact]
    public void A_buttons_shortcut_clicks_it_wherever_the_focus_is()
    {
        var (screen, view, host) = Shown("""
            <Textbox name="note" at="1,1" span="6" />
            <Button name="save" at="7,1" span="3" caption="Save" key="Ctrl+S" onclick="save" />
            <Button name="hidden" at="10,1" span="3" caption="Hidden" key="F5" onclick="never" />
            """);
        host.State("hidden")!.IsVisible = false;
        screen.Focus(((TextBoxPart)view.Parts[0]).Editor);
        Press(screen, Key.S, Modifiers.Control);
        Press(screen, Key.F5);
        Assert.Equal(["run save click save"], host.Log);
    }

    [Fact]
    public void Enter_presses_the_default_button_and_Escape_the_cancel_button()
    {
        var (screen, view, host) = Shown("""
            <Textbox name="name" at="1,1" span="6" />
            <Memo name="story" at="1,2" span="6" rows="3" />
            <Button name="ok" at="7,1" span="3" caption="OK" default="true" onclick="ok" />
            <Button name="cancel" at="10,1" span="3" caption="Cancel" cancel="true" onclick="cancel" />
            """);
        screen.Focus(((TextBoxPart)view.Parts[0]).Editor);
        Press(screen, Key.Enter);
        Press(screen, Key.Escape);
        // Enter in a memo is a new line.
        screen.Focus(((TextBoxPart)view.Parts[1]).Editor);
        Press(screen, Key.Enter);
        Assert.Equal(["run ok click ok", "run cancel click cancel"], host.Log);
        Assert.Equal("\n", ((TextBoxPart)view.Parts[1]).Editor.Text);
    }

    [Fact]
    public void A_menu_items_shortcut_runs_its_handler()
    {
        var (screen, _, host) = Shown("""
            <Button name="more" at="1,1" span="3" caption="More" split="true">
              <Menu><Item caption="Print" key="Ctrl+P" onclick="print" /></Menu>
            </Button>
            """);
        Press(screen, Key.P, Modifiers.Control);
        Assert.Equal(["run more click print"], host.Log);
    }

    [Fact]
    public void A_dialog_over_the_window_keeps_its_keys()
    {
        var (screen, _, host) = Shown("""<Button name="ok" at="1,1" span="3" caption="OK" default="true" onclick="ok" />""");
        screen.Open(new Overlay(new Plate("panel"), new Box(0, 0, 100, 100)) { Modal = true });
        Press(screen, Key.Enter);
        Assert.Empty(host.Log);
    }
}
