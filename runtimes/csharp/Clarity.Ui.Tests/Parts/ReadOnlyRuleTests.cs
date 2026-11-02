// One read-only rule for every part: readonly="true" in the markup, or its
// state set read-only by code. A named part has a state, and that state does
// not undo what the markup says.

namespace Clarity.Ui.Tests.Parts;

public class ReadOnlyRuleTests
{
    [Theory]
    [InlineData("""<Textbox at="1,1" span="4" name="p" readonly="true" />""")]
    [InlineData("""<Number at="1,1" span="4" name="p" readonly="true" />""")]
    [InlineData("""<Checkbox at="1,1" span="4" name="p" readonly="true" />""")]
    [InlineData("""<Slider at="1,1" span="4" name="p" readonly="true" />""")]
    [InlineData("""<Folder at="1,1" span="4" name="p" readonly="true" />""")]
    [InlineData("""<Date at="1,1" span="4" name="p" readonly="true" />""")]
    public void A_named_part_marked_read_only_in_its_markup_is_read_only(string markup)
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window(markup, host);
        Assert.IsNotType<MissingPart>(parts[0]);
        Assert.True(parts[0].ReadOnly);
    }

    [Fact]
    public void Code_makes_a_part_read_only_by_its_state()
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="p" />""", host);
        Assert.False(parts[0].ReadOnly);
        host.States["p"].IsReadOnly = true;
        Assert.True(parts[0].ReadOnly);
    }

    [Fact]
    public void A_named_read_only_input_takes_no_typing()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="p" readonly="true" />""", host);
        var box = (TextBoxPart)parts[0];
        screen.Focus(box.Editor);
        screen.TextTyped("x");
        Assert.Equal("", box.Editor.Text);
    }
}
