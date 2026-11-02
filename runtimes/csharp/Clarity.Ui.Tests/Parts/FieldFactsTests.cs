// What parts take from the facts of the fields they bind, the host's
// services and the person's edits of an unbound part: a table's headers in
// the fields' labels, a slider's range, a tree's parent field, a label's
// required mark; a button that opens a window; an unbound input's own text
// and tick told to the host as an edit.

namespace Clarity.Ui.Tests.Parts;

public class FieldFactsTests
{
    [Fact]
    public void A_tables_headers_are_its_fields_labels()
    {
        var host = new TestHost { Texts = { ["taskTitle"] = "Title" } };
        host.Lists["tasks"] = [new Dictionary<string, object?> { ["title"] = "Write", ["due"] = 3L }];
        host.Fields["tasks.title"] = new("@taskTitle");
        var (_, _, parts) = Made.Window("""<Table name="tasks" at="1,1" span="12" rows="4" bind="tasks" columns="title, due" />""", host);
        var table = (TablePart)parts[0];
        Assert.Equal(["Title", "due"], table.Columns.Select(c => c.Label));
    }

    [Fact]
    public void A_slider_takes_its_fields_range_and_a_label_its_required_mark()
    {
        var host = new TestHost { Values = { ["order.rating"] = 3L } };
        host.Fields["order.rating"] = new("Rating", Required: true, Min: "1", Max: "5");
        var (_, _, parts) = Made.Window("""
            <Label at="1,1" span="2" caption="Rating" for="rating" />
            <Slider name="rating" at="3,1" span="4" bind="order.rating" />
            """, host);
        Assert.True(((LabelPart)parts[0]).Required);
        var range = ((SliderPart)parts[1]).Range;
        Assert.Equal((1m, 5m), (range.Min, range.Max));
    }

    [Fact]
    public void A_trees_parent_field_is_its_entitys()
    {
        var host = new TestHost();
        host.Lists["folders"] = [new Dictionary<string, object?> { ["id"] = 1L, ["name"] = "a", ["up"] = null }, new Dictionary<string, object?> { ["id"] = 2L, ["name"] = "b", ["up"] = 1L }];
        host.Fields["folders.id"] = new("Id", Parent: "up");
        var (_, _, parts) = Made.Window("""<Tree name="folders" at="1,1" span="12" rows="4" bind="folders" show="name" />""", host);
        var tree = (TablePart)parts[0];
        Assert.Equal("up", tree.ParentField);
        tree.Toggle(0);
        Assert.Equal(1, tree.TreePlace(1)!.Value.Depth);
    }

    [Fact]
    public void A_button_and_a_menu_item_open_the_window_they_name()
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window("""<Button name="users" at="1,1" span="3" caption="Users" opens="Users" />""", host);
        ((ButtonPart)parts[0]).Click();
        Assert.Equal(["run users click Users"], host.Log);
    }

    [Fact]
    public void An_unbound_inputs_own_text_and_tick_are_edits()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""
            <Textbox name="note" at="1,1" span="4" />
            <Checkbox name="agree" at="1,2" span="4" caption="Agree" />
            <Toggle name="bold" at="1,3" span="2" caption="B" />
            """, host);
        var note = (TextBoxPart)parts[0];
        screen.Focus(note.Editor);
        screen.TextTyped("hi");
        screen.Focus(null);
        ((CheckPart)parts[1]).Tick();
        parts[2].OnClick();
        Assert.Equal(["note", "agree", "bold"], host.Edits);
        Assert.Equal("hi", host.State("note")!.Text);
    }
}
