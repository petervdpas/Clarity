// How the inputs of a form behave, against the cases every renderer shares
// (conformance/forms.json), run through the real controls of the library:
// ticks and picks go through the pointer, typing through the text boxes,
// Enter through the keyboard, inside a form whose submit is wired.

using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// The task the form shows, as a generated record: it tells when a field changes.
sealed class FormTask : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;
    string name = "";
    string notes = "";
    decimal hours;
    bool done;
    bool urgent;
    object? priority;
    long? owner;
    object? size;
    DateOnly? day;
    DateTime? due;
    public string Name { get => name; set => Set(ref name, value); }
    public string Notes { get => notes; set => Set(ref notes, value); }
    public decimal Hours { get => hours; set => Set(ref hours, value); }
    public bool Done { get => done; set => Set(ref done, value); }
    public bool Urgent { get => urgent; set => Set(ref urgent, value); }
    public object? Priority { get => priority; set => Set(ref priority, value); }
    public long? Owner { get => owner; set => Set(ref owner, value); }
    public object? Size { get => size; set => Set(ref size, value); }
    public DateOnly? Day { get => day; set => Set(ref day, value); }
    public DateTime? Due { get => due; set => Set(ref due, value); }

    void Set<T>(ref T field, T value, [CallerMemberName] string name = "")
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
            return;
        field = value;
        PropertyChanged?.Invoke(this, new(name));
    }
}

public class FormCaseTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "forms.json"))).RootElement;

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

    static FormTask Record()
    {
        var task = new FormTask();
        foreach (var field in File.GetProperty("record").EnumerateObject())
            typeof(FormTask).GetProperty(Pascal(field.Name))!.SetValue(task, Typed(field.Name, field.Value));
        return task;
    }

    static Choice[] ChoicesOf(JsonElement field) =>
        field.GetProperty("choices").EnumerateArray().Select(p => new Choice(Plain(p[0]), p[1].GetString()!)).ToArray();

    /// The library's input for a field of the cases, bound to the record's property.
    static Control Input(JsonElement field)
    {
        var path = Pascal(field.GetProperty("field").GetString()!);
        var bound = new Binding(path) { Mode = BindingMode.TwoWay };
        return field.GetProperty("kind").GetString() switch
        {
            "text" => new TextInput { [!TextInput.ValueProperty] = bound },
            "memo" => new TextInput { AcceptsReturn = true, [!TextInput.ValueProperty] = bound },
            "number" => new NumberInput { Format = new FieldFormat(FormatKind.Decimal, field.GetProperty("scale").GetInt32()), [!NumberInput.ValueProperty] = bound },
            "check" => new CheckInput { Content = field.GetProperty("label").GetString(), [!CheckInput.IsCheckedProperty] = bound },
            "switch" => new SwitchInput { [!SwitchInput.IsCheckedProperty] = bound },
            "choice" => new ChoiceInput { Choices = ChoicesOf(field), [!ChoiceInput.ValueProperty] = bound },
            "radio" => new RadioInput { Choices = ChoicesOf(field), [!RadioInput.ValueProperty] = bound },
            "date" => new DateInput { [!DateInput.ValueProperty] = bound },
            "moment" => new DateTimeInput { [!DateTimeInput.ValueProperty] = bound },
            var kind => throw new ArgumentException($"no input for {kind}"),
        };
    }

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
        var events = new List<string>();
        var fields = File.GetProperty("fields").EnumerateArray().ToList();
        var inputs = new Dictionary<string, Control>();
        var form = new StackPanel { Spacing = 4 };
        foreach (var field in fields)
        {
            var key = field.GetProperty("field").GetString()!;
            var input = Input(field);
            ((IInput)input).ValueChanged += (_, _) => events.Add($"change {key}");
            inputs[key] = input;
            form.Children.Add(input);
        }
        Events.OnEnter(form, () => events.Add("submit"));
        var elsewhere = new Button { Content = "elsewhere" };
        form.Children.Add(elsewhere);
        var window = new Window { Width = 600, Height = 900, Content = form, DataContext = Record() };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        FormTask Task() => (FormTask)window.DataContext!;

        void Click(Control control)
        {
            var middle = control.TranslatePoint(new Point(control.Bounds.Width / 2, control.Bounds.Height / 2), window)!.Value;
            window.MouseDown(middle, MouseButton.Left);
            window.MouseUp(middle, MouseButton.Left);
            Dispatcher.UIThread.RunJobs();
        }
        Control Of(string field) => inputs.TryGetValue(field, out var input) ? input : throw new InvalidOperationException($"{name}: no field {field}");
        TextBox TextOf(string field) => Of(field) switch
        {
            DateTimeInput moment => moment.Time,
            TextBox box => box,
            var other => throw new InvalidOperationException($"{name}: {field} is a {other.GetType().Name}, no text"),
        };

        foreach (var step in @case.GetProperty("steps").EnumerateArray())
        {
            var what = step[0].GetString();
            string Field() => step[1].GetString()!;
            switch (what)
            {
                case "focus":
                    TextOf(Field()).Focus();
                    break;
                case "type":
                    var text = TextOf(Field());
                    text.Focus();
                    text.Text = step[2].GetString();
                    break;
                case "enter":
                    window.KeyPressQwerty(PhysicalKey.Enter, RawInputModifiers.None);
                    break;
                case "leave":
                    elsewhere.Focus();
                    break;
                case "tick":
                    Click(Of(Field()));
                    break;
                case "pick":
                    var wanted = Plain(step[2]);
                    switch (Of(Field()))
                    {
                        case ChoiceInput choices:
                            choices.Combo.SelectedItem = choices.Combo.Items.OfType<Choice>().First(c => Equals(c.Value, wanted));
                            break;
                        case RadioInput radio:
                            Click(radio.ButtonOf(wanted) ?? throw new InvalidOperationException($"{name}: no button for {wanted}"));
                            break;
                    }
                    break;
                case "pickDay":
                    var day = DateTime.Parse(step[2].GetString()!, CultureInfo.InvariantCulture);
                    switch (Of(Field()))
                    {
                        case DateInput date:
                            date.SelectedDate = day;
                            break;
                        case DateTimeInput moment:
                            moment.Day.SelectedDate = day;
                            break;
                    }
                    break;
                case "handler":
                    typeof(FormTask).GetProperty(Pascal(Field()))!.SetValue(Task(), Typed(Field(), step[2]));
                    break;
                case "reload":
                    window.DataContext = Record();
                    break;
                default:
                    throw new ArgumentException($"{name}: unknown step {what}");
            }
            Dispatcher.UIThread.RunJobs();
        }

        var expect = @case.GetProperty("expect");
        Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()!).SequenceEqual(events), $"{name}: events were [{string.Join(", ", events)}]");

        // Every value as it started, but those the case changes.
        var start = Record();
        var changed = expect.TryGetProperty("values", out var values) ? values.EnumerateArray().ToList() : [];
        foreach (var field in fields.Select(f => f.GetProperty("field").GetString()!))
        {
            var property = typeof(FormTask).GetProperty(Pascal(field))!;
            var change = changed.FirstOrDefault(v => v[0].GetString() == field);
            var wanted = change.ValueKind == JsonValueKind.Undefined ? Written(property.GetValue(start)) : Written(Typed(field, change[1]));
            Assert.True(wanted == Written(property.GetValue(Task())), $"{name}: {field} is {Written(property.GetValue(Task()))}, not {wanted}");
        }

        if (expect.TryGetProperty("marked", out var marked))
            foreach (var m in marked.EnumerateArray())
                Assert.True(TextOf(m.GetString()!).Classes.Contains("error"), $"{name}: {m} is not marked");
        if (expect.TryGetProperty("shown", out var shown))
            foreach (var s in shown.EnumerateArray())
            {
                var field = s[0].GetString()!;
                var text = Of(field) is ChoiceInput choice ? (choice.Combo.SelectedItem as Choice)?.Label : TextOf(field).Text;
                Assert.True(s[1].GetString() == (text ?? ""), $"{name}: {field} shows '{text}'");
            }
        if (expect.TryGetProperty("choices", out var expectedChoices))
            foreach (var field in expectedChoices.EnumerateObject())
            {
                var labels = Of(field.Name) switch
                {
                    ChoiceInput choice => choice.Combo.Items.OfType<Choice>().Select(c => c.Label).ToList(),
                    RadioInput radio => radio.Buttons.Select(b => b.Content as string ?? "").ToList(),
                    _ => [],
                };
                Assert.True(field.Value.EnumerateArray().Select(l => l.GetString()!).SequenceEqual(labels), $"{name}: the choices of {field.Name} are [{string.Join(", ", labels)}]");
            }
        window.Close();
    }
}
