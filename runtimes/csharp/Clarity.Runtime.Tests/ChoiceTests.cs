// The choices of a reference: an empty one for no record, then each record
// by its id (Avalonia's BindingTests.ChoicesOfRecordsStartEmptyAndHoldTheirIds).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class ChoiceTests
{
    sealed record Person(long Id, string Name);

    [Fact]
    public void ChoicesOfRecordsStartEmptyAndHoldTheirIds()
    {
        var choices = Choice.Of([new Person(1, "Ada"), new Person(2, "Grace")], p => p.Name, p => p.Id);
        Assert.Equal([null, 1L, 2L], choices.Select(c => c.Value));
        Assert.Equal(["", "Ada", "Grace"], choices.Select(c => c.Label));
    }

    [Fact]
    public void The_records_of_an_entity_are_choices_by_their_ids()
    {
        var choices = Choice.Of([new Sketched { Id = 7, Name = "box" }], s => s.Name);
        Assert.Equal([null, 7L], choices.Select(c => c.Value));
        Assert.Equal(["", "box"], choices.Select(c => c.Label));
        var cells = Choice.Of([new Sketched { Id = 7, Name = "box", X = 3 }], s => s.Name, s => [s.Name, s.X.ToString()]);
        Assert.Equal(7L, Assert.Single(cells).Value);
        Assert.Equal(["box", "3"], Assert.Single(cells).Cells!);
    }
}
