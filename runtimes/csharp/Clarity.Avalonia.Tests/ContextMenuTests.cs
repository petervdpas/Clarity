// A context menu on a table (DESIGN 5.16): a right click chooses the row
// under the pointer first, as a click does, so the menu's handler finds it
// as the table's selected row.

using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Threading;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class ContextMenuTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "overlays.json"))).RootElement;

    /// A part's state as the cases give it, on a PartState made as the generator makes it.
    static PartState State(JsonElement part, bool popover)
    {
        var state = popover ? new PartState("filters") { IsVisible = false } : new PartState("deleteItem");
        if (part.TryGetProperty("visible", out var visible))
            state.IsVisible = visible.GetBoolean();
        if (part.TryGetProperty("enabled", out var enabled))
            state.IsEnabled = enabled.GetBoolean();
        return state;
    }

    [AvaloniaFact]
    public void A_popover_shows_once_opened_and_an_item_as_its_state_says()
    {
        foreach (var c in File.GetProperty("popover").EnumerateArray())
            Assert.Equal(c.GetProperty("shown").GetBoolean(), State(c.GetProperty("part"), popover: true).IsVisible);
        foreach (var c in File.GetProperty("item").EnumerateArray())
        {
            // A MenuItem bound to the state, as the generator writes it for a named item.
            var state = State(c.GetProperty("part"), popover: false);
            var item = new MenuItem { DataContext = state };
            item.Bind(Visual.IsVisibleProperty, new global::Avalonia.Data.Binding(nameof(PartState.IsVisible)));
            item.Bind(InputElement.IsEnabledProperty, new global::Avalonia.Data.Binding(nameof(PartState.IsEnabled)));
            Assert.Equal(c.GetProperty("shown").GetBoolean(), item.IsVisible);
            Assert.Equal(c.GetProperty("clicks").GetBoolean(), item.IsVisible && item.IsEnabled);
        }
    }

    [AvaloniaFact]
    public void A_right_click_chooses_the_row_under_the_pointer()
    {
        OrderRow[] rows = [new() { Title = "Milk" }, new() { Title = "Bread" }, new() { Title = "Eggs" }];
        var table = new ClarityTable { ItemsSource = rows };
        table.Columns.Add(new TableColumn { Field = "Title", Header = "Task", Kind = CellKind.Text });
        var window = new Window { Width = 600, Height = 300, Content = table };
        var chosen = 0;
        table.Selected += (_, _) => chosen++;
        window.Show();
        Dispatcher.UIThread.RunJobs();

        var cell = table.CellOf(rows[1], "Title")!;
        var middle = cell.TranslatePoint(new Point(cell.Bounds.Width / 2, cell.Bounds.Height / 2), window)!.Value;
        window.MouseDown(middle, MouseButton.Right);
        window.MouseUp(middle, MouseButton.Right);
        Dispatcher.UIThread.RunJobs();

        Assert.Same(rows[1], table.SelectedItem);
        Assert.Equal(1, chosen);
        window.Close();
    }
}
