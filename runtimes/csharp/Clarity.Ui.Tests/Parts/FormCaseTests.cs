// Every case of conformance/forms.json run on one whole form, as the Avalonia
// FormCaseTests did: the text inputs, the checkbox and switch, the dropdowns,
// the radio group, the date and the moment side by side, so the cases that
// mix them (a record read again, then ticked and typed) and the date and
// time cases run from the shared file too, not restated by hand.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class FormCaseTests
{
    static readonly JsonElement Forms = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "forms.json"))).RootElement;

    static readonly Dictionary<string, string> Tags = new()
    {
        ["text"] = "Textbox",
        ["memo"] = "Memo",
        ["number"] = "Number",
        ["check"] = "Checkbox",
        ["switch"] = "Switch",
        ["choice"] = "Dropdown",
        ["radio"] = "Radio",
        ["date"] = "Date",
        ["moment"] = "Date",
    };

    static readonly Dictionary<string, string> Kinds = Forms.GetProperty("fields").EnumerateArray()
        .ToDictionary(f => f.GetProperty("field").GetString()!, f => f.GetProperty("kind").GetString()!);

    /// <summary>A value of the file as the record holds it: a decimal for a number, a day, a moment, else as it is.</summary>
    static object? Stored(string field, JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        JsonValueKind.Number => value.GetInt64(),
        _ => Kinds[field] switch
        {
            "number" => decimal.Parse(value.GetString()!, CultureInfo.InvariantCulture),
            "date" => DateOnly.ParseExact(value.GetString()!, "yyyy-MM-dd", CultureInfo.InvariantCulture),
            "moment" => DateTime.ParseExact(value.GetString()!, "yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture),
            _ => value.GetString(),
        },
    };

    /// <summary>A value as one text, the same for the file's and the record's.</summary>
    static string Written(object? value) => value switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        decimal d => d.ToString(CultureInfo.InvariantCulture),
        DateOnly d => d.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture),
        DateTime t => t.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture),
        _ => Convert.ToString(value, CultureInfo.InvariantCulture)!,
    };

    static void Load(TextFactsHost host)
    {
        foreach (var field in Forms.GetProperty("record").EnumerateObject())
            host.Values[field.Name] = Stored(field.Name, field.Value);
    }

    sealed record Form(TextFactsHost Host, Screen Screen, Dictionary<string, Part> Parts);

    static Form Make()
    {
        var host = new TextFactsHost();
        host.Formatted["hours"] = new FieldFormat(FormatKind.Decimal, 1);
        host.Formatted["day"] = new FieldFormat(FormatKind.Date);
        host.Formatted["due"] = new FieldFormat(FormatKind.DateTime);
        Load(host);
        var markup = new System.Text.StringBuilder();
        var row = 1;
        foreach (var field in Forms.GetProperty("fields").EnumerateArray())
        {
            var name = field.GetProperty("field").GetString()!;
            if (field.TryGetProperty("choices", out var choices))
                host.Offered[name] = [.. choices.EnumerateArray().Select(c => new Choice(c[0].ValueKind == JsonValueKind.Number ? c[0].GetInt64() : c[0].ValueKind == JsonValueKind.Null ? null : c[0].GetString(), c[1].GetString()!))];
            var rows = Kinds[name] == "memo" ? " rows=\"2\"" : "";
            markup.Append($"""<{Tags[Kinds[name]]} at="1,{row}" span="6"{rows} name="{name}" bind="{name}" caption="{field.GetProperty("label").GetString()}" onchange="changed" />""");
            row += Kinds[name] == "memo" ? 2 : 1;
        }
        var (screen, form, parts) = TextWindow.InForm(markup.ToString(), host, 800, 900);
        form.Submitted = () => host.Events.Add("submit");
        Shot.Of(screen, 800, 900).Dispose();
        return new Form(host, screen, parts.ToDictionary(p => p.Spec.Name!, p => p));
    }

    static TextEditor EditorOf(Part part) => part switch
    {
        TextBoxPart t => t.Editor,
        NumberPart n => n.Editor,
        // A moment's typed value in these cases is its time.
        DatePart { IsMoment: true } d => d.Time!,
        DatePart d => d.Editor,
        _ => throw new InvalidOperationException($"no editor in {part.Spec.Kind}"),
    };

    static void Click(Screen screen, double x, double y)
    {
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    static void Run(Form form, JsonElement step)
    {
        var screen = form.Screen;
        var part = step.GetArrayLength() > 1 ? form.Parts[step[1].GetString()!] : null;
        switch (step[0].GetString())
        {
            case "focus":
                screen.Focus(EditorOf(part!));
                break;
            case "type":
                TextWindow.Type(screen, EditorOf(part!), step[2].GetString()!);
                break;
            case "enter":
                screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
                break;
            case "leave":
                screen.Focus(null);
                break;
            case "tick":
                Click(screen, part!.Bounds.X + 8, part.Bounds.Y + part.Bounds.Height / 2);
                break;
            case "pick" when part is DropdownPart dropdown:
                var at = dropdown.Choices.ToList().FindIndex(c => Written(c.Value) == Written(Stored(step[1].GetString()!, step[2])));
                dropdown.Open();
                var list = screen.Overlays[^1].Place;
                Click(screen, list.X + 10, list.Y + 1 + (at + 0.5) * DropdownList.Line);
                break;
            case "pick" when part is RadioPart radio:
                var choice = radio.Choices.ToList().FindIndex(c => Written(c.Value) == Written(Stored(step[1].GetString()!, step[2])));
                var place = radio.Places(screen.Fonts)[choice];
                Click(screen, place.X + 4, place.Y + place.Height / 2);
                break;
            case "pickDay":
                var date = (DatePart)part!;
                Click(screen, date.Button.Bounds.X + date.Button.Bounds.Width / 2, date.Button.Bounds.Y + date.Button.Bounds.Height / 2);
                var calendar = (DayCalendar)screen.Overlays[^1].Content;
                var cell = calendar.CellOf(DateOnly.ParseExact(step[2].GetString()!, "yyyy-MM-dd", CultureInfo.InvariantCulture))!.Value;
                Click(screen, cell.X + cell.Width / 2, cell.Y + cell.Height / 2);
                break;
            case "handler":
                form.Host.Values[step[1].GetString()!] = Stored(step[1].GetString()!, step[2]);
                form.Host.Change();
                break;
            case "reload":
                Load(form.Host);
                form.Host.Change();
                break;
            default:
                throw new InvalidOperationException($"no step {step[0]}");
        }
        Shot.Of(screen, 800, 900).Dispose();
    }

    static bool Marked(Part part) => part switch
    {
        NumberPart n => n.Bad is not null,
        DatePart d => d.Bad is not null || d.BadTime is not null,
        _ => false,
    };

    [Fact]
    public void Every_case_behaves_as_on_every_renderer()
    {
        var ran = 0;
        foreach (var c in Forms.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var form = Make();
            foreach (var step in c.GetProperty("steps").EnumerateArray())
                Run(form, step);
            var expect = c.GetProperty("expect");
            var events = form.Host.Events.Where(e => e.StartsWith("change ") || e == "submit").ToList();
            Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()).SequenceEqual(events), $"{name}: events {string.Join(", ", events)}");
            if (expect.TryGetProperty("values", out var values))
                foreach (var v in values.EnumerateArray())
                {
                    var field = v[0].GetString()!;
                    Assert.True(Written(Stored(field, v[1])) == Written(form.Host.Values[field]), $"{name}: {field} is {Written(form.Host.Values[field])}");
                }
            var marked = expect.TryGetProperty("marked", out var m) ? m.EnumerateArray().Select(e => e.GetString()!).ToHashSet() : [];
            foreach (var (field, part) in form.Parts)
                Assert.True(marked.Contains(field) == Marked(part), $"{name}: {field} marked {Marked(part)}");
            if (expect.TryGetProperty("shown", out var shown))
                foreach (var s in shown.EnumerateArray())
                {
                    var part = form.Parts[s[0].GetString()!];
                    var text = part is DropdownPart d ? d.Shown : EditorOf(part).Text;
                    Assert.True(s[1].GetString() == text, $"{name}: {s[0]} shows {text}");
                }
            if (expect.TryGetProperty("choices", out var choices))
                foreach (var field in choices.EnumerateObject())
                {
                    var offered = form.Parts[field.Name] switch
                    {
                        DropdownPart d => d.Choices,
                        RadioPart r => r.Choices,
                        _ => [],
                    };
                    Assert.Equal(field.Value.EnumerateArray().Select(l => l.GetString()), offered.Select(o => o.Label));
                }
            ran++;
        }
        Assert.Equal(Forms.GetProperty("cases").GetArrayLength(), ran);
    }
}
