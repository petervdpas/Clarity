// How a table with edited columns behaves, against the cases every renderer
// shares (conformance/tables.json), run through the real ClarityTable with
// real controls: clicks and ticks go through the pointer, typing through
// the text boxes.

using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// The task the cases show, as a generated record: it tells when a field changes.
sealed class CaseTask : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;
    string title = "";
    bool done;
    bool urgent;
    object? priority;
    long? owner;
    DateTime? due;
    DateOnly? day;
    decimal hours;
    string notes = "";
    public string Title { get => title; set => Set(ref title, value); }
    public bool Done { get => done; set => Set(ref done, value); }
    public bool Urgent { get => urgent; set => Set(ref urgent, value); }
    public object? Priority { get => priority; set => Set(ref priority, value); }
    public long? Owner { get => owner; set => Set(ref owner, value); }
    public DateTime? Due { get => due; set => Set(ref due, value); }
    public DateOnly? Day { get => day; set => Set(ref day, value); }
    public decimal Hours { get => hours; set => Set(ref hours, value); }
    public string Notes { get => notes; set => Set(ref notes, value); }

    void Set<T>(ref T field, T value, [CallerMemberName] string name = "")
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
            return;
        field = value;
        PropertyChanged?.Invoke(this, new(name));
    }
}

public class TableCaseTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "tables.json"))).RootElement;

    static string Pascal(string field) => char.ToUpperInvariant(field[0]) + field[1..];

    static object? Plain(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.String => value.GetString(),
        JsonValueKind.Number => value.GetInt64(),
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => throw new ArgumentException($"no value {value}"),
    };

    /// A value of the cases as the record's property holds it.
    static object? Typed(string field, JsonElement value) => (field, value.ValueKind) switch
    {
        (_, JsonValueKind.Null) => null,
        ("due", _) => DateTime.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        ("day", _) => DateOnly.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        ("hours", _) => decimal.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        _ => Plain(value),
    };

    /// A record's value as the cases write it.
    static string Written(object? value) => value switch
    {
        null => "null",
        DateTime t => t.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture),
        DateOnly d => d.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture),
        decimal n => n.ToString(CultureInfo.InvariantCulture),
        bool b => b ? "true" : "false",
        _ => Convert.ToString(value, CultureInfo.InvariantCulture)!,
    };

    static CaseTask[] Rows() => File.GetProperty("rows").EnumerateArray().Select(row =>
    {
        var task = new CaseTask();
        foreach (var field in row.EnumerateObject())
            typeof(CaseTask).GetProperty(Pascal(field.Name))!.SetValue(task, Typed(field.Name, field.Value));
        return task;
    }).ToArray();

    static List<TableColumn> Columns() => File.GetProperty("columns").EnumerateArray().Select(c => new TableColumn
    {
        Field = Pascal(c.GetProperty("field").GetString()!),
        Header = c.GetProperty("label").GetString()!,
        Kind = Enum.Parse<CellKind>(c.GetProperty("kind").GetString()!, ignoreCase: true),
        Edit = c.GetProperty("edit").GetBoolean(),
        Switch = c.TryGetProperty("switch", out var s) && s.GetBoolean(),
        Choices = c.TryGetProperty("choices", out var choices)
            ? choices.EnumerateArray().Select(p => new Choice(Plain(p[0]), p[1].GetString()!)).ToArray()
            : null,
        Number = c.GetProperty("kind").GetString() == "number" ? new EditConverter(new FieldFormat(FormatKind.Decimal, c.GetProperty("scale").GetInt32())) : null,
    }).ToList();

    [AvaloniaFact]
    public void EveryCaseBehavesAsOnEveryRenderer()
    {
        Locale.Start(null, [], null, (_, _) => null, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
        try
        {
            var cases = File.GetProperty("cases").EnumerateArray().ToList();
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
        var rows = Rows();
        var handling = false;
        var table = new ClarityTable { ItemsSource = rows, Handling = () => handling };
        foreach (var column in Columns())
            table.Columns.Add(column);
        var elsewhere = new Button { Content = "elsewhere" };
        var window = new Window { Width = 1200, Height = 400, Content = new DockPanel { Children = { elsewhere, table } } };
        DockPanel.SetDock(elsewhere, Dock.Bottom);
        var events = new List<string>();
        table.Selected += (_, _) => events.Add($"select {Array.IndexOf(rows, table.SelectedItem)}");
        table.Edited += row => events.Add($"change {Array.IndexOf(rows, row)}");
        window.Show();
        Dispatcher.UIThread.RunJobs();

        T Cell<T>(int row, string field) where T : Control =>
            table.CellOf(rows[row], Pascal(field)) is { } cell
                ? cell.GetSelfAndVisualDescendants().OfType<T>().FirstOrDefault() ?? throw new InvalidOperationException($"{name}: no {typeof(T).Name} in {field} of row {row}")
                : throw new InvalidOperationException($"{name}: no cell {field} of row {row}");
        void Click(Control control)
        {
            var middle = control.TranslatePoint(new Point(control.Bounds.Width / 2, control.Bounds.Height / 2), window)!.Value;
            window.MouseDown(middle, MouseButton.Left);
            window.MouseUp(middle, MouseButton.Left);
            Dispatcher.UIThread.RunJobs();
        }
        TextBox TextOf(int row, string field) =>
            table.CellOf(rows[row], Pascal(field))?.GetSelfAndVisualDescendants().OfType<DateTimeBox>().FirstOrDefault() is { } moment ? moment.Time : Cell<TextBox>(row, field);

        foreach (var step in @case.GetProperty("steps").EnumerateArray())
        {
            var what = step[0].GetString();
            int Row() => step[1].GetInt32();
            string Field() => step[2].GetString()!;
            switch (what)
            {
                case "click":
                    Click(Cell<TextBlock>(Row(), "notes"));
                    break;
                case "tick":
                    Click(Cell<ToggleButton>(Row(), Field()) is { } box ? box : throw new InvalidOperationException());
                    break;
                case "pick":
                    var choices = Cell<ChoiceBox>(Row(), Field());
                    var wanted = Plain(step[3]);
                    choices.Combo.SelectedItem = choices.Combo.Items.OfType<Choice>().First(c => Equals(c.Value, wanted));
                    break;
                case "pickDay":
                    var day = DateTime.Parse(step[3].GetString()!, CultureInfo.InvariantCulture);
                    var picker = Cell<CalendarDatePicker>(Row(), Field());
                    picker.SelectedDate = day;
                    break;
                case "type":
                    var text = TextOf(Row(), Field());
                    text.Focus();
                    text.Text = step[3].GetString();
                    break;
                case "enter":
                    window.KeyPressQwerty(PhysicalKey.Enter, RawInputModifiers.None);
                    break;
                case "leave":
                    elsewhere.Focus();
                    break;
                case "handler":
                    handling = true;
                    typeof(CaseTask).GetProperty(Pascal(Field()))!.SetValue(rows[Row()], Typed(Field(), step[3]));
                    handling = false;
                    break;
                case "reload":
                    rows = Rows();
                    table.ItemsSource = rows;
                    break;
                default:
                    throw new ArgumentException($"{name}: unknown step {what}");
            }
            Dispatcher.UIThread.RunJobs();
        }

        var expect = @case.GetProperty("expect");
        Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()!).SequenceEqual(events), $"{name}: events were [{string.Join(", ", events)}]");

        // Every value as it started, but those the case changes.
        var start = Rows();
        var changed = expect.TryGetProperty("values", out var values) ? values.EnumerateArray().ToList() : [];
        foreach (var field in File.GetProperty("columns").EnumerateArray().Select(c => c.GetProperty("field").GetString()!))
            for (var row = 0; row < rows.Length; row++)
            {
                var property = typeof(CaseTask).GetProperty(Pascal(field))!;
                var change = changed.FirstOrDefault(v => v[0].GetInt32() == row && v[1].GetString() == field);
                var wanted = change.ValueKind == JsonValueKind.Undefined ? Written(property.GetValue(start[row])) : Written(Typed(field, change[2]));
                Assert.True(wanted == Written(property.GetValue(rows[row])), $"{name}: {field} of row {row} is {Written(property.GetValue(rows[row]))}, not {wanted}");
            }

        if (expect.TryGetProperty("marked", out var marked))
            foreach (var m in marked.EnumerateArray())
            {
                var box = TextOf(m[0].GetInt32(), m[1].GetString()!);
                Assert.True(DataValidationErrors.GetHasErrors(box) || box.Classes.Contains("error"), $"{name}: {m[1]} of row {m[0]} is not marked");
            }
        if (expect.TryGetProperty("shown", out var shown))
            foreach (var s in shown.EnumerateArray())
            {
                var (row, field) = (s[0].GetInt32(), s[1].GetString()!);
                var cell = table.CellOf(rows[row], Pascal(field))!;
                var text = cell.GetSelfAndVisualDescendants().OfType<DateTimeBox>().FirstOrDefault() is { } moment
                    ? moment.Time.Text
                    : cell.GetSelfAndVisualDescendants().OfType<TextBox>().FirstOrDefault()?.Text ?? cell.GetSelfAndVisualDescendants().OfType<TextBlock>().First().Text;
                Assert.True(s[2].GetString() == (text ?? ""), $"{name}: {field} of row {row} shows '{text}'");
            }
        if (expect.TryGetProperty("choices", out var expectedChoices))
            foreach (var field in expectedChoices.EnumerateObject())
            {
                var labels = Cell<ChoiceBox>(0, field.Name).Combo.Items.OfType<Choice>().Select(c => c.Label);
                Assert.True(field.Value.EnumerateArray().Select(l => l.GetString()!).SequenceEqual(labels), $"{name}: the choices of {field.Name} are [{string.Join(", ", labels)}]");
            }
        window.Close();
    }
}
