// The tags part by keyboard and its words, as Avalonia's tags input had
// them: a chip's remove button says what it does, is a Tab stop that Enter
// and Space press, and draws a focus ring; Enter in the typing box takes the
// item and then submits the form; the box shows its word to add; a list of
// days has a calendar button inside the box; an unbound tags part keeps its
// own list.

using System.Collections;
using Clarity.Runtime;
using Clarity.Ui.Tests.Parts;

namespace Clarity.Ui.Tests;

public class TagsKeyboardTests
{
    const string Markup = """<Tags at="1,1" span="8" name="genres" bind="album.genres" onchange="genresChanged" />""";

    static (Screen Screen, TagsPart Tags, TestHost Host) Make(IList items, string markup = Markup)
    {
        var host = new TestHost();
        host.Values["album.genres"] = items;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<TagsPart>(parts[0]), host);
    }

    static void Click(Screen screen, Box box)
    {
        screen.PointerPressed(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left);
        screen.PointerReleased(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left);
    }

    static void Press(Screen screen, Key key) => screen.KeyPressed(new KeyInput(key, Modifiers.None));

    static List<string> Items(TestHost host) => [.. ((IList)host.Values["album.genres"]!).Cast<string>()];

    [Fact]
    public void The_remove_button_says_what_it_does()
    {
        var (_, tags, _) = Make(new List<string> { "jazz" });
        var remove = tags.Chips[0].Remove!;
        Assert.Equal("Remove", remove.TooltipAt(remove.Bounds.X + 2, remove.Bounds.Y + 2));
    }

    [Fact]
    public void The_remove_buttons_are_Tab_stops_before_the_box()
    {
        var (screen, tags, _) = Make(new List<string> { "jazz", "live" });
        Assert.True(tags.Chips[0].Remove!.Focusable);
        screen.Focus(tags.Chips[0].Remove);
        screen.MoveFocus();
        Assert.Same(tags.Chips[1].Remove, screen.Focused);
        screen.MoveFocus();
        Assert.Same(tags.Editor, screen.Focused);
    }

    [Fact]
    public void Enter_and_Space_press_the_remove_button_and_the_focus_goes_on_to_the_next_chip_else_the_box()
    {
        var (screen, tags, host) = Make(new List<string> { "jazz", "live", "folk" });
        screen.Focus(tags.Chips[1].Remove);
        Press(screen, Key.Enter);
        Assert.Equal(["jazz", "folk"], Items(host));
        Assert.Equal("run genres change genresChanged", host.Log[^1]);
        Assert.Same(tags.Chips[1].Remove, screen.Focused);
        Press(screen, Key.Space);
        Assert.Equal(["jazz"], Items(host));
        // The last chip gone, the box is next.
        Assert.Same(tags.Editor, screen.Focused);
        screen.Focus(tags.Chips[0].Remove);
        Press(screen, Key.Enter);
        Assert.Empty(Items(host));
        Assert.Same(tags.Editor, screen.Focused);
    }

    [Fact]
    public void A_focused_remove_button_draws_its_focus_ring()
    {
        var (screen, tags, _) = Make(new List<string> { "jazz" });
        var focus = screen.Look.Color("focus");
        var remove = tags.Chips[0].Remove!;
        using (var shot = Shot.Of(screen, 800, 400))
            Assert.False(TextEditorTests.Inked(shot, remove.Bounds, focus));
        screen.Focus(remove);
        using (var shot = Shot.Of(screen, 800, 400))
            Assert.True(TextEditorTests.Inked(shot, remove.Bounds, focus));
    }

    [Fact]
    public void Enter_in_the_box_takes_the_item_and_then_the_form_submits()
    {
        var host = new TestHost();
        host.Values["album.genres"] = new List<string> { "jazz" };
        var (screen, form, parts) = TextWindow.InForm(Markup, host);
        form.Submitted = () => host.Log.Add("submit");
        var tags = (TagsPart)parts[0];
        screen.Focus(tags.Editor);
        screen.TextTyped("live");
        Press(screen, Key.Enter);
        Assert.Equal(["jazz", "live"], Items(host));
        Assert.Equal("run genres change genresChanged", host.Log[^2]);
        Assert.Equal("submit", host.Log[^1]);
    }

    [Fact]
    public void The_box_shows_the_word_to_add()
    {
        var (_, tags, _) = Make(new List<string> { "jazz" });
        Assert.Equal("Add", tags.Editor!.Placeholder);
        var (_, picked, _) = Make(new List<TagsPartGenre> { TagsPartGenre.Jazz });
        Assert.Equal("Add", picked.Adder.Name);
    }

    [Fact]
    public void A_list_of_days_opens_its_calendar_from_the_button_inside_the_box()
    {
        var (screen, tags, host) = Make(new List<DateOnly> { new(2026, 10, 1) });
        var box = tags.Editor!.Bounds;
        var button = tags.CalendarButton!;
        Assert.True(button.Bounds.X > box.X && button.Bounds.Right <= box.Right);
        Assert.True(tags.Editor.RightInset >= button.Bounds.Width);
        Assert.Equal("Calendar", button.Name);
        // The focus alone opens nothing.
        screen.Focus(tags.Editor);
        Assert.Empty(screen.Overlays);
        Click(screen, button.Bounds);
        var calendar = Assert.IsType<DayCalendar>(Assert.Single(screen.Overlays).Content);
        Click(screen, calendar.CellOf(calendar.Day)!.Value);
        Assert.Empty(screen.Overlays);
        Assert.Equal([new DateOnly(2026, 10, 1), calendar.Day], ((List<DateOnly>)host.Values["album.genres"]!).ToArray());
        // The button closes it again.
        Click(screen, button.Bounds);
        Assert.Single(screen.Overlays);
        Click(screen, button.Bounds);
        Assert.Empty(screen.Overlays);
        // A list of words has no calendar button.
        var (_, words, _) = Make(new List<string> { "jazz" });
        Assert.Null(words.CalendarButton);
    }

    [Fact]
    public void An_unbound_tags_part_keeps_its_own_list()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Tags at="1,1" span="8" name="moods" onchange="moodsChanged" />""", host);
        var tags = (TagsPart)parts[0];
        Assert.NotNull(tags.Editor);
        screen.Focus(tags.Editor);
        screen.TextTyped("calm");
        Press(screen, Key.Enter);
        screen.TextTyped("bright");
        Press(screen, Key.Enter);
        Assert.Equal(["calm", "bright"], tags.Value!.Cast<string>());
        Assert.Equal(["calm", "bright"], tags.Chips.Select(c => c.Label));
        Click(screen, tags.Chips[0].Remove!.Bounds);
        Assert.Equal(["bright"], tags.Value!.Cast<string>());
        Assert.Equal(["run moods change moodsChanged", "run moods change moodsChanged", "run moods change moodsChanged"], host.Log);
    }
}
