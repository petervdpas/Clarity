// A label shows its caption, as code set it or as written and translated,
// with a mark when its input's field is required; a help text shows its
// input's error in the error color, else its own text or its input's help.

using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class LabelPartTests
{
    [Fact]
    public void A_caption_is_translated_and_code_may_set_it()
    {
        var host = new TextFactsHost { Texts = { ["nameCaption"] = "Name" } };
        var (_, _, parts) = Made.Window("""<Label at="1,1" span="3" name="nameLabel" caption="@nameCaption" />""", host);
        var label = (LabelPart)parts[0];
        Assert.Equal("Name", label.Text);
        host.States["nameLabel"].CaptionText = Clarity.Runtime.Text.Plain("Full name");
        Assert.Equal("Full name", label.Text);
    }

    [Fact]
    public void A_label_marks_the_input_of_a_required_field()
    {
        var host = new TextFactsHost();
        host.Required.Add("order.number");
        var (screen, _, parts) = Made.Window("""
            <Label at="1,1" span="3" caption="Number" for="number" />
            <Textbox at="4,1" span="3" name="number" bind="order.number" />
            <Label at="1,2" span="3" caption="Note" for="note" />
            <Textbox at="4,2" span="3" name="note" bind="order.note" />
            """, host);
        var required = (LabelPart)parts[0];
        Assert.True(required.Required);
        Assert.False(((LabelPart)parts[2]).Required);
        using var pixels = Shot.Of(screen, 800, 400);
        Assert.True(TextEditorTests.Inked(pixels, required.Bounds, screen.Look.Color("error")));
        Assert.False(TextEditorTests.Inked(pixels, parts[2].Bounds, screen.Look.Color("error")));
    }

    [Fact]
    public void Help_shows_its_inputs_error_in_the_error_color_else_its_text()
    {
        var host = new TextFactsHost();
        var (screen, _, parts) = Made.Window("""
            <Textbox at="1,1" span="4" name="email" />
            <Help at="1,2" span="4" caption="We never share it" for="email" />
            """, host);
        var help = (HelpPart)parts[1];
        Assert.Equal("We never share it", help.Text);
        Assert.False(help.ShowsError);
        host.States["email"].ErrorText = Clarity.Runtime.Text.Plain("Not an address");
        Assert.Equal("Not an address", help.Text);
        using var pixels = Shot.Of(screen, 800, 400);
        Assert.True(TextEditorTests.Inked(pixels, help.Bounds, screen.Look.Color("error")));
    }

    [Fact]
    public void Help_without_text_of_its_own_shows_its_inputs_help()
    {
        var host = new TextFactsHost();
        host.States["email"] = new PartState("email", help: "An address we can write to");
        var (_, _, parts) = Made.Window("""
            <Textbox at="1,1" span="4" name="email" />
            <Help at="1,2" span="4" for="email" />
            """, host);
        Assert.Equal("An address we can write to", ((HelpPart)parts[1]).Text);
    }
}
