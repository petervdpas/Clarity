// A key that runs a command (Save's Ctrl+S) after the person typed into an
// input: the input takes what is typed first, so the command sees the new
// value, as a click on the button does by moving the focus. Every part with
// a text being typed takes part: a text box, a number, a tag's box, a table's
// cell, a day, a color, a typed or a search dropdown, a folder.

using Clarity.Runtime;
using Clarity.Ui.Tests.Parts;

namespace Clarity.Ui.Tests;

public class TypedThenKeyTests
{
    const string Save = """<Button name="save" at="1,12" span="2" key="Ctrl+S" onclick="save" /><Button name="add" at="3,12" span="2" onclick="add" />""";

    static (Screen Screen, WindowView View) Window(string markup, IPartHost host)
    {
        var spec = WindowMarkup.Read($"""<Window name="Keys">{markup}{Save}</Window>""");
        var view = new WindowView(spec, host);
        var screen = new Screen(view, ThemeLook.Clarity(dark: false)) { Keys = view.KeyPressed };
        screen.Resize(900, 600, 1);
        return (screen, view);
    }

    static T PartOf<T>(WindowView view, string name) where T : Part =>
        Assert.IsType<T>(view.Find<Part>(p => p.Spec.Name == name));

    static void CtrlS(Screen screen) => screen.KeyPressed(new KeyInput(Key.S, Modifiers.Control));

    /// <summary>The write of <paramref name="written"/> comes before the run of save.</summary>
    static void WrittenBeforeSave(TestHost host, string written)
    {
        var write = host.Log.FindIndex(l => l.StartsWith("write ") && l.EndsWith(written));
        var save = host.Log.FindIndex(l => l.StartsWith("run save click"));
        Assert.True(write >= 0, string.Join(" | ", host.Log));
        Assert.True(save > write, string.Join(" | ", host.Log));
    }

    [Fact]
    public void New_then_a_typed_name_then_ctrl_s_saves_the_name()
    {
        var host = new TestHost();
        host.Values["person.name"] = "";
        var (screen, view) = Window("""<Button name="new" at="1,1" span="2" onclick="newPerson" /><Textbox name="name" at="1,2" span="4" bind="person.name" />""", host);
        var name = PartOf<TextBoxPart>(view, "name");
        Cases.Click(screen, view.Find<ButtonPart>(b => b.Spec.Name == "new")!.Bounds);
        screen.Focus(name.Editor);
        screen.TextTyped("Ada");
        CtrlS(screen);
        Assert.Equal("Ada", host.Values["person.name"]);
        WrittenBeforeSave(host, "Ada");
        // The input keeps the focus, and pressing again changes nothing more.
        Assert.Same(name.Editor, screen.Focused);
        var writes = host.Log.Count(l => l.StartsWith("write "));
        CtrlS(screen);
        Assert.Equal(writes, host.Log.Count(l => l.StartsWith("write ")));
    }

    [Fact]
    public void A_typed_number_is_taken_before_the_key()
    {
        var host = new TestHost();
        host.Values["order.hours"] = 1L;
        var (screen, view) = Window("""<Number name="hours" at="1,1" span="3" bind="order.hours" />""", host);
        var hours = PartOf<NumberPart>(view, "hours");
        TextWindow.Type(screen, hours.Editor, "8");
        CtrlS(screen);
        Assert.Equal(8L, host.Values["order.hours"]);
        WrittenBeforeSave(host, "8");
    }

    [Fact]
    public void A_typed_tag_is_added_before_the_key()
    {
        var host = new TestHost();
        host.Values["album.genres"] = new List<string> { "live" };
        var (screen, view) = Window("""<Tags name="genres" at="1,1" span="8" bind="album.genres" />""", host);
        var tags = PartOf<TagsPart>(view, "genres");
        var editor = tags.Find<TextEditor>()!;
        screen.Focus(editor);
        screen.TextTyped("demo");
        CtrlS(screen);
        Assert.Equal(["live", "demo"], (List<string>)host.Values["album.genres"]!);
        Assert.True(host.Log.FindIndex(l => l.StartsWith("write ")) < host.Log.FindIndex(l => l.StartsWith("run save click")));
    }

    [Fact]
    public void A_typed_cell_is_taken_before_the_key()
    {
        var host = new DataHost();
        host.Lists["relations"] = [new Relation { Multiplicity = "1" }];
        var (screen, view) = Window("""<Table name="grid" at="1,1" span="12" rows="6" bind="relations" columns="multiplicity, label" edit="multiplicity, label" />""", host);
        var table = PartOf<TablePart>(view, "grid");
        Cases.Click(screen, table.CellBox(0, "label")!.Value);
        screen.TextTyped("owns");
        CtrlS(screen);
        Assert.Equal("owns", ((Relation)host.Lists["relations"][0]).Label);
        Assert.True(host.Log.FindIndex(l => l.StartsWith("write ")) < host.Log.FindIndex(l => l.StartsWith("run save click")), string.Join(" | ", host.Log));
    }

    [Fact]
    public void A_typed_dropdown_s_text_is_taken_before_the_key()
    {
        var host = new TestHost();
        host.Values["person.city"] = "Rome";
        host.Offered["person.city"] = [new("Rome", "Rome"), new("Utrecht", "Utrecht")];
        var (screen, view) = Window("""<Dropdown name="city" at="1,1" span="4" bind="person.city" typed="true" />""", host);
        var city = PartOf<TypedDropdownPart>(view, "city");
        TextWindow.Type(screen, city.Find<TextEditor>()!, "Oslo");
        CtrlS(screen);
        Assert.Equal("Oslo", host.Values["person.city"]);
        WrittenBeforeSave(host, "Oslo");
    }

    [Fact]
    public void A_typed_color_is_taken_before_the_key()
    {
        var host = new TestHost();
        host.Values["tag.color"] = "#123456";
        var (screen, view) = Window("""<Color name="tint" at="1,1" span="4" bind="tag.color" />""", host);
        var tint = PartOf<ColorPart>(view, "tint");
        TextWindow.Type(screen, tint.Find<TextEditor>()!, "#abcdef");
        CtrlS(screen);
        Assert.Equal("#abcdef", host.Values["tag.color"]);
        WrittenBeforeSave(host, "#abcdef");
    }

    [Fact]
    public void A_typed_day_is_taken_before_the_key()
    {
        var host = new TestHost { Formats = new(System.Globalization.CultureInfo.GetCultureInfo("en-US"), null) };
        host.Values["task.due"] = new DateOnly(2026, 1, 2);
        var (screen, view) = Window("""<Date name="due" at="1,1" span="4" bind="task.due" />""", host);
        var due = PartOf<DatePart>(view, "due");
        TextWindow.Type(screen, due.Find<TextEditor>()!, "3/4/2026");
        CtrlS(screen);
        Assert.NotEqual(new DateOnly(2026, 1, 2), host.Values["task.due"]);
        Assert.True(host.Log.FindIndex(l => l.StartsWith("write ")) < host.Log.FindIndex(l => l.StartsWith("run save click")), string.Join(" | ", host.Log));
    }

    [Fact]
    public void Every_part_with_typed_text_takes_it()
    {
        foreach (var type in new[] { typeof(TextBoxPart), typeof(NumberPart), typeof(TagsPart), typeof(TablePart), typeof(DatePart), typeof(ColorPart), typeof(TypedDropdownPart), typeof(SearchDropdownPart), typeof(FolderPart) })
            Assert.True(typeof(ITakesTyped).IsAssignableFrom(type), type.Name);
    }
}
