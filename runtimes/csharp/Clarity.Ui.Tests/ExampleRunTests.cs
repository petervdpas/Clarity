// What running the C# examples (todo, modelling, noughts) showed, each kept
// from coming back: a dialog is as wide as the captions in its containers
// need (a settings form's labels), as Avalonia's SizeToContent made it; a
// sidebar that fills the window and gives it back leaves the desktop window
// as wide as it was (noughts grew by the sidebar's width each time).

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class ExampleRunTests
{
    sealed class PlainViewModel() : WindowViewModel(null)
    {
        public override string Title => "Plain";
    }

    static readonly string LongCaption = string.Join(" ", Enumerable.Repeat("a long caption", 4));

    [Fact]
    public void A_dialog_is_as_wide_as_the_labels_of_its_form_need()
    {
        var host = new TestHost();
        var form = new WindowView(WindowMarkup.Read($"""
            <Window name="D">
              <Form name="settingsForm" at="1,1" span="12" rows="2" labels="3">
                <Label name="colorLabel" at="1,1" span="3" caption="{LongCaption}" for="colorBox" />
                <Textbox name="colorBox" at="4,1" span="6" />
              </Form>
            </Window>
            """), host);
        var flat = new WindowView(WindowMarkup.Read($"""<Window name="D"><Label at="1,1" span="3" caption="{LongCaption}" /></Window>"""), host);
        var width = WindowStack.DialogSize(form, new PlainViewModel()).Width;
        Assert.True(width > WindowStack.DialogWidth);
        Assert.Equal(WindowStack.DialogSize(flat, new PlainViewModel()).Width, width, 1);
    }

    [Fact]
    public void A_dialog_is_as_wide_as_a_tab_that_is_not_open_needs()
    {
        var host = new TestHost();
        var tabs = new WindowView(WindowMarkup.Read($"""
            <Window name="D">
              <Tabs name="pages" at="1,1" span="12" rows="3">
                <Tab title="One"><Label at="1,1" span="3" caption="Short" /></Tab>
                <Tab title="Two"><Label at="1,1" span="3" caption="{LongCaption}" /></Tab>
              </Tabs>
            </Window>
            """), host);
        Assert.True(WindowStack.DialogSize(tabs, new PlainViewModel()).Width > WindowStack.DialogWidth);
    }

    [Fact]
    public void A_sidebar_that_fills_the_window_and_gives_it_back_takes_the_same_room()
    {
        var spec = WindowMarkup.Read("""<Window name="Game"><Sidebar side="left" name="help" width="320" collapsible="true" maximizable="true"><Text name="rules" at="1,1" span="12" /></Sidebar><Button name="go" at="1,1" span="12" /></Window>""");
        var view = new WindowView(spec, new TestHost());
        var screen = new Screen(view, ThemeLook.Clarity(dark: false));
        screen.Resize(697, 510, 1);
        Assert.Equal(320, view.SidebarsWidth);
        view.Frame.Left!.Fill(true);
        Assert.Equal(320, view.SidebarsWidth);
        view.Frame.Left!.Fill(false);
        Assert.Equal(320, view.SidebarsWidth);
    }
}
