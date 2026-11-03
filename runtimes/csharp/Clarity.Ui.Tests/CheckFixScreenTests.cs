// What the screen does for every window, each shown as the person meets it:
// the frame comes before the window's grid in Tab order wherever it is
// docked; a part (a library's custom part) that fails to take its place or
// to draw says why in its place while its neighbours go on; changing an
// accordion or choosing in a list that only selects makes no edit; a scroll
// box keeps room for its bar; the long call bar sits at the bottom right; a
// note is 420 wide; a context menu greys its disabled entries; a designer's
// tooltip shows over the inner pieces of a part; and a key Clarity.UI has no
// Key for is written and matched by its own name.

using Clarity.Runtime;
using Clarity.Ui.Browser;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Tests;

/// <summary>A library's part that cannot take its place.</summary>
[PartKind("UnplaceableGauge")]
public sealed class UnplaceableGauge : CustomPart
{
    protected override void Set(string name, object? value) { }

    protected override void Hear(string hear, object?[] values) { }

    public override void Arrange(Box box) => throw new InvalidOperationException("no room today");
}

/// <summary>A library's part whose drawing fails.</summary>
[PartKind("UndrawableGauge")]
public sealed class UndrawableGauge : CustomPart
{
    protected override void Set(string name, object? value) { }

    protected override void Hear(string hear, object?[] values) { }

    public override void Draw(Painter painter) => throw new InvalidOperationException("no paint today");
}

/// <summary>A window's view model as the generator writes one: an accordion's and a list's handlers that change the note.</summary>
public sealed class FoldViewModel() : WindowViewModel(null)
{
    public sealed class Person
    {
        public string Name { get; set; } = "";
    }

    public override string Title => "Fold";

    public string Note { get => note; set => SetProperty(ref note, value); }
    string note = "";

    public string Name { get => name; set => SetProperty(ref name, value); }
    string name = "";

    public List<Person> People { get; } = [new() { Name = "Ann" }, new() { Name = "Bob" }];

    public PartState Fold { get; } = new("fold");
    public PartState PeopleList { get; } = new("peopleList");
    public PartState NameBox { get; } = new("nameBox");

    RelayCommand? folded;
    public IRelayCommand FoldedCommand => folded ??= new RelayCommand(Folded);
    void Folded() => Handle(OnFolded);
    void OnFolded() => Note = $"folded {Fold.OpenSection}";

    RelayCommand? picked;
    public IRelayCommand PickedCommand => picked ??= new RelayCommand(Picked);
    void Picked() => Handle(OnPicked);
    void OnPicked() => Note = "picked";
}

public class CheckFixScreenTests
{
    static void Click(Screen screen, Box box, PointerButton button = PointerButton.Left)
    {
        var (x, y) = (box.X + box.Width / 2, box.Y + box.Height / 2);
        screen.PointerPressed(x, y, button);
        screen.PointerReleased(x, y, button);
    }

    static string NameOf(Element e) => e switch
    {
        FrameButton b => b.Item.Text("name") ?? "",
        Part p => p.Spec.Name ?? "",
        _ => e.GetType().Name,
    };

    [Fact]
    public void The_frame_comes_before_the_windows_grid_in_tab_order_wherever_it_is_docked()
    {
        // The sidebar is on the right, the grid's button at the top left: the frame still comes first.
        var (screen, _, _) = Framed.Window("""
            <Toolbar name="tools"><Item name="save" caption="Save" onclick="save" /></Toolbar>
            <Sidebar side="right" name="side" width="220" columns="4">
              <Button name="find" at="1,1" span="4" caption="Find" onclick="find" />
            </Sidebar>
            <Button name="inGrid" at="1,1" span="3" caption="In the grid" onclick="here" />
            """);
        Framed.Draw(screen);
        Assert.Equal(["save", "find", "inGrid"], screen.TabOrder().Select(NameOf));
        Framed.Press(screen, Key.Tab);
        Assert.Equal("save", NameOf(screen.Focused!));
    }

    static bool HasError(SKBitmap shot, Box box, SKColor error)
    {
        var y = (int)(box.Y + box.Height / 2);
        // The error frame's left edge, from a pixel before the box where the box leaves room for one.
        var from = Math.Max(0, (int)box.X - 1);
        return Enumerable.Range(from, 4).Any(x => shot.PixelAt(x, y) == error);
    }

    [Fact]
    public void A_part_that_fails_to_take_its_place_or_to_draw_says_why_and_the_rest_of_the_window_goes_on()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""
            <Custom name="gauge" at="1,1" span="6" rows="2" part="UnplaceableGauge" />
            <Label name="beside" at="7,1" span="6" caption="Beside" />
            <Custom name="dial" at="1,3" span="6" rows="2" part="UndrawableGauge" />
            <Button name="go" at="7,3" span="3" caption="Go" onclick="go" />
            """, host);
        using var shot = Shot.Of(screen, 800, 400);
        var error = screen.Look.Color("error");
        var gauge = (CustomPlace)parts[0];
        Assert.Equal("no room today", gauge.Failed);
        Assert.True(gauge.Bounds.Width > 0);
        Assert.True(HasError(shot, gauge.Bounds, error), "the part that could not take its place says so in it");
        var dial = (CustomPlace)parts[2];
        Assert.Equal("no paint today", dial.Inner.Failed);
        Assert.True(HasError(shot, dial.Bounds, error), "the part whose drawing failed says so in it");
        // The parts beside them took their places and work.
        Assert.True(parts[1].Bounds.X >= gauge.Bounds.Right && parts[1].Bounds.Width > 0);
        Click(screen, parts[3].Bounds);
        Assert.Equal(["run go click go"], host.Log);
    }

    [Fact]
    public void Changing_an_accordion_or_choosing_in_a_list_that_only_selects_is_no_edit()
    {
        // As the generator writes the markup: only the text box has an undo scope.
        var spec = WindowMarkup.Read("""
            <Window name="Fold">
              <Accordion name="fold" at="1,1" span="6" rows="4" onchange="folded">
                <Section title="One" /><Section title="Two" />
              </Accordion>
              <List name="peopleList" at="7,1" span="6" rows="4" bind="people" show="name" onselect="picked" />
              <Textbox name="nameBox" at="1,6" span="6" bind="name" undoScope="nameBox" />
            </Window>
            """);
        var model = new FoldViewModel();
        var host = new ViewModelHost(model, spec);
        var view = new WindowView(spec, host, model);
        var screen = new Screen(view, ThemeLook.Clarity(dark: false)) { Keys = view.KeyPressed };
        Shot.Of(screen, 800, 500);
        var accordion = view.Find<AccordionPart>(_ => true)!;
        Click(screen, accordion.Headers[1].Bounds);
        Assert.Equal("folded 2", model.Note);
        Assert.Null(model.Edits.LastOf("fold"));
        Assert.False(model.Edits.CanUndo("fold"));
        var list = view.Find<ListPart>(_ => true)!;
        Click(screen, list.RowBox(1)!.Value);
        Assert.Equal("picked", model.Note);
        Assert.Null(model.Edits.LastOf("peopleList"));
        Assert.False(model.Edits.CanUndo("peopleList"));
        // A part with an undo scope does make an edit, which Ctrl+Z takes back.
        host.Write("nameBox", "name", "Ada");
        Assert.NotNull(model.Edits.LastOf("nameBox"));
        model.UndoEdit("nameBox", back: true);
        Assert.Equal("", model.Name);
        Assert.Equal("picked", model.Note);
    }

    [Fact]
    public void A_scroll_box_keeps_room_for_its_bar_beside_its_parts()
    {
        var host = new TestHost();
        var many = string.Concat(Enumerable.Range(1, 10).Select(i => $"""<Button at="1,{i}" span="6" caption="Row {i}" onclick="row" />"""));
        var (screen, _, parts) = Made.Window($"""
            <Scroll at="1,1" span="6" rows="3">{many}</Scroll>
            <Scroll at="7,1" span="6" rows="3"><Button at="1,1" span="6" caption="One" onclick="one" /></Scroll>
            """, host);
        Shot.Of(screen, 800, 400);
        var full = (ScrollPart)parts[0];
        var fits = (ScrollPart)parts[1];
        var fullButton = full.Inner.Children[0].Bounds;
        var fitsButton = fits.Inner.Children[0].Bounds;
        Assert.True(fullButton.Right <= full.View.Bounds.Right - ScrollView.BarThickness, $"{fullButton.Right} leaves the bar its room in {full.View.Bounds.Right}");
        // Where nothing scrolls the parts have the whole width.
        Assert.True(fitsButton.Width - fullButton.Width >= ScrollView.BarThickness);
    }

    [Fact]
    public void The_long_call_bar_shows_at_the_bottom_right_of_the_window()
    {
        var bar = new LongCallBar(() => (30, 120), () => { });
        var screen = new Screen(bar, ThemeLook.Clarity(dark: false));
        Shot.Of(screen, 1000, 700);
        var note = bar.Note;
        var (right, below) = (1000 - note.Right, 700 - note.Bottom);
        Assert.InRange(right, 0, 40);
        Assert.InRange(below, 0, 40);
        Assert.Equal(right, below);
        Assert.True(note.X > 500 && note.Y > 500);
        // Stop sits inside it, on its right.
        Assert.True(bar.Stop.Bounds.Right <= note.Right && bar.Stop.Bounds.X > note.X + note.Width / 2);
    }

    sealed class NotingViewModel() : WindowViewModel(null)
    {
        public override string Title => "Noting";

        public void Note() => Handle(() => Say(Level.Info, Text.Plain("Saved")));
    }

    [Fact]
    public void A_note_is_420_wide_at_the_top_right()
    {
        var model = new NotingViewModel();
        var stack = new WindowStack(_ => "<Window name=\"Noting\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Resize(1000, 400, 1);
        stack.Push(model);
        model.Note();
        using var shot = Shot.Of(screen, 1000, 400);
        var pad = WindowView.ContentPadding;
        var left = (int)(1000 - pad - 420);
        var y = (int)pad + 20;
        // Its accent edge starts 420 from its right, and nothing of it is left of that.
        Assert.Equal(screen.Look.Color("accent"), shot.PixelAt(left + 1, y));
        Assert.NotEqual(screen.Look.Color("accent"), shot.PixelAt(left - 2, y));
        Assert.NotEqual(screen.Look.Color("raised"), shot.PixelAt(left - 2, y));
        Assert.Equal(screen.Look.Color("raised"), shot.PixelAt((int)(1000 - pad - 2), y + 10));
    }

    static int Sum(SKColor c) => c.Red + c.Green + c.Blue;

    [Fact]
    public void A_context_menu_greys_its_disabled_entries()
    {
        var host = new TestHost();
        host.Values["note"] = "Hello";
        var (screen, _, parts) = Made.Window("""<Textbox name="note" at="1,1" span="6" bind="note" />""", host);
        Shot.Of(screen, 800, 400);
        // Nothing is selected: Cut and Copy are disabled, Paste and Select all are not.
        Click(screen, parts[0].Bounds, PointerButton.Right);
        var list = Assert.Single(screen.ContextMenus.Lists);
        using var shot = Shot.Of(screen, 800, 400);
        int Darkest(int at)
        {
            var row = list.RowOf(at);
            var darkest = int.MaxValue;
            for (var x = (int)row.X + 26; x < (int)row.X + 90; x++)
                for (var y = (int)row.Y + 2; y < (int)row.Bottom - 2; y++)
                    darkest = Math.Min(darkest, Sum(shot.PixelAt(x, y)));
            return darkest;
        }
        var muted = Sum(screen.Look.Color("muted"));
        // An enabled entry's caption is in the text color; a disabled one is dimmed, lighter than muted text.
        Assert.True(Darkest(2) < muted, $"Paste {Darkest(2)} is darker than muted {muted}");
        Assert.True(Darkest(0) > muted + 60, $"Cut {Darkest(0)} is greyed beyond muted {muted}");
        Assert.True(Darkest(1) > muted + 60, $"Copy {Darkest(1)} is greyed beyond muted {muted}");
    }

    [Fact]
    public void A_designers_tooltip_shows_over_the_inner_pieces_of_a_part()
    {
        var host = new TestHost();
        host.Values["post.title"] = "Hi";
        host.Values["post.count"] = 3L;
        host.Fields["post.count"] = new FieldFacts("Count", new FieldFormat(FormatKind.Integer));
        host.Values["post.tags"] = new List<string> { "jazz" };
        host.Values["post.color"] = "#336699";
        var (screen, _, parts) = Made.Window("""
            <Textbox name="title" at="1,1" span="6" bind="post.title" tooltip="Title tip" />
            <Spinbox name="count" at="1,2" span="6" bind="post.count" min="0" max="9" tooltip="Count tip" />
            <Tags name="tags" at="1,3" span="6" bind="post.tags" tooltip="Tags tip" />
            <Color name="color" at="1,4" span="6" bind="post.color" tooltip="Color tip" />
            """, host);
        Shot.Of(screen, 800, 400);
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        string? TipAt(Box box)
        {
            screen.PointerMoved(box.X + box.Width / 2, box.Y + box.Height / 2);
            now += Screen.TipPause;
            screen.WakeDue(now);
            return screen.Tip?.Text;
        }
        Assert.Equal("Title tip", TipAt(parts[0].Find<TextEditor>()!.Bounds));
        var count = (NumberPart)parts[1];
        Assert.Equal("Count tip", TipAt(count.Up!.Bounds));
        Assert.Equal("Count tip", TipAt(count.Find<TextEditor>()!.Bounds));
        var tags = (TagsPart)parts[2];
        Assert.Equal("Tags tip", TipAt(tags.Editor!.Bounds));
        // A piece that says what it does keeps its own words.
        var remove = tags.Chips[0].Remove!;
        Assert.Equal(remove.Name, TipAt(remove.Bounds));
        Assert.Equal("Color tip", TipAt(((ColorPart)parts[3]).Swatch.Bounds));
    }

}
