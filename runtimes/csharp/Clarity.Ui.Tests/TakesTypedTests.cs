// A key that runs a command (a button's Ctrl+S, the default button's Enter,
// a menu's key) first has the input with the focus take what is typed in it,
// as a click does by moving the focus: the command sees the new value.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class TakesTypedTests
{
    sealed class Typing(List<string> log) : Element, ITakesTyped
    {
        public override bool Focusable => true;

        public void Add(Element child) => AddChild(child);

        public void TakeTyped() => log.Add("taken");
    }

    [Fact]
    public void The_input_the_focus_is_in_takes_what_is_typed()
    {
        var log = new List<string>();
        var input = new Typing(log);
        var inner = new Hooked();
        input.Add(inner);
        TakesTyped.From(inner);
        Assert.Equal(["taken"], log);
        TakesTyped.From(null);
        TakesTyped.From(new Hooked());
        Assert.Single(log);
    }

    [Fact]
    public void A_button_s_key_has_the_focused_input_take_its_text_first()
    {
        var host = new TestHost();
        var log = host.Log;
        var spec = WindowMarkup.Read("""<Window name="Keys"><Button name="save" at="1,1" span="2" key="Ctrl+S" onclick="save" /></Window>""");
        var view = new WindowView(spec, host);
        var screen = new Screen(view, ThemeLook.Clarity(dark: false)) { Keys = view.KeyPressed };
        screen.Resize(800, 400, 1);
        // An input with the focus, which an overlay holds here.
        var input = new Typing(log);
        screen.Open(new Overlay(input, new Box(0, 300, 100, 30)));
        screen.Focus(input);
        screen.KeyPressed(new KeyInput(Key.S, Modifiers.Control));
        Assert.Equal("taken", log[0]);
        Assert.StartsWith("run save click save", log[1]);
    }
}
