// Every kind of edited cell is reached and changed with the keyboard, as the
// inputs of Avalonia's table were: Tab walks checks, switches, choices and
// dates as well as typed cells; Space ticks a check; a choice cell's list
// opens with Enter, Space or Alt+Down and is worked with the arrows, Home,
// End, Enter, Escape and the first letters of a choice; a date is typed as
// well as picked from its calendar.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class TableKeyTests
{
    static (Screen Screen, TablePart Table, DataHost Host, CaseTask Task) Made(string extra = "")
    {
        Locale.Start(null, [], null, (_, _) => null, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
        var host = new DataHost();
        var task = new CaseTask { Title = "Write", Priority = "Low", Day = new DateOnly(2026, 10, 5) };
        host.Lists["tasks"] = [task, new CaseTask { Title = "Read", Priority = "High" }];
        host.Offered["tasks.priority"] = [new Choice("Low", "Low"), new Choice("Medium", "Medium"), new Choice("High", "High"), new Choice("Highest", "Highest")];
        var (screen, _, parts) = Clarity.Ui.Tests.Made.Window($"""<Table name="tasks" at="1,1" span="12" rows="8" bind="tasks" columns="title, done, priority, day" edit="title, done, priority, day" onopen="opened" onselect="picked" onchange="changed" {extra} />""", host, 900, 600);
        return (screen, (TablePart)parts[0], host, task);
    }

    static KeyInput K(Key key, Modifiers modifiers = Modifiers.None) => new(key, modifiers);

    [Fact]
    public void Tab_walks_checks_choices_and_dates_too_and_space_ticks_a_check()
    {
        var (screen, table, _, task) = Made();
        try
        {
            Cases.Click(screen, table.CellBox(0, "title")!.Value);
            screen.KeyPressed(K(Key.Tab));
            Assert.Equal((0, "done"), table.FocusedCell);
            Assert.Same(table, screen.Focused);
            Assert.Null(table.CellEditor);
            screen.KeyPressed(K(Key.Space));
            Assert.True(task.Done);
            screen.KeyPressed(K(Key.Tab));
            Assert.Equal((0, "priority"), table.FocusedCell);
            screen.KeyPressed(K(Key.Enter));
            Assert.NotNull(table.OpenChoices);
            screen.KeyPressed(K(Key.Down));
            screen.KeyPressed(K(Key.Enter));
            Assert.Null(table.OpenChoices);
            Assert.Equal("Medium", task.Priority);
            // A date is typed in its cell.
            screen.KeyPressed(K(Key.Tab));
            Assert.NotNull(table.CellEditor);
            Assert.Equal((0, "day"), table.FocusedCell);
            table.CellEditor!.SelectAll();
            screen.TextTyped("10/22/2026");
            screen.KeyPressed(K(Key.Enter));
            Assert.Equal(new DateOnly(2026, 10, 22), task.Day);
            // Shift+Tab goes back through them.
            Cases.Click(screen, table.CellBox(0, "day")!.Value);
            screen.KeyPressed(K(Key.Tab, Modifiers.Shift));
            Assert.Equal((0, "priority"), table.FocusedCell);
            screen.KeyPressed(K(Key.Tab, Modifiers.Shift));
            Assert.Equal((0, "done"), table.FocusedCell);
            // A press on a check ticks it and leaves it the cell the keys work on.
            Cases.Click(screen, table.CellBox(0, "done")!.Value);
            Assert.False(task.Done);
            Assert.Equal((0, "done"), table.FocusedCell);
            using var pixels = Shot.Of(screen, 900, 600);
        }
        finally
        {
            Locale.Reset();
        }
    }

    [Fact]
    public void A_choice_list_is_worked_with_the_keyboard()
    {
        var (screen, table, host, task) = Made();
        try
        {
            Cases.Click(screen, table.CellBox(0, "title")!.Value);
            screen.KeyPressed(K(Key.Tab));
            screen.KeyPressed(K(Key.Tab));
            Assert.Equal((0, "priority"), table.FocusedCell);
            screen.KeyPressed(K(Key.Down, Modifiers.Alt));
            var list = table.OpenChoices!;
            Assert.Equal(0, list.Highlighted);
            screen.KeyPressed(K(Key.End));
            Assert.Equal(3, list.Highlighted);
            screen.KeyPressed(K(Key.Home));
            Assert.Equal(0, list.Highlighted);
            // The first letters of a choice go to it.
            screen.TextTyped("h");
            Assert.Equal(2, list.Highlighted);
            screen.TextTyped("ighe");
            Assert.Equal(3, list.Highlighted);
            // Escape closes the list and the value stays.
            screen.KeyPressed(K(Key.Escape));
            Assert.Null(table.OpenChoices);
            Assert.Equal("Low", task.Priority);
            Assert.Same(table, screen.Focused);
            screen.KeyPressed(K(Key.Space));
            screen.KeyPressed(K(Key.Up));
            Assert.Equal(0, table.OpenChoices!.Highlighted);
            screen.KeyPressed(K(Key.Down));
            screen.KeyPressed(K(Key.Down));
            screen.KeyPressed(K(Key.Enter));
            Assert.Equal("High", task.Priority);
            Assert.Equal(["select", "change"], host.Events);
        }
        finally
        {
            Locale.Reset();
        }
    }

    [Fact]
    public void A_date_cell_is_typed_and_its_button_or_alt_down_opens_the_calendar()
    {
        var (screen, table, _, task) = Made();
        try
        {
            var cell = table.CellBox(0, "day")!.Value;
            Cases.Click(screen, cell);
            var editor = Assert.IsType<TextEditor>(screen.Focused);
            Assert.Equal(Locale.Formats.Show(new FieldFormat(FormatKind.Date), new DateOnly(2026, 10, 5)), editor.Text);
            screen.TextTyped("soon");
            screen.KeyPressed(K(Key.Enter));
            Assert.True(table.Marked(0, "day"));
            Assert.Equal(new DateOnly(2026, 10, 5), task.Day);
            // Alt+Down in the typed date opens its calendar.
            Cases.Click(screen, cell);
            screen.KeyPressed(K(Key.Down, Modifiers.Alt));
            Assert.NotNull(table.OpenCalendar);
            screen.KeyPressed(K(Key.Escape));
            Assert.Null(table.OpenCalendar);
            // So does a press on its button.
            Cases.Click(screen, table.CalendarButtonBox(0, "day")!.Value);
            var calendar = table.OpenCalendar!;
            Cases.Click(screen, calendar.CellOf(new DateOnly(2026, 10, 20))!.Value);
            Assert.Equal(new DateOnly(2026, 10, 20), task.Day);
            Assert.False(table.Marked(0, "day"));
        }
        finally
        {
            Locale.Reset();
        }
    }

    [Fact]
    public void Enter_on_a_chosen_row_opens_it_and_alt_and_an_arrow_without_a_chosen_row_passes_on()
    {
        var (screen, table, host, _) = Made("""reorder="position" """);
        try
        {
            screen.Focus(table);
            Assert.False(table.OnKey(K(Key.Down, Modifiers.Alt)));
            Assert.Equal(-1, table.ChosenPlace);
            screen.KeyPressed(K(Key.Down));
            screen.KeyPressed(K(Key.Enter));
            Assert.Equal(["select", "open"], host.Events);
        }
        finally
        {
            Locale.Reset();
        }
    }
}
