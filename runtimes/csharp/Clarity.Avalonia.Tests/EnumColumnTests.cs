// An editable enum column of a table, written in XAML as generated code
// writes it, compiles and offers the enum's values.

using Avalonia.Headless.XUnit;
using Avalonia.LogicalTree;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class EnumColumnTests
{
    [AvaloniaFact]
    public void An_enum_columns_choices_come_from_xaml()
    {
        var view = new EnumColumnView();
        var table = view.GetLogicalDescendants().OfType<ClarityTable>().Single();
        var choices = table.Columns[0].Choices!.Cast<Choice>().ToList();
        Assert.Equal(["Small", "Large"], choices.Select(c => c.Label));
    }
}
