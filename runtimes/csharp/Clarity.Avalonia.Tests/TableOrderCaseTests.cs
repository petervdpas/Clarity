// The order a table's rows show in, against the cases every renderer shares
// (conformance/table-order.json), run through the real ClarityTable: a click
// on a header sorts, a click on a shown row chooses it. And the widths: the
// designed ones, the person's remembered ones, and back to the designed.

using System.Collections.ObjectModel;
using System.Globalization;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// A row of the order cases, as a generated record.
public sealed class OrderRow
{
    public string Title { get; set; } = "";
    public decimal? Hours { get; set; }
    public long? Owner { get; set; }
}

public class TableOrderCaseTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "table-order.json"))).RootElement;

    static string Pascal(string field) => char.ToUpperInvariant(field[0]) + field[1..];

    static OrderRow[] Rows() => File.GetProperty("rows").EnumerateArray().Select(r => new OrderRow
    {
        Title = r.GetProperty("title").GetString()!,
        Hours = r.GetProperty("hours").ValueKind == JsonValueKind.Null ? null : decimal.Parse(r.GetProperty("hours").GetString()!, CultureInfo.InvariantCulture),
        Owner = r.GetProperty("owner").ValueKind == JsonValueKind.Null ? null : r.GetProperty("owner").GetInt64(),
    }).ToArray();

    static List<TableColumn> Columns() => File.GetProperty("columns").EnumerateArray().Select(c => new TableColumn
    {
        Field = Pascal(c.GetProperty("field").GetString()!),
        Header = c.GetProperty("label").GetString()!,
        Kind = Enum.Parse<CellKind>(c.GetProperty("kind").GetString()!, ignoreCase: true),
        Choices = c.TryGetProperty("choices", out var choices)
            ? choices.EnumerateArray().Select(p => new Choice(p[0].ValueKind == JsonValueKind.Null ? null : p[0].GetInt64(), p[1].GetString()!)).ToArray()
            : null,
    }).ToList();

    [AvaloniaFact]
    public void EveryOrderCaseBehavesAsOnEveryRenderer()
    {
        var cases = File.GetProperty("cases").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var @case in cases)
            Run(@case);
    }

    static void Run(JsonElement @case)
    {
        var name = @case.GetProperty("name").GetString();
        var rows = Rows();
        var table = new ClarityTable { ItemsSource = rows, Sortable = @case.GetProperty("sortable").GetBoolean() };
        foreach (var column in Columns())
            table.Columns.Add(column);
        if (@case.GetProperty("order") is { ValueKind: JsonValueKind.Object } order)
        {
            table.OrderBy = Pascal(order.GetProperty("field").GetString()!);
            table.OrderDescending = order.GetProperty("desc").GetBoolean();
        }
        var window = new Window { Width = 900, Height = 400, Content = table };
        var events = new List<string>();
        table.Selected += (_, _) => events.Add($"select {Array.IndexOf(rows, table.SelectedItem)}");
        window.Show();
        Dispatcher.UIThread.RunJobs();

        List<object> Shown() => table.Grid.ItemsSource!.Cast<object>().ToList();
        void Click(Control control)
        {
            var middle = control.TranslatePoint(new Point(control.Bounds.Width / 2, control.Bounds.Height / 2), window)!.Value;
            window.MouseDown(middle, MouseButton.Left);
            window.MouseUp(middle, MouseButton.Left);
            Dispatcher.UIThread.RunJobs();
        }

        foreach (var step in @case.GetProperty("steps").EnumerateArray())
        {
            switch (step[0].GetString())
            {
                case "sort":
                    var label = Columns().First(c => c.Field == Pascal(step[1].GetString()!)).Header;
                    var header = table.Grid.GetVisualDescendants().OfType<DataGridColumnHeader>()
                        .First(h => h.Content is string text && (text == label || text.StartsWith(label + " ")));
                    Click(header);
                    break;
                case "click":
                    var row = Shown()[step[1].GetInt32()];
                    Click(table.CellOf(row, "Title") ?? throw new InvalidOperationException($"{name}: no title cell"));
                    break;
                default:
                    throw new ArgumentException($"{name}: unknown step {step[0]}");
            }
            Dispatcher.UIThread.RunJobs();
        }

        var expect = @case.GetProperty("expect");
        var shown = Shown().Select(r => Array.IndexOf(rows, r)).ToList();
        Assert.True(expect.GetProperty("shown").EnumerateArray().Select(e => e.GetInt32()).SequenceEqual(shown), $"{name}: rows show as [{string.Join(", ", shown)}]");
        Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()!).SequenceEqual(events), $"{name}: events were [{string.Join(", ", events)}]");
        if (expect.TryGetProperty("chosen", out var chosen))
            Assert.True(ReferenceEquals(chosen.ValueKind == JsonValueKind.Null ? null : rows[chosen.GetInt32()], table.SelectedItem), $"{name}: row {Array.IndexOf(rows, table.SelectedItem)} is chosen");
        window.Close();
    }
}

public sealed class TableWidthTests : IDisposable
{
    readonly string folder = Directory.CreateTempSubdirectory("clarity-widths-").FullName;

    public TableWidthTests() => Remembered.Folder = folder;

    public void Dispose()
    {
        Remembered.Folder = null;
        Directory.Delete(folder, recursive: true);
    }

    static ClarityTable Table(string? remember)
    {
        var table = new ClarityTable { ItemsSource = new ObservableCollection<OrderRow> { new() { Title = "Milk" } }, Resizable = true, Remember = remember };
        table.Columns.Add(new TableColumn { Field = "Title", Header = "Task", Kind = CellKind.Text, Width = 150 });
        table.Columns.Add(new TableColumn { Field = "Hours", Header = "Hours", Kind = CellKind.Number });
        return table;
    }

    [AvaloniaFact]
    public void A_designed_width_is_in_pixels_and_a_column_without_one_shares_what_is_left()
    {
        var table = Table(null);
        Assert.Equal(new DataGridLength(150, DataGridLengthUnitType.Pixel), table.Grid.Columns[0].Width);
        Assert.True(table.Grid.Columns[1].Width.IsStar);
        Assert.True(table.Grid.CanUserResizeColumns);
    }

    [AvaloniaFact]
    public void The_persons_widths_are_remembered_per_window_and_table_and_a_reset_gives_the_designed_ones()
    {
        Remembered.Set("columns.Main.grid", "{\"Title\":220,\"Hours\":90}");
        var table = Table("Main.grid");
        Assert.Equal(new DataGridLength(220, DataGridLengthUnitType.Pixel), table.Grid.Columns[0].Width);
        Assert.Equal(new DataGridLength(90, DataGridLengthUnitType.Pixel), table.Grid.Columns[1].Width);
        Assert.Equal(new DataGridLength(150, DataGridLengthUnitType.Pixel), Table("Other.grid").Grid.Columns[0].Width);

        table.DesignedWidths();
        Assert.Equal(new DataGridLength(150, DataGridLengthUnitType.Pixel), table.Grid.Columns[0].Width);
        Assert.Equal(new DataGridLength(150, DataGridLengthUnitType.Pixel), Table("Main.grid").Grid.Columns[0].Width);
    }
}
