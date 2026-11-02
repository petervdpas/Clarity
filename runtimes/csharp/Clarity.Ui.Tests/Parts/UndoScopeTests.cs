// Only a part the project gave an undo scope makes edits (Avalonia's
// Edits.Scope, clarity_spec undo_scope): an accordion, tabs or a list that
// only selects has none in its markup, so what the person does there is no
// edit Ctrl+Z takes back. A part the markup does not know is its own scope.

using Clarity.Runtime;
using Clarity.Ui.Tests.Generated;

namespace Clarity.Ui.Tests;

public class UndoScopeTests
{
    [Fact]
    public void Only_a_part_with_an_undo_scope_makes_edits()
    {
        var spec = WindowMarkup.Read("""
            <Window name="Orders">
              <List name="people" at="1,1" span="6" bind="people" />
              <Textbox name="nameBox" at="1,2" span="6" bind="name" undoScope="nameBox" />
              <Accordion name="fold" at="1,3" span="6" rows="3"><Section title="One" /></Accordion>
            </Window>
            """);
        var host = new ViewModelHost(new OrdersViewModel(), spec);
        Assert.Null(host.Scope("people"));
        Assert.Null(host.Scope("fold"));
        Assert.Equal("nameBox", host.Scope("nameBox"));
        Assert.Equal("madeByCode", host.Scope("madeByCode"));
    }
}
