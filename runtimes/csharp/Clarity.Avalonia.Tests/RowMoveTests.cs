// Reordering through the real parts (DESIGN 5.30): a table, a tree table and
// a list whose rows the person drags, or moves with Alt and an arrow; each
// record gets its number (and in a tree its parent), the list its new order,
// the moved row stays chosen, and the part tells once.

using System.Collections.ObjectModel;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Templates;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class RowMoveTests
{
    public sealed class Task
    {
        public long Id { get; set; }
        public string Title { get; set; } = "";
        public long Position { get; set; }
        public long? Parent { get; set; }
    }

    static ObservableCollection<Task> Tasks() =>
    [
        new() { Id = 1, Title = "One", Position = 1 },
        new() { Id = 2, Title = "Two", Position = 2 },
        new() { Id = 3, Title = "Three", Position = 3 },
    ];

    static (Window, ClarityTable) Shown(ClarityTable table)
    {
        table.Columns.Add(new TableColumn { Field = "Title", Header = "Title" });
        var window = new Window { Width = 400, Height = 300, Content = table };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        return (window, table);
    }

    static string Order(IEnumerable<Task> tasks) => string.Join(",", tasks.Select(t => $"{t.Title}{t.Position}"));

    [AvaloniaFact]
    public void Alt_and_down_moves_the_chosen_row_and_numbers_the_rows()
    {
        var tasks = Tasks();
        var told = 0;
        var (window, table) = Shown(new ClarityTable { ReorderBy = "Position" });
        table.ItemsSource = tasks;
        table.Reordered += (_, _) => told++;
        table.SelectedItem = tasks[0];
        Dispatcher.UIThread.RunJobs();
        table.Grid.Focus();
        window.KeyPressQwerty(PhysicalKey.ArrowDown, RawInputModifiers.Alt);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal("Two1,One2,Three3", Order(tasks));
        Assert.Same(tasks[1], table.SelectedItem);
        Assert.Equal(1, told);
        // Up on the first row changes nothing and tells nothing.
        table.SelectedItem = tasks[0];
        window.KeyPressQwerty(PhysicalKey.ArrowUp, RawInputModifiers.Alt);
        Assert.Equal(1, told);
    }

    [AvaloniaFact]
    public void A_dragged_row_drops_after_another()
    {
        var tasks = Tasks();
        var told = 0;
        var (window, table) = Shown(new ClarityTable { ReorderBy = "Position", HeadersVisible = false });
        table.ItemsSource = tasks;
        table.Reordered += (_, _) => told++;
        Dispatcher.UIThread.RunJobs();
        var rows = table.Grid.GetVisualDescendants().OfType<DataGridRow>().OrderBy(r => r.Bounds.Y).ToList();
        Point Middle(DataGridRow row, double part) => row.TranslatePoint(new Point(20, row.Bounds.Height * part), window)!.Value;
        window.MouseDown(Middle(rows[0], 0.5), MouseButton.Left);
        window.MouseMove(Middle(rows[1], 0.5));
        window.MouseMove(Middle(rows[2], 0.8));
        window.MouseUp(Middle(rows[2], 0.8), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal("Two1,Three2,One3", Order(tasks));
        Assert.Equal(1, told);
        Assert.Same(tasks[2], table.SelectedItem);
    }

    [AvaloniaFact]
    public void In_a_tree_alt_and_right_puts_a_row_under_the_row_before_it()
    {
        var tasks = Tasks();
        var (window, table) = Shown(new ClarityTable { ReorderBy = "Position", TreeParent = "Parent", HeadersVisible = false });
        table.ItemsSource = tasks;
        table.SelectedItem = tasks[1];
        Dispatcher.UIThread.RunJobs();
        table.Grid.Focus();
        window.KeyPressQwerty(PhysicalKey.ArrowRight, RawInputModifiers.Alt);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal(1L, tasks.Single(t => t.Title == "Two").Parent);
        Assert.Equal("One1,Two1,Three2", Order(tasks));
        // The row it went under opened, so the moved row shows, chosen.
        Assert.True(table.IsOpen(tasks[0]));
        Assert.Same(tasks.Single(t => t.Title == "Two"), table.SelectedItem);
    }

    [AvaloniaFact]
    public void A_list_moves_its_rows_with_alt_and_the_arrows()
    {
        var tasks = Tasks();
        var told = 0;
        var list = new ListBox
        {
            ItemsSource = tasks,
            ItemTemplate = new FuncDataTemplate<Task>((t, _) => new TextBlock { Text = t?.Title }),
        };
        RowMoves.SetBy(list, "Position");
        RowMoves.OnReorder(list, () => told++);
        var window = new Window { Width = 300, Height = 300, Content = list };
        window.Show();
        list.SelectedItem = tasks[2];
        Dispatcher.UIThread.RunJobs();
        // A click on a row focuses it, inside the list.
        list.ContainerFromIndex(2)!.Focus();
        Assert.True(list.IsKeyboardFocusWithin);
        window.KeyPressQwerty(PhysicalKey.ArrowUp, RawInputModifiers.Alt);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal("One1,Three2,Two3", Order(tasks));
        Assert.Same(tasks[1], list.SelectedItem);
        Assert.Equal(1, told);
    }

    [AvaloniaFact]
    public void On_a_paged_table_a_row_moved_off_its_page_shows_on_the_next()
    {
        var tasks = Tasks();
        var (window, table) = Shown(new ClarityTable { ReorderBy = "Position", PageSize = 2 });
        table.ItemsSource = tasks;
        table.SelectedItem = tasks[1];
        Dispatcher.UIThread.RunJobs();
        table.Grid.Focus();
        window.KeyPressQwerty(PhysicalKey.ArrowDown, RawInputModifiers.Alt);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal("One1,Three2,Two3", Order(tasks));
        Assert.Equal(2, table.Page);
        Assert.Same(tasks[2], table.Grid.SelectedItem);
    }

    /// A generated window holds a list as a plain List, which tells nobody it moved: the list shows the new order all the same.
    [AvaloniaFact]
    public void A_list_of_a_plain_list_shows_its_new_order()
    {
        List<Task> tasks = [.. Tasks()];
        var list = new ListBox
        {
            ItemsSource = tasks,
            ItemTemplate = new FuncDataTemplate<Task>((t, _) => new TextBlock { [!TextBlock.TextProperty] = new global::Avalonia.Data.Binding("Title") }),
        };
        RowMoves.SetBy(list, "Position");
        var window = new Window { Width = 300, Height = 300, Content = list };
        window.Show();
        list.SelectedItem = tasks[0];
        Dispatcher.UIThread.RunJobs();
        list.ContainerFromIndex(0)!.Focus();
        window.KeyPressQwerty(PhysicalKey.ArrowDown, RawInputModifiers.Alt);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal("Two1,One2,Three3", Order(tasks));
        var shown = Enumerable.Range(0, 3).Select(i => ((list.ContainerFromIndex(i) as ListBoxItem)?.DataContext as Task)?.Title).ToList();
        Assert.Equal(["Two", "One", "Three"], shown);
        Assert.Same(tasks[1], list.SelectedItem);
    }

    [AvaloniaFact]
    public void Without_reorder_rows_stay_put()
    {
        var tasks = Tasks();
        var (window, table) = Shown(new ClarityTable());
        table.ItemsSource = tasks;
        table.SelectedItem = tasks[0];
        Dispatcher.UIThread.RunJobs();
        table.Grid.Focus();
        window.KeyPressQwerty(PhysicalKey.ArrowDown, RawInputModifiers.Alt);
        Assert.Equal("One1,Two2,Three3", Order(tasks));
    }
}
