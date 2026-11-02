// A text box, a memo and a password against the cases every stack shares
// (conformance/forms.json), with the number box of the same form: typing
// goes through the screen, Enter through the keys, leaving through the
// focus, inside a form whose submit is wired. A password's eye and copy
// button, and an unbound input that keeps its text in its state.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

/// <summary>A host that knows its fields' formats, ranges and required marks, in the person's formats (en-US unless a test sets others).</summary>
class TextFactsHost : TestHost
{
    public readonly Dictionary<string, FieldFormat> Formatted = [];
    public readonly HashSet<string> Required = [];
    public readonly Dictionary<string, (string?, string?)> Ranges = [];
    public readonly List<string> Events = [];

    public TextFactsHost() => Formats = new(CultureInfo.GetCultureInfo("en-US"), null);

    public override FieldFacts? Field(string path)
    {
        var known = base.Field(path);
        if (known is null && !Formatted.ContainsKey(path) && !Required.Contains(path) && !Ranges.ContainsKey(path))
            return null;
        var (min, max) = Ranges.GetValueOrDefault(path);
        return (known ?? new FieldFacts(path)) with
        {
            Format = Formatted.GetValueOrDefault(path) ?? known?.Format,
            Required = Required.Contains(path) || known?.Required == true,
            Min = min ?? known?.Min,
            Max = max ?? known?.Max,
        };
    }

    public override bool Run(string? part, HandlerCall call)
    {
        Events.Add($"{call.Event} {part}");
        return base.Run(part, call);
    }
}

/// <summary>A form around a grid of parts: Enter in its inputs submits it.</summary>
sealed class TextTestForm : Element, ISubmits
{
    public readonly GridPanel Grid = new(columns: 12);
    public Action Submitted = () => { };

    public TextTestForm() => AddChild(Grid);

    public override void Arrange(Box box)
    {
        Bounds = box;
        Grid.Arrange(box);
    }

    public bool Submit()
    {
        Submitted();
        return true;
    }
}

static class TextWindow
{
    /// <summary>The parts of <paramref name="markup"/> in a form, on a screen.</summary>
    public static (Screen Screen, TextTestForm Form, List<Part> Parts) InForm(string markup, IPartHost host, double width = 800, double height = 400)
    {
        var spec = WindowMarkup.Read($"<Window name=\"Test\">{markup}</Window>");
        var form = new TextTestForm();
        var parts = new List<Part>();
        foreach (var p in spec.Parts)
        {
            var part = PartKinds.Make(p, host);
            parts.Add(part);
            form.Grid.Add(part, p.Place(part.Grows));
        }
        var screen = new Screen(form, ThemeLook.Clarity(dark: false));
        screen.Resize(width, height, 1);
        return (screen, form, parts);
    }

    /// <summary>Puts <paramref name="text"/> in an editor as the person types it, without taking it.</summary>
    public static void Type(Screen screen, TextEditor editor, string text)
    {
        screen.Focus(editor);
        editor.SelectAll();
        screen.KeyPressed(new KeyInput(Key.Backspace, Modifiers.None));
        if (text.Length > 0)
            screen.TextTyped(text);
    }
}

public class TextBoxPartTests
{
    static readonly JsonElement Forms = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "forms.json"))).RootElement;

    /// <summary>The fields of the cases these parts show.</summary>
    static readonly string[] Mine = ["name", "notes", "hours"];

    const string Form = """
        <Textbox at="1,1" span="6" name="name" bind="name" onchange="changed" />
        <Memo at="1,2" span="6" rows="3" name="notes" bind="notes" onchange="changed" />
        <Number at="1,5" span="3" name="hours" bind="hours" onchange="changed" />
        """;

    static object? Stored(string field, JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        _ when field == "hours" => decimal.Parse(value.GetString()!, CultureInfo.InvariantCulture),
        JsonValueKind.String => value.GetString(),
        _ => value.ToString(),
    };

    static string Written(object? value) => value switch
    {
        null => "null",
        decimal d => d.ToString(CultureInfo.InvariantCulture),
        _ => Convert.ToString(value, CultureInfo.InvariantCulture)!,
    };

    static void Load(TextFactsHost host)
    {
        foreach (var field in Forms.GetProperty("record").EnumerateObject())
            if (Mine.Contains(field.Name))
                host.Values[field.Name] = Stored(field.Name, field.Value);
    }

    /// <summary>Whether a case only concerns the fields these parts show.</summary>
    static bool Concerns(JsonElement c)
    {
        foreach (var step in c.GetProperty("steps").EnumerateArray())
        {
            var what = step[0].GetString();
            if (what is "tick" or "pick" or "pickDay")
                return false;
            if (step.GetArrayLength() > 1 && !Mine.Contains(step[1].GetString()))
                return false;
        }
        var expect = c.GetProperty("expect");
        if (expect.TryGetProperty("values", out var values) && values.EnumerateArray().Any(v => !Mine.Contains(v[0].GetString())))
            return false;
        return expect.GetProperty("events").EnumerateArray().All(e => Mine.Contains(e.GetString()!.Split(' ').Last()) || e.GetString() == "submit");
    }

    [Fact]
    public void Every_form_case_of_text_memo_and_number_behaves_as_on_every_renderer()
    {
        var run = 0;
        foreach (var c in Forms.GetProperty("cases").EnumerateArray())
        {
            if (Concerns(c))
            {
                Run(c);
                run++;
            }
        }
        Assert.True(run >= 12, $"only {run} cases ran");
    }

    static void Run(JsonElement c)
    {
        var name = c.GetProperty("name").GetString();
        var host = new TextFactsHost();
        host.Formatted["hours"] = new FieldFormat(FormatKind.Decimal, 1);
        Load(host);
        var (screen, form, parts) = TextWindow.InForm(Form, host);
        form.Submitted = () => host.Events.Add("submit");
        TextEditor EditorOf(string field) => parts.First(p => p.Spec.Name == field) switch
        {
            TextBoxPart t => t.Editor,
            NumberPart n => n.Editor,
            var other => throw new InvalidOperationException($"{name}: no input for {other.Spec.Kind}"),
        };
        foreach (var step in c.GetProperty("steps").EnumerateArray())
        {
            switch (step[0].GetString())
            {
                case "focus":
                    screen.Focus(EditorOf(step[1].GetString()!));
                    break;
                case "type":
                    TextWindow.Type(screen, EditorOf(step[1].GetString()!), step[2].GetString()!);
                    break;
                case "enter":
                    screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
                    break;
                case "leave":
                    screen.Focus(null);
                    break;
                case "handler":
                    host.Values[step[1].GetString()!] = Stored(step[1].GetString()!, step[2]);
                    host.Change();
                    break;
                case "reload":
                    Load(host);
                    host.Change();
                    break;
                default:
                    throw new InvalidOperationException($"{name}: no step {step[0]}");
            }
        }
        using (Shot.Of(screen, 800, 400)) { }
        var expect = c.GetProperty("expect");
        Assert.True(expect.GetProperty("events").EnumerateArray().Select(e => e.GetString()).SequenceEqual(host.Events), $"{name}: events {string.Join(", ", host.Events)}");
        if (expect.TryGetProperty("values", out var values))
            foreach (var v in values.EnumerateArray())
                Assert.True(Written(Stored(v[0].GetString()!, v[1])) == Written(host.Values[v[0].GetString()!]), $"{name}: {v[0]} is {Written(host.Values[v[0].GetString()!])}");
        var marked = expect.TryGetProperty("marked", out var m) ? m.EnumerateArray().Select(e => e.GetString()).ToList() : [];
        var hours = (NumberPart)parts.First(p => p.Spec.Name == "hours");
        Assert.True(marked.Contains("hours") == (hours.Bad is not null), $"{name}: hours marked {hours.Bad}");
        if (expect.TryGetProperty("shown", out var shown))
            foreach (var s in shown.EnumerateArray().Where(s => Mine.Contains(s[0].GetString())))
                Assert.True(s[1].GetString() == EditorOf(s[0].GetString()!).Text, $"{name}: {s[0]} shows {EditorOf(s[0].GetString()!).Text}");
    }

    [Fact]
    public void Entering_and_leaving_an_input_run_its_handlers()
    {
        var host = new TextFactsHost();
        var (screen, _, parts) = TextWindow.InForm("""<Textbox at="1,1" span="4" name="note" onenter="came" onleave="went" />""", host);
        var box = (TextBoxPart)parts[0];
        screen.Focus(box.Editor);
        screen.Focus(null);
        Assert.Equal(["enter note", "leave note"], host.Events);
    }

    [Fact]
    public void An_unbound_named_input_keeps_its_text_in_its_state()
    {
        var host = new TextFactsHost();
        var (screen, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="search" onchange="look" />""", host);
        var box = (TextBoxPart)parts[0];
        TextWindow.Type(screen, box.Editor, "milk");
        screen.Focus(null);
        Assert.Equal("milk", host.States["search"].Text);
        Assert.Equal(["change search"], host.Events);
        // What code sets shows, and is no change.
        host.States["search"].Text = "bread";
        host.Change();
        Assert.Equal("bread", box.Editor.Text);
        Assert.Single(host.Events);
    }

    [Fact]
    public void Enter_outside_a_form_takes_the_text_and_is_not_taken()
    {
        var host = new TextFactsHost { Values = { ["name"] = "a" } };
        var (screen, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="name" bind="name" />""", host);
        var box = (TextBoxPart)parts[0];
        TextWindow.Type(screen, box.Editor, "b");
        Assert.False(box.Editor.OnKey(new KeyInput(Key.Enter, Modifiers.None)));
        Assert.Equal("b", host.Values["name"]);
    }

    [Fact]
    public void A_placeholder_is_translated_and_a_read_only_input_takes_no_typing()
    {
        var host = new TextFactsHost { Texts = { ["searchHint"] = "Search" } };
        var (screen, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="q" placeholder="@searchHint" />""", host);
        var box = (TextBoxPart)parts[0];
        Assert.Equal("Search", box.Editor.Placeholder);
        host.States["q"].IsReadOnly = true;
        TextWindow.Type(screen, box.Editor, "x");
        Assert.Equal("", box.Editor.Text);
        host.States["q"].IsEnabled = false;
        Assert.False(box.Editor.Focusable);
    }

    [Fact]
    public void A_memo_wraps_and_Enter_is_a_new_line()
    {
        var host = new TextFactsHost();
        var (screen, _, parts) = Made.Window("""<Memo at="1,1" span="4" rows="3" name="notes" />""", host);
        var memo = (TextBoxPart)parts[0];
        Assert.True(memo.Editor.Multiline);
        screen.Focus(memo.Editor);
        screen.TextTyped("one");
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        screen.TextTyped("two");
        Assert.Equal("one\ntwo", memo.Editor.Text);
        Assert.Empty(host.Events);
    }

    // -- A password (the old SecretInputTests) --------------------------------

    [Fact]
    public void The_eye_and_copy_say_what_they_do_when_the_pointer_rests()
    {
        var host = new TextFactsHost();
        var (_, _, parts) = Made.Window("""<Password at="1,1" span="4" name="secret" />""", host);
        var box = (TextBoxPart)parts[0];
        Assert.Equal(Locale.Translate("showSecret", "Show"), box.Eye!.TooltipAt(0, 0));
        Assert.Equal(Locale.Translate("copySecret", "Copy"), box.Copy!.TooltipAt(0, 0));
    }

    [Fact]
    public void The_eye_shows_what_is_typed_and_hides_it_again()
    {
        var host = new TextFactsHost { Values = { ["token"] = "s3cr3t" } };
        var (screen, _, parts) = Made.Window("""<Password at="1,1" span="4" name="token" bind="token" />""", host);
        var password = (TextBoxPart)parts[0];
        Assert.True(password.Editor.Password);
        Assert.Equal("Show", password.Eye!.Name);
        var eye = password.Eye.Bounds;
        screen.PointerPressed(eye.X + 5, eye.Y + 5, PointerButton.Left);
        screen.PointerReleased(eye.X + 5, eye.Y + 5, PointerButton.Left);
        Assert.False(password.Editor.Password);
        // Pressed, it says what it does next.
        Assert.Equal("Hide", password.Eye.Name);
        screen.PointerPressed(eye.X + 5, eye.Y + 5, PointerButton.Left);
        screen.PointerReleased(eye.X + 5, eye.Y + 5, PointerButton.Left);
        Assert.True(password.Editor.Password);
    }

    [Fact]
    public void Copy_puts_what_is_in_the_input_on_the_clipboard_typed_or_not_yet_taken()
    {
        var host = new TextFactsHost { Values = { ["token"] = "s3cr3t" } };
        var (screen, _, parts) = Made.Window("""<Password at="1,1" span="4" name="token" bind="token" />""", host);
        var password = (TextBoxPart)parts[0];
        password.Editor.Text = "typed";
        Assert.Equal("Copy", password.Copy!.Name);
        password.Copy.OnClick();
        Assert.Equal("typed", screen.Clipboard.GetText());
    }

    [Fact]
    public void A_text_input_has_neither()
    {
        var (_, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="x" />""", new TextFactsHost());
        var box = (TextBoxPart)parts[0];
        Assert.Null(box.Eye);
        Assert.Null(box.Copy);
        Assert.Single(box.Children);
    }

    [Fact]
    public void An_error_marks_the_input_in_the_error_color()
    {
        var host = new TextFactsHost();
        var (screen, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="email" />""", host);
        host.States["email"].ErrorText = Clarity.Runtime.Text.Plain("Not an address");
        using var pixels = Shot.Of(screen, 800, 400);
        Assert.True(TextEditorTests.Inked(pixels, parts[0].Bounds, screen.Look.Color("error")));
    }
}
