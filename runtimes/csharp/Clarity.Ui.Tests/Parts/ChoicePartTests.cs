// The checkbox, switch, dropdown and radio group of a form against the cases
// every renderer shares (conformance/forms.json): ticks and picks are the
// person's and change the record; what a handler sets is not. The cases
// that type, enter or leave are the text inputs'.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class ChoicePartTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "forms.json"))).RootElement;

    static readonly Dictionary<string, string> Tags = new() { ["check"] = "Checkbox", ["switch"] = "Switch", ["choice"] = "Dropdown", ["radio"] = "Radio" };

    static readonly string[] Steps = ["tick", "pick", "handler", "reload"];

    static object? Value(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        JsonValueKind.Number => value.GetInt64(),
        _ => value.GetString(),
    };

    sealed record Form(TestHost Host, Screen Screen, Dictionary<string, Part> Parts);

    static Form Make()
    {
        var host = new TestHost();
        var markup = new System.Text.StringBuilder();
        var row = 1;
        var parts = new List<string>();
        foreach (var field in File.GetProperty("fields").EnumerateArray())
        {
            var name = field.GetProperty("field").GetString()!;
            if (!Tags.TryGetValue(field.GetProperty("kind").GetString()!, out var tag))
                continue;
            host.Values[$"task.{name}"] = Value(File.GetProperty("record").GetProperty(name));
            if (field.TryGetProperty("choices", out var choices))
                host.Offered[$"task.{name}"] = [.. choices.EnumerateArray().Select(c => new Choice(Value(c[0]), c[1].GetString()!))];
            markup.Append($"""<{tag} at="1,{row++}" span="6" name="{name}" bind="task.{name}" onchange="changed" />""");
            parts.Add(name);
        }
        var (screen, _, made) = Made.Window(markup.ToString(), host, 800, 600);
        return new Form(host, screen, parts.Zip(made).ToDictionary(p => p.First, p => p.Second));
    }

    static void Run(Form form, JsonElement step)
    {
        var what = step[0].GetString();
        switch (what)
        {
            case "tick":
                var check = (CheckPart)form.Parts[step[1].GetString()!];
                form.Screen.PointerPressed(check.Bounds.X + 8, check.Bounds.Y + 18, PointerButton.Left);
                form.Screen.PointerReleased(check.Bounds.X + 8, check.Bounds.Y + 18, PointerButton.Left);
                break;
            case "pick":
                var part = form.Parts[step[1].GetString()!];
                var value = Value(step[2]);
                if (part is DropdownPart dropdown)
                {
                    var at = dropdown.Choices.ToList().FindIndex(c => Equals(c.Value, value));
                    dropdown.Open();
                    var list = form.Screen.Overlays[^1].Place;
                    form.Screen.PointerPressed(list.X + 10, list.Y + 1 + (at + 0.5) * DropdownList.Line, PointerButton.Left);
                    form.Screen.PointerReleased(list.X + 10, list.Y + 1 + (at + 0.5) * DropdownList.Line, PointerButton.Left);
                }
                else
                {
                    var radio = (RadioPart)part;
                    var at = radio.Choices.ToList().FindIndex(c => Equals(c.Value, value));
                    var place = radio.Places(form.Screen.Fonts)[at];
                    form.Screen.PointerPressed(place.X + 4, place.Y + 12, PointerButton.Left);
                    form.Screen.PointerReleased(place.X + 4, place.Y + 12, PointerButton.Left);
                }
                break;
            case "handler":
                form.Host.Values[$"task.{step[1].GetString()}"] = Value(step[2]);
                form.Host.Change();
                break;
            case "reload":
                form.Host.Change();
                break;
        }
    }

    [Fact]
    public void Every_case_of_the_choices_behaves_as_on_every_renderer()
    {
        var ran = 0;
        foreach (var c in File.GetProperty("cases").EnumerateArray())
        {
            if (!c.GetProperty("steps").EnumerateArray().All(s => Steps.Contains(s[0].GetString())))
                continue;
            ran++;
            var name = c.GetProperty("name").GetString();
            var form = Make();
            foreach (var step in c.GetProperty("steps").EnumerateArray())
                Run(form, step);
            var expect = c.GetProperty("expect");
            var events = form.Host.Log.Where(l => l.StartsWith("run ")).Select(l => $"change {l.Split(' ')[1]}").ToList();
            Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()).SequenceEqual(events), $"{name}: {string.Join(", ", events)}");
            if (expect.TryGetProperty("values", out var values))
                foreach (var v in values.EnumerateArray().Where(v => form.Parts.ContainsKey(v[0].GetString()!)))
                    Assert.True(Equals(Value(v[1]), form.Host.Values[$"task.{v[0].GetString()}"]), $"{name}: {v[0]}");
            if (expect.TryGetProperty("shown", out var shown))
                foreach (var s in shown.EnumerateArray().Where(s => form.Parts.GetValueOrDefault(s[0].GetString()!) is DropdownPart))
                    Assert.Equal(s[1].GetString(), ((DropdownPart)form.Parts[s[0].GetString()!]).Shown);
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
        }
        Assert.True(ran >= 8, $"only {ran} cases ran");
    }
}
