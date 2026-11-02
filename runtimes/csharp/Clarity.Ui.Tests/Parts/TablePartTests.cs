// Clarity's table part against the cases every stack shares: edited cells
// (conformance/tables.json), the order rows show in (table-order.json) and
// pages (table-pages.json), run through the screen as the person uses it:
// clicks on rows, checks, choices and headers, typing into cells. And what
// it draws: its header in the header look, only the rows in sight.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

/// <summary>The task the cases show, as a generated record.</summary>
public sealed class CaseTask
{
    public string Title { get; set; } = "";
    public bool Done { get; set; }
    public bool Urgent { get; set; }
    public string? Priority { get; set; }
    public long? Owner { get; set; }
    public DateTime? Due { get; set; }
    public DateOnly? Day { get; set; }
    public decimal Hours { get; set; }
    public string Notes { get; set; } = "";
}

/// <summary>A row of the order cases.</summary>
public sealed class OrderRow
{
    public string Title { get; set; } = "";
    public decimal? Hours { get; set; }
    public long? Owner { get; set; }
}

public sealed class Album
{
    public string Title { get; set; } = "";
}

static class Cases
{
    public static JsonElement Of(string file) =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", file))).RootElement;

    public static object? Plain(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.String => value.GetString(),
        JsonValueKind.Number => value.GetInt64(),
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => throw new ArgumentException($"no value {value}"),
    };

    public static void Click(Screen screen, Box box, int clicks = 1) => ClickAt(screen, box.X + box.Width / 2, box.Y + box.Height / 2, clicks);

    public static void ClickAt(Screen screen, double x, double y, int clicks = 1)
    {
        screen.PointerPressed(x, y, PointerButton.Left, clicks);
        screen.PointerReleased(x, y, PointerButton.Left);
    }
}

public class TablePartCaseTests
{
    static readonly JsonElement Tables = Cases.Of("tables.json");

    /// <summary>A value of the cases as the record's property holds it.</summary>
    static object? Typed(string field, JsonElement value) => (field, value.ValueKind) switch
    {
        (_, JsonValueKind.Null) => null,
        ("due", _) => DateTime.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        ("day", _) => DateOnly.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        ("hours", _) => decimal.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        _ => Cases.Plain(value),
    };

    static List<object> Rows() => [.. Tables.GetProperty("rows").EnumerateArray().Select(row =>
    {
        var task = new CaseTask();
        foreach (var field in row.EnumerateObject())
            RowFields.Set(task, field.Name, Typed(field.Name, field.Value));
        return (object)task;
    })];

    static IEnumerable<JsonElement> Columns => Tables.GetProperty("columns").EnumerateArray();

    [Fact]
    public void Every_case_of_edited_cells_behaves_as_on_every_stack()
    {
        Locale.Start(null, [], null, (_, _) => null, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
        try
        {
            var cases = Tables.GetProperty("cases").EnumerateArray().ToList();
            Assert.NotEmpty(cases);
            foreach (var @case in cases)
                Run(@case);
        }
        finally
        {
            Locale.Reset();
        }
    }

    static void Run(JsonElement @case)
    {
        var name = @case.GetProperty("name").GetString();
        var host = new DataHost();
        host.Lists["tasks"] = Rows();
        foreach (var column in Columns)
            if (column.TryGetProperty("choices", out var choices))
                host.Offered[RowFields.ColumnPath("tasks", column.GetProperty("field").GetString()!)] = [.. choices.EnumerateArray().Select(c => new Choice(Cases.Plain(c[0]), c[1].GetString()!))];
        var fields = string.Join(", ", Columns.Select(c => c.GetProperty("field").GetString()));
        var edited = string.Join(", ", Columns.Where(c => c.GetProperty("edit").GetBoolean()).Select(c => c.GetProperty("field").GetString()));
        var switches = string.Join(", ", Columns.Where(c => c.TryGetProperty("switch", out var s) && s.GetBoolean()).Select(c => c.GetProperty("field").GetString()));
        var (screen, _, parts) = Made.Window($"""<Table at="1,1" span="12" rows="6" name="tasks" bind="tasks" columns="{fields}" edit="{edited}" switches="{switches}" onselect="picked" onchange="changed" />""", host, 1200, 400);
        var table = Assert.IsType<TablePart>(parts[0]);
        var events = new List<string>();
        host.Ran = call => events.Add($"{call.Event} {table.ChosenPlace}");
        Box Cell(int row, string field) => table.CellBox(row, field) ?? throw new InvalidOperationException($"{name}: no cell {field} of row {row}");
        foreach (var step in @case.GetProperty("steps").EnumerateArray())
        {
            var what = step[0].GetString();
            int Row() => step[1].GetInt32();
            string Field() => step[2].GetString()!;
            switch (what)
            {
                case "click":
                    Cases.Click(screen, Cell(Row(), "notes"));
                    break;
                case "tick":
                    Cases.Click(screen, Cell(Row(), Field()));
                    break;
                case "pick":
                    Cases.Click(screen, Cell(Row(), Field()));
                    var list = table.OpenChoices ?? throw new InvalidOperationException($"{name}: no choices open");
                    var wanted = Cases.Plain(step[3]);
                    var at = list.Choices.ToList().FindIndex(c => RowsPart.Same(c.Value, wanted));
                    Cases.Click(screen, list.ItemBox(at));
                    Assert.Null(table.OpenChoices);
                    break;
                case "pickDay":
                    table.PickDay(Row(), Field(), DateOnly.Parse(step[3].GetString()!, CultureInfo.InvariantCulture));
                    break;
                case "type":
                    var cell = Cell(Row(), Field());
                    // A moment's time is typed at its right, beside its day.
                    Cases.ClickAt(screen, cell.Right - 12, cell.Y + cell.Height / 2);
                    screen.TextTyped(step[3].GetString()!);
                    break;
                case "enter":
                    screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
                    break;
                case "leave":
                    screen.Focus(null);
                    break;
                case "handler":
                    RowFields.Set(host.Lists["tasks"][Row()], Field(), Typed(Field(), step[3]));
                    host.Change();
                    break;
                case "reload":
                    host.Lists["tasks"] = Rows();
                    host.Change();
                    break;
                default:
                    throw new ArgumentException($"{name}: unknown step {what}");
            }
        }
        var expect = @case.GetProperty("expect");
        Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()!).SequenceEqual(events), $"{name}: events were [{string.Join(", ", events)}]");
        // Every value as it started, but those the case changes.
        var start = Rows();
        var rows = host.Lists["tasks"];
        var changed = expect.TryGetProperty("values", out var values) ? values.EnumerateArray().ToList() : [];
        foreach (var field in Columns.Select(c => c.GetProperty("field").GetString()!))
            for (var row = 0; row < rows.Count; row++)
            {
                var change = changed.FirstOrDefault(v => v[0].GetInt32() == row && v[1].GetString() == field);
                var wanted = change.ValueKind == JsonValueKind.Undefined ? DataHost.Written(RowFields.Read(start[row], field)) : DataHost.Written(Typed(field, change[2]));
                var now = DataHost.Written(RowFields.Read(rows[row], field));
                Assert.True(wanted == now, $"{name}: {field} of row {row} is {now}, not {wanted}");
            }
        if (expect.TryGetProperty("marked", out var marked))
            foreach (var m in marked.EnumerateArray())
                Assert.True(table.Marked(m[0].GetInt32(), m[1].GetString()!), $"{name}: {m[1]} of row {m[0]} is not marked");
        if (expect.TryGetProperty("shown", out var shown))
            foreach (var s in shown.EnumerateArray())
            {
                var text = table.Shown(s[0].GetInt32(), s[1].GetString()!);
                Assert.True(s[2].GetString() == text, $"{name}: {s[1]} of row {s[0]} shows '{text}'");
            }
        if (expect.TryGetProperty("chosen", out var chosen))
            Assert.True((chosen.ValueKind == JsonValueKind.Null ? -1 : chosen.GetInt32()) == table.ChosenPlace, $"{name}: row {table.ChosenPlace} is chosen");
        if (expect.TryGetProperty("choices", out var expectedChoices))
            foreach (var field in expectedChoices.EnumerateObject())
            {
                var labels = table.Columns.First(c => c.Field == field.Name).Choices.Select(c => c.Label);
                Assert.True(field.Value.EnumerateArray().Select(l => l.GetString()!).SequenceEqual(labels), $"{name}: the choices of {field.Name} are [{string.Join(", ", labels)}]");
            }
        // Drawing it is no edit either.
        using (Shot.Of(screen, 1200, 400)) { }
        Assert.Equal(events.Count, host.Events.Count);
    }

    [Fact]
    public void An_enum_column_offers_the_enums_values()
    {
        var host = new DataHost();
        host.Lists["shirts"] = [new Shirt { Size = Size.Large }];
        var (_, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="4" name="shirts" bind="shirts" columns="size" edit="size" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        var column = Assert.Single(table.Columns);
        Assert.Equal(CellKind.Choice, column.Kind);
        Assert.Equal(["Small", "Large"], column.Choices.Select(c => c.Label));
        Assert.Equal("Large", table.Shown(0, "size"));
    }

    public enum Size { Small, Large }

    public sealed class Shirt
    {
        public Size Size { get; set; }
    }
}

public class TablePartOrderTests
{
    static readonly JsonElement Order = Cases.Of("table-order.json");

    static List<object> Rows() => [.. Order.GetProperty("rows").EnumerateArray().Select(r => (object)new OrderRow
    {
        Title = r.GetProperty("title").GetString()!,
        Hours = r.GetProperty("hours").ValueKind == JsonValueKind.Null ? null : decimal.Parse(r.GetProperty("hours").GetString()!, CultureInfo.InvariantCulture),
        Owner = r.GetProperty("owner").ValueKind == JsonValueKind.Null ? null : r.GetProperty("owner").GetInt64(),
    })];

    [Fact]
    public void Every_order_case_behaves_as_on_every_stack()
    {
        var cases = Order.GetProperty("cases").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var @case in cases)
        {
            var name = @case.GetProperty("name").GetString();
            var host = new DataHost();
            host.Lists["orders"] = Rows();
            var owner = Order.GetProperty("columns").EnumerateArray().First(c => c.GetProperty("field").GetString() == "owner");
            host.Offered["orders.owner"] = [.. owner.GetProperty("choices").EnumerateArray().Select(c => new Choice(Cases.Plain(c[0]), c[1].GetString()!))];
            var sortable = @case.GetProperty("sortable").GetBoolean() ? "sortable=\"true\"" : "";
            var order = @case.GetProperty("order") is { ValueKind: JsonValueKind.Object } o ? $"order='{o.GetRawText()}'" : "";
            var (screen, _, parts) = Made.Window($"""<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title, hours, owner" {sortable} {order} onselect="picked" />""", host);
            var table = Assert.IsType<TablePart>(parts[0]);
            var events = new List<string>();
            host.Ran = call => events.Add($"{call.Event} {table.ChosenPlace}");
            foreach (var step in @case.GetProperty("steps").EnumerateArray())
                switch (step[0].GetString())
                {
                    case "sort":
                        Cases.Click(screen, table.HeaderBox(step[1].GetString()!)!.Value);
                        break;
                    case "click":
                        Cases.Click(screen, table.CellBox(table.ShownPlaces[step[1].GetInt32()], "title")!.Value);
                        break;
                    default:
                        throw new ArgumentException($"{name}: unknown step {step[0]}");
                }
            var expect = @case.GetProperty("expect");
            Assert.True(expect.GetProperty("shown").EnumerateArray().Select(e => e.GetInt32()).SequenceEqual(table.ShownPlaces), $"{name}: rows show as [{string.Join(", ", table.ShownPlaces)}]");
            Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()!).SequenceEqual(events), $"{name}: events were [{string.Join(", ", events)}]");
            if (expect.TryGetProperty("chosen", out var chosen))
                Assert.True(chosen.GetInt32() == table.ChosenPlace, $"{name}: row {table.ChosenPlace} is chosen");
        }
    }

    [Fact]
    public void The_person_drags_a_header_edge_and_a_double_click_gives_the_designed_widths()
    {
        var host = new DataHost();
        host.Lists["orders"] = Rows();
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title 150, hours" resizable="true" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        var title = table.HeaderBox("title")!.Value;
        Assert.Equal(150, title.Width);
        screen.PointerPressed(title.Right - 2, title.Y + 10, PointerButton.Left);
        screen.PointerMoved(title.Right + 48, title.Y + 10);
        screen.PointerReleased(title.Right + 48, title.Y + 10, PointerButton.Left);
        Assert.Equal(200, table.HeaderBox("title")!.Value.Width);
        Assert.Equal(200, table.CellBox(0, "title")!.Value.Width);
        var edge = table.HeaderBox("title")!.Value;
        screen.PointerPressed(edge.Right - 2, edge.Y + 10, PointerButton.Left, 2);
        screen.PointerReleased(edge.Right - 2, edge.Y + 10, PointerButton.Left);
        Assert.Equal(150, table.HeaderBox("title")!.Value.Width);
    }

    [Fact]
    public void The_header_is_drawn_in_the_header_look()
    {
        var host = new DataHost();
        host.Lists["orders"] = Rows();
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title, hours" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        using var pixels = Shot.Of(screen, 800, 400);
        var header = table.HeaderBox("hours")!.Value;
        Assert.Equal(screen.Look.Part("header", null).Background, pixels.GetPixel((int)header.Right - 6, (int)header.Y + 4));
        host.State("orders")!.SelectedItem = host.Lists["orders"][1];
        using var chosen = Shot.Of(screen, 800, 400);
        var row = table.RowBox(1)!.Value;
        Assert.Equal(screen.Look.Part("list", "selected").Background, chosen.GetPixel((int)row.Right - 6, (int)(row.Y + row.Height / 2)));
    }
}

public class TablePartPageTests
{
    static readonly JsonElement Pages = Cases.Of("table-pages.json");

    [Fact]
    public void A_page_shows_the_rows_every_stack_shows()
    {
        foreach (var c in Pages.GetProperty("pages").EnumerateArray())
        {
            var host = new DataHost();
            var count = c.GetProperty("count").GetInt32();
            host.Lists["albums"] = [.. Enumerable.Range(1, count).Select(i => (object)new Album { Title = $"Album {i:000}" })];
            var (screen, _, parts) = Made.Window($"""<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" page="{c.GetProperty("size").GetInt32()}" />""", host, 800, 2000);
            var table = Assert.IsType<TablePart>(parts[0]);
            table.TurnTo(c.GetProperty("page").GetInt32());
            var paged = table.Paged!;
            Assert.Equal(new Paged(c.GetProperty("shows").GetInt32(), c.GetProperty("pages").GetInt32(), c.GetProperty("from").GetInt32(), c.GetProperty("to").GetInt32(), c.GetProperty("text").GetString()!), paged);
            Assert.Equal(paged.From == 0 ? 0 : paged.To - paged.From + 1, table.ShownPlaces.Count);
            // First and previous work past the first page, next and last before the last.
            Assert.Equal(paged.Page > 1, table.PageButtonWorks(PageButton.First));
            Assert.Equal(paged.Page < paged.Pages, table.PageButtonWorks(PageButton.Last));
        }
        foreach (var c in Pages.GetProperty("selected").EnumerateArray())
        {
            var host = new DataHost();
            host.Lists["albums"] = [.. Enumerable.Range(0, c.GetProperty("at").GetInt32() + 1).Select(i => (object)new Album { Title = $"Album {i:000}" })];
            var (_, _, parts) = Made.Window($"""<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" page="{c.GetProperty("size").GetInt32()}" />""", host);
            var table = Assert.IsType<TablePart>(parts[0]);
            host.State("albums")!.SelectedItem = host.Lists["albums"][^1];
            Assert.Equal(c.GetProperty("page").GetInt32(), table.Paged!.Page);
        }
    }

    [Fact]
    public void The_bar_turns_the_pages_and_a_selected_row_shows_its_page()
    {
        var host = new DataHost();
        host.Lists["albums"] = [.. Enumerable.Range(1, 7).Select(i => (object)new Album { Title = $"Album {i:00}" })];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" page="3" sortable="true" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        List<string> Shown() => [.. table.ShownPlaces.Select(p => ((Album)host.Lists["albums"][p]).Title)];
        Assert.Equal(["Album 01", "Album 02", "Album 03"], Shown());
        Assert.Equal("1-3 / 7", table.Paged!.Text);
        Assert.False(table.PageButtonWorks(PageButton.First));
        Assert.Equal("Next page", TablePart.PageWord(PageButton.Next));
        Cases.Click(screen, table.PagerBox(PageButton.Next)!.Value);
        Assert.Equal(["Album 04", "Album 05", "Album 06"], Shown());
        Cases.Click(screen, table.PagerBox(PageButton.Last)!.Value);
        Assert.Equal(["Album 07"], Shown());
        Assert.False(table.PageButtonWorks(PageButton.Next));
        // Rows that come again keep the page, within the pages there are.
        host.Lists["albums"].RemoveAt(6);
        host.Change();
        Assert.Equal("4-6 / 6", table.Paged!.Text);
        // A sort shows the first page.
        Cases.Click(screen, table.HeaderBox("title")!.Value);
        Assert.Equal("1-3 / 6", table.Paged!.Text);
        // A row selected by code shows its page.
        host.State("albums")!.SelectedItem = host.Lists["albums"][4];
        Assert.Equal(["Album 04", "Album 05", "Album 06"], Shown());
        // The bar is drawn below the rows.
        using var pixels = Shot.Of(screen, 800, 400);
        Assert.NotNull(table.PagerBox(PageButton.First));
    }

    [Fact]
    public void Only_the_rows_in_sight_are_drawn_and_the_rows_scroll_under_the_header()
    {
        var host = new DataHost();
        host.Lists["albums"] = [.. Enumerable.Range(1, 5000).Select(i => (object)new Album { Title = $"Album {i:0000}" })];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        Assert.NotNull(table.RowBox(0));
        Assert.Null(table.RowBox(100));
        var header = table.HeaderBox("title")!.Value;
        screen.Wheel(header.X + 20, header.Bottom + 20, 0, 1000);
        Assert.Null(table.RowBox(0));
        Assert.Equal(header, table.HeaderBox("title"));
        screen.Focus(table);
        screen.KeyPressed(new KeyInput(Key.End, Modifiers.None));
        Assert.Equal(4999, table.ChosenPlace);
        Assert.NotNull(table.RowBox(4999));
        using var pixels = Shot.Of(screen, 800, 400);
    }
}

/// <summary>A cell is typed into with Clarity.UI's text editor: its caret, selection and clipboard, under the table's rules.</summary>
public class TableCellEditorTests
{
    static (Screen Screen, TablePart Table, DataHost Host) Made()
    {
        var host = new DataHost();
        host.Lists["albums"] = [new Album { Title = "Blue" }, new Album { Title = "Kind of Blue" }];
        var (screen, _, parts) = Clarity.Ui.Tests.Made.Window("""<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" edit="title" onselect="picked" onchange="changed" />""", host);
        return (screen, Assert.IsType<TablePart>(parts[0]), host);
    }

    static KeyInput K(Key key, Modifiers modifiers = Modifiers.None) => new(key, modifiers);

    static string Title(DataHost host, int place) => ((Album)host.Lists["albums"][place]).Title;

    [Fact]
    public void A_cell_takes_caret_moves_a_selection_and_pasting()
    {
        var (screen, table, host) = Made();
        Cases.Click(screen, table.CellBox(0, "title")!.Value);
        var editor = Assert.IsType<TextEditor>(screen.Focused);
        // All of it is selected, so what is typed first replaces it.
        Assert.Equal("Blue", editor.SelectedText);
        screen.KeyPressed(K(Key.End));
        screen.KeyPressed(K(Key.Left, Modifiers.Shift));
        screen.KeyPressed(K(Key.Left, Modifiers.Shift));
        Assert.Equal("ue", editor.SelectedText);
        screen.Clipboard.SetText("ack");
        screen.KeyPressed(K(Key.V, Modifiers.Control));
        Assert.Equal("Black", table.Shown(0, "title"));
        Assert.Equal("Blue", Title(host, 0));
        // Enter takes it, and the table has the focus again.
        screen.KeyPressed(K(Key.Enter));
        Assert.Equal("Black", Title(host, 0));
        Assert.Same(table, screen.Focused);
        Assert.Equal(["select", "change"], host.Events);
        using var pixels = Shot.Of(screen, 800, 400);
    }

    [Fact]
    public void Escape_leaves_the_cell_and_its_value_stays()
    {
        var (screen, table, host) = Made();
        Cases.Click(screen, table.CellBox(1, "title")!.Value);
        screen.TextTyped("Red");
        Assert.Equal("Red", table.Shown(1, "title"));
        screen.KeyPressed(K(Key.Escape));
        Assert.Equal("Kind of Blue", table.Shown(1, "title"));
        Assert.Same(table, screen.Focused);
        screen.Focus(null);
        Assert.Equal("Kind of Blue", Title(host, 1));
        Assert.Empty(host.Events);
    }

    [Fact]
    public void A_press_in_the_cell_typed_into_places_the_caret_and_Tab_takes_the_text()
    {
        var (screen, table, host) = Made();
        var cell = table.CellBox(0, "title")!.Value;
        Cases.Click(screen, cell);
        var editor = Assert.IsType<TextEditor>(screen.Focused);
        Cases.ClickAt(screen, cell.X + 6, cell.Y + cell.Height / 2);
        Assert.Same(editor, screen.Focused);
        Assert.Equal((0, 0), editor.Selection);
        screen.TextTyped("True ");
        screen.KeyPressed(K(Key.Tab));
        Assert.Equal("True Blue", Title(host, 0));
        Assert.NotSame(editor, screen.Focused);
    }
}

public class TableScrollTests
{
    [Fact]
    public void The_rows_scroll_with_the_bar_of_a_scroll_view()
    {
        var host = new DataHost();
        host.Lists["albums"] = [.. Enumerable.Range(1, 500).Select(i => (object)new Album { Title = $"Album {i:000}" })];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        var view = Assert.Single(table.Children.OfType<ScrollView>());
        // The rows scroll under the header: the scroll view is the body.
        Assert.Equal(table.HeaderBox("title")!.Value.Bottom, view.Bounds.Y);
        var x = view.Bounds.Right - 3;
        // A press on the track below the thumb shows the next page.
        screen.PointerPressed(x, view.Bounds.Bottom - 3, PointerButton.Left);
        screen.PointerReleased(x, view.Bounds.Bottom - 3, PointerButton.Left);
        Assert.Equal(view.Bounds.Height, table.Scrolled, 3);
        Assert.Null(table.RowBox(0));
        // The wheel over the header scrolls the rows too; a turn up shows what is above.
        var header = table.HeaderBox("title")!.Value;
        screen.Wheel(header.X + 20, header.Y + 5, 0, -10000);
        Assert.Equal(0, table.Scrolled);
        using var pixels = Shot.Of(screen, 800, 400);
    }

    [Fact]
    public void A_choice_cell_with_more_choices_than_show_scrolls_to_the_last()
    {
        var host = new DataHost();
        var task = new CaseTask { Title = "Write", Priority = "P0" };
        host.Lists["tasks"] = [task];
        host.Offered["tasks.priority"] = [.. Enumerable.Range(0, 12).Select(i => new Choice($"P{i}", $"Priority {i}"))];
        var (screen, _, parts) = Made.Window("""<Table name="tasks" at="1,1" span="12" rows="8" bind="tasks" columns="title, priority" edit="priority" />""", host, height: 600);
        var table = (TablePart)parts[0];
        Cases.Click(screen, table.CellBox(0, "priority")!.Value);
        var list = table.OpenChoices!;
        Assert.Equal(CellChoices.HeightFor(12, list.ItemBox(0).Height), list.Bounds.Height);
        // The wheel over the list scrolls it, and the last choice can be picked.
        screen.Wheel(list.Bounds.X + 20, list.Bounds.Y + 20, 0, 1000);
        Assert.True(list.View.ScrollY > 0);
        Assert.Equal(list.View.MaxScrollY, list.View.ScrollY);
        var last = list.ItemBox(11);
        Assert.True(last.Bottom <= list.Bounds.Bottom + 0.5);
        Cases.Click(screen, last);
        Assert.Null(table.OpenChoices);
        Assert.Equal("P11", task.Priority);
    }

    [Fact]
    public void A_choice_cell_opens_at_its_value_marked()
    {
        var host = new DataHost();
        var task = new CaseTask { Title = "Write", Priority = "P10" };
        host.Lists["tasks"] = [task];
        host.Offered["tasks.priority"] = [.. Enumerable.Range(0, 12).Select(i => new Choice($"P{i}", $"Priority {i}"))];
        var (screen, _, parts) = Made.Window("""<Table name="tasks" at="1,1" span="12" rows="8" bind="tasks" columns="title, priority" edit="priority" />""", host, height: 600);
        var table = (TablePart)parts[0];
        Cases.Click(screen, table.CellBox(0, "priority")!.Value);
        var list = table.OpenChoices!;
        // The value's choice is in view when the list opens, without scrolling to it.
        Assert.True(list.View.ScrollY > 0);
        Assert.Equal(10, list.Marked);
        var shown = list.View.Bounds;
        var at = 10 * (list.Bounds.Height - 2) / CellChoices.MostShown - list.View.ScrollY + shown.Y;
        Assert.InRange(at, shown.Y - 0.5, shown.Bottom);
    }
}

public class TableCalendarTests
{
    [Fact]
    public void A_date_cell_opens_the_calendar_and_a_picked_day_is_its_edit()
    {
        var host = new DataHost();
        var task = new CaseTask { Title = "Write", Day = new DateOnly(2026, 10, 5), Due = new DateTime(2026, 10, 5, 9, 30, 0) };
        host.Lists["tasks"] = [task];
        var (screen, _, parts) = Made.Window("""<Table name="tasks" at="1,1" span="12" rows="8" bind="tasks" columns="title, day, due" edit="day, due" />""", host, height: 600);
        var table = (TablePart)parts[0];
        var cell = table.CellBox(0, "day")!.Value;
        Cases.Click(screen, cell);
        var calendar = Assert.IsType<DayCalendar>(Assert.Single(screen.Overlays).Content);
        Assert.Equal(new DateOnly(2026, 10, 5), calendar.Chosen);
        Cases.Click(screen, calendar.CellOf(new DateOnly(2026, 10, 20))!.Value);
        Assert.Empty(screen.Overlays);
        Assert.Equal(new DateOnly(2026, 10, 20), task.Day);
        // A moment's day is picked the same way, and keeps its time.
        var due = table.CellBox(0, "due")!.Value;
        Cases.ClickAt(screen, due.X + 8, due.Y + due.Height / 2);
        calendar = Assert.IsType<DayCalendar>(Assert.Single(screen.Overlays).Content);
        Cases.Click(screen, calendar.CellOf(new DateOnly(2026, 10, 21))!.Value);
        Assert.Equal(new DateTime(2026, 10, 21, 9, 30, 0), task.Due);
    }
}
