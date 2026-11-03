// The date part, a day or a moment (conformance/forms.json, tables.json):
// the day shows in the person's format (en-US here) and typed text is taken
// when the input is left or Enter is pressed; a day is also picked from a
// calendar under it. A moment has its time typed beside the day as hh:mm on
// a 24-hour clock. Only the person's pick or typed change is a change; the
// same day again and what code sets are not; text that is no day is marked
// and writes nothing.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

/// <summary>A host in en-US whose fields can be told to hold moments.</summary>
class DatePartHost : TestHost
{
    public DatePartHost() => Formats = new(CultureInfo.GetCultureInfo("en-US"), null);
}

public class DatePartTests
{
    const string Due = """<Date at="1,1" span="4" name="due" bind="task.due" onchange="dueChanged" />""";

    static (Screen Screen, DatePart Part, DatePartHost Host) Day(object? value, string markup = Due)
    {
        var host = new DatePartHost();
        host.Values["task.due"] = value;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<DatePart>(parts[0]), host);
    }

    static (Screen Screen, DatePart Part, DatePartHost Host) Moment(DateTime? value)
    {
        var host = new DatePartHost();
        host.Values["task.due"] = value;
        host.Fields["task.due"] = new FieldFacts("Due", new FieldFormat(FormatKind.DateTime));
        var (screen, _, parts) = Made.Window(Due, host);
        return (screen, Assert.IsType<DatePart>(parts[0]), host);
    }

    static void Click(Screen screen, double x, double y)
    {
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    static void Press(Screen screen, Key key) => screen.KeyPressed(new KeyInput(key, Modifiers.None));

    static void Type(Screen screen, TextEditor editor, string text)
    {
        screen.Focus(editor);
        editor.SelectAll();
        screen.KeyPressed(new KeyInput(Key.Delete, Modifiers.None));
        screen.TextTyped(text);
    }

    static void ClickButton(Screen screen, DatePart part) =>
        Click(screen, part.Button.Bounds.X + part.Button.Bounds.Width / 2, part.Button.Bounds.Y + part.Button.Bounds.Height / 2);

    static void ClickDay(Screen screen, DayCalendar calendar, DateOnly day)
    {
        var cell = calendar.CellOf(day)!.Value;
        Click(screen, cell.X + cell.Width / 2, cell.Y + cell.Height / 2);
    }

    [Fact]
    public void It_shows_its_day_in_the_persons_format_and_what_code_sets_is_no_change()
    {
        var (_, part, host) = Day(new DateOnly(2026, 10, 20));
        Assert.False(part.IsMoment);
        Assert.Null(part.Time);
        Assert.Equal("10/20/2026", part.Editor.Text);
        host.Values["task.due"] = new DateOnly(2026, 11, 1);
        host.Change();
        Assert.Equal("11/01/2026", part.Editor.Text);
        host.Values["task.due"] = "2026-12-24";
        host.Change();
        Assert.Equal("12/24/2026", part.Editor.Text);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_typed_day_is_taken_once_when_the_input_is_left()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        Type(screen, part.Editor, "10/21/2026");
        Assert.Empty(host.Log);
        screen.Focus(null);
        Assert.Equal(new DateOnly(2026, 10, 21), host.Values["task.due"]);
        Assert.Equal(2, host.Log.Count);
        Assert.Equal("run due change dueChanged", host.Log[1]);
        Assert.Equal("10/21/2026", part.Editor.Text);
        screen.Focus(part.Editor);
        screen.Focus(null);
        Assert.Equal(2, host.Log.Count);
    }

    [Fact]
    public void Enter_takes_a_typed_day_and_an_iso_day_reads_too()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        Type(screen, part.Editor, "2026-10-25");
        Press(screen, Key.Enter);
        Assert.Equal(new DateOnly(2026, 10, 25), host.Values["task.due"]);
        Assert.Equal("10/25/2026", part.Editor.Text);
    }

    [Fact]
    public void The_same_day_typed_again_is_no_change()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        Type(screen, part.Editor, "10/20/2026");
        screen.Focus(null);
        Type(screen, part.Editor, " 10/20/2026 ");
        screen.Focus(null);
        Assert.Empty(host.Log);
        Assert.Equal("10/20/2026", part.Editor.Text);
    }

    [Fact]
    public void Text_that_is_no_day_is_marked_and_writes_nothing()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        Type(screen, part.Editor, "someday");
        screen.Focus(null);
        Assert.Empty(host.Log);
        Assert.Equal("someday", part.Bad);
        Assert.Equal("someday", part.Editor.Text);
        Type(screen, part.Editor, "13/45/2026");
        screen.Focus(null);
        Assert.Empty(host.Log);
        host.Values["task.due"] = new DateOnly(2026, 10, 22);
        host.Change();
        Assert.Null(part.Bad);
        Assert.Equal("10/22/2026", part.Editor.Text);
    }

    [Fact]
    public void Clearing_the_text_clears_the_day()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        Type(screen, part.Editor, "");
        screen.Focus(null);
        Assert.Null(host.Values["task.due"]);
        Assert.Equal("run due change dueChanged", host.Log[^1]);
        Assert.Equal(2, host.Log.Count);
    }

    [Fact]
    public void A_required_day_cleared_is_no_day_and_the_form_says_it_is_missing_on_save()
    {
        // As Avalonia's DateInput: no input refuses an empty value, required or not.
        var host = new DatePartHost();
        host.Values["task.due"] = new DateOnly(2026, 10, 20);
        host.Fields["task.due"] = new FieldFacts("Due", Required: true);
        var (screen, _, parts) = Made.Window(Due, host);
        var part = Assert.IsType<DatePart>(parts[0]);
        Type(screen, part.Editor, "");
        screen.Focus(null);
        Assert.Null(part.Bad);
        Assert.Null(host.Values["task.due"]);
        Assert.Equal(["write due task.due ", "run due change dueChanged"], host.Log);
    }

    [Fact]
    public void The_calendar_opens_below_on_its_value_month_and_a_click_on_a_day_picks_it()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        ClickButton(screen, part);
        var open = Assert.Single(screen.Overlays);
        Assert.True(open.Place.Y >= part.Bounds.Bottom);
        var calendar = Assert.IsType<DayCalendar>(open.Content);
        Assert.Equal(new DateOnly(2026, 10, 1), calendar.Month);
        Assert.Equal(new DateOnly(2026, 10, 20), calendar.Chosen);
        Assert.Equal(new DateOnly(2026, 10, 20), calendar.Day);
        ClickDay(screen, calendar, new DateOnly(2026, 10, 21));
        Assert.Empty(screen.Overlays);
        Assert.Equal(new DateOnly(2026, 10, 21), host.Values["task.due"]);
        Assert.Equal("run due change dueChanged", host.Log[^1]);
        Assert.Equal(2, host.Log.Count);
        Assert.Equal("10/21/2026", part.Editor.Text);
    }

    [Fact]
    public void Picking_the_day_it_has_or_a_click_outside_is_no_change_and_the_button_closes_it_again()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        ClickButton(screen, part);
        ClickDay(screen, (DayCalendar)screen.Overlays[0].Content, new DateOnly(2026, 10, 20));
        Assert.Empty(screen.Overlays);
        ClickButton(screen, part);
        Assert.Single(screen.Overlays);
        ClickButton(screen, part);
        Assert.Empty(screen.Overlays);
        ClickButton(screen, part);
        Click(screen, 700, 380);
        Assert.Empty(screen.Overlays);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void The_months_buttons_turn_the_month()
    {
        var (screen, part, _) = Day(new DateOnly(2026, 1, 20));
        ClickButton(screen, part);
        var calendar = (DayCalendar)screen.Overlays[0].Content;
        var back = calendar.Previous;
        Click(screen, back.X + back.Width / 2, back.Y + back.Height / 2);
        Assert.Equal(new DateOnly(2025, 12, 1), calendar.Month);
        var on = calendar.Next;
        Click(screen, on.X + on.Width / 2, on.Y + on.Height / 2);
        Click(screen, on.X + on.Width / 2, on.Y + on.Height / 2);
        Assert.Equal(new DateOnly(2026, 2, 1), calendar.Month);
        Assert.Single(screen.Overlays);
        Assert.Equal("February 2026", calendar.Title);
    }

    [Fact]
    public void Keys_move_the_day_turn_the_month_pick_and_close()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20));
        screen.Focus(part.Editor);
        Press(screen, Key.Down);
        var calendar = Assert.IsType<DayCalendar>(Assert.Single(screen.Overlays).Content);
        Press(screen, Key.Right);
        Assert.Equal(new DateOnly(2026, 10, 21), calendar.Day);
        Press(screen, Key.Down);
        Assert.Equal(new DateOnly(2026, 10, 28), calendar.Day);
        Press(screen, Key.Up);
        Press(screen, Key.Left);
        Assert.Equal(new DateOnly(2026, 10, 20), calendar.Day);
        Press(screen, Key.PageDown);
        Assert.Equal(new DateOnly(2026, 11, 20), calendar.Day);
        Assert.Equal(new DateOnly(2026, 11, 1), calendar.Month);
        Press(screen, Key.PageUp);
        Press(screen, Key.PageUp);
        Assert.Equal(new DateOnly(2026, 9, 20), calendar.Day);
        Press(screen, Key.Escape);
        Assert.Empty(screen.Overlays);
        Assert.Empty(host.Log);
        Press(screen, Key.Down);
        Press(screen, Key.Right);
        Press(screen, Key.Enter);
        Assert.Empty(screen.Overlays);
        Assert.Equal(new DateOnly(2026, 10, 21), host.Values["task.due"]);
        Assert.Same(part.Editor, screen.Focused);
    }

    [Fact]
    public void A_moment_shows_its_time_on_a_24_hour_clock_and_a_picked_day_keeps_it()
    {
        var (screen, part, host) = Moment(new DateTime(2026, 10, 20, 14, 5, 0));
        Assert.True(part.IsMoment);
        Assert.Equal("10/20/2026", part.Editor.Text);
        Assert.Equal("14:05", part.Time!.Text);
        Assert.True(part.Time.Bounds.X >= part.Editor.Bounds.Right);
        ClickButton(screen, part);
        ClickDay(screen, (DayCalendar)screen.Overlays[0].Content, new DateOnly(2026, 10, 22));
        Assert.Equal(new DateTime(2026, 10, 22, 14, 5, 0), host.Values["task.due"]);
        Assert.Equal(2, host.Log.Count);
    }

    [Fact]
    public void A_typed_time_changes_the_moment_and_a_time_that_is_no_time_is_marked()
    {
        var (screen, part, host) = Moment(new DateTime(2026, 10, 20, 14, 5, 0));
        Type(screen, part.Time!, "9:30");
        screen.Focus(null);
        Assert.Equal(new DateTime(2026, 10, 20, 9, 30, 0), host.Values["task.due"]);
        Assert.Equal("09:30", part.Time!.Text);
        Assert.Equal(2, host.Log.Count);
        Type(screen, part.Time, "25:99");
        Press(screen, Key.Enter);
        Assert.Equal("25:99", part.BadTime);
        Assert.Equal(2, host.Log.Count);
        Type(screen, part.Time, "09:30");
        screen.Focus(null);
        Assert.Null(part.BadTime);
        Assert.Equal(2, host.Log.Count);
    }

    /// <summary>A time typed on a 12-hour clock (9:30 PM) is read as the person means it; it then shows as hh:mm, as on every renderer.</summary>
    [Fact]
    public void A_time_typed_with_am_or_pm_is_read()
    {
        var (screen, part, host) = Moment(new DateTime(2026, 10, 20, 14, 5, 0));
        Type(screen, part.Time!, "9:30 PM");
        screen.Focus(null);
        Assert.Equal(new DateTime(2026, 10, 20, 21, 30, 0), host.Values["task.due"]);
        Assert.Equal("21:30", part.Time!.Text);
        Assert.Equal(TimeSpan.FromMinutes(30), DatePart.ReadTime("12:30 am", new Formats(System.Globalization.CultureInfo.GetCultureInfo("en-US"), null)));
        Assert.Null(DatePart.ReadTime("13:30 PM", new Formats(System.Globalization.CultureInfo.GetCultureInfo("en-US"), null)));
    }

    [Fact]
    public void A_typed_day_keeps_a_moments_time_and_an_empty_moment_starts_at_midnight()
    {
        var (screen, part, host) = Moment(new DateTime(2026, 10, 20, 14, 5, 0));
        Type(screen, part.Editor, "10/23/2026");
        screen.Focus(null);
        Assert.Equal(new DateTime(2026, 10, 23, 14, 5, 0), host.Values["task.due"]);

        var (screen2, part2, host2) = Moment(null);
        Assert.Equal("", part2.Time!.Text);
        Type(screen2, part2.Time, "8:15");
        screen2.Focus(null);
        Assert.Empty(host2.Log);
        ClickButton(screen2, part2);
        var calendar = (DayCalendar)screen2.Overlays[0].Content;
        Assert.Equal(calendar.Today, calendar.Day);
        var day = calendar.Month.AddDays(21);
        ClickDay(screen2, calendar, day);
        Assert.Equal(day.ToDateTime(TimeOnly.MinValue), host2.Values["task.due"]);
    }

    [Fact]
    public void A_readonly_or_disabled_date_opens_nothing_and_takes_nothing()
    {
        var (screen, part, host) = Day(new DateOnly(2026, 10, 20), """<Date at="1,1" span="4" name="due" bind="task.due" readonly="true" />""");
        ClickButton(screen, part);
        Assert.Empty(screen.Overlays);
        screen.Focus(part.Editor);
        Press(screen, Key.Down);
        Assert.Empty(screen.Overlays);
        screen.TextTyped("9");
        screen.Focus(null);
        Assert.Empty(host.Log);

        var (screen2, part2, host2) = Day(new DateOnly(2026, 10, 20));
        host2.States["due"] = new PartState("due") { IsEnabled = false };
        ClickButton(screen2, part2);
        Assert.Empty(screen2.Overlays);
    }

    [Fact]
    public void An_unbound_named_date_keeps_its_day_in_its_state()
    {
        var host = new DatePartHost();
        var (screen, _, parts) = Made.Window("""<Date at="1,1" span="4" name="when" onchange="whenChanged" />""", host);
        var part = (DatePart)parts[0];
        Type(screen, part.Editor, "10/21/2026");
        screen.Focus(null);
        Assert.Equal("2026-10-21", host.States["when"].Text);
        Assert.Equal(["when"], host.Edits);
        Assert.Equal(["run when change whenChanged"], host.Log);
        Assert.Equal(new DateOnly(2026, 10, 21), part.Day);
    }

    [Fact]
    public void The_calendar_alone_takes_the_focus_marks_today_and_hands_the_picked_day()
    {
        var host = new DatePartHost();
        var (screen, _, _) = Made.Window("", host);
        DateOnly? picked = null;
        var open = DayCalendar.Open(screen, new Box(10, 10, 100, 24), null, d => picked = d, host.Formats.Culture);
        var calendar = (DayCalendar)open.Content;
        // The keys start from the day it opened on, the system's today: the test marks that day, whatever day it runs.
        calendar.Today = DateOnly.FromDateTime(DateTime.Now);
        Assert.Same(calendar, screen.Focused);
        Assert.Null(calendar.Chosen);
        Assert.True(calendar.CellOf(calendar.Today) is not null);
        Press(screen, Key.Right);
        Press(screen, Key.Enter);
        Assert.Equal(calendar.Today.AddDays(1), picked);
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void The_calendar_names_its_weekdays_and_months_in_the_persons_language()
    {
        var host = new DatePartHost { Formats = new(CultureInfo.GetCultureInfo("nl-NL"), null) };
        host.Values["task.due"] = new DateOnly(2026, 10, 20);
        var (screen, _, parts) = Made.Window(Due, host);
        var part = (DatePart)parts[0];
        Assert.Equal("20-10-2026", part.Editor.Text);
        ClickButton(screen, part);
        var calendar = (DayCalendar)screen.Overlays[0].Content;
        Assert.Equal("oktober 2026", calendar.Title);
        Assert.Equal("ma", calendar.Weekdays[0]);
    }

    [Fact]
    public void The_month_arrows_point_the_way_they_turn()
    {
        var host = new DatePartHost();
        var (screen, _, _) = Made.Window("", host);
        var calendar = (DayCalendar)DayCalendar.Open(screen, new Box(10, 10, 100, 24), null, _ => { }, host.Formats.Culture).Content;
        using var shot = Shot.Of(screen, 800, 400);
        // The point of an arrow is its ink nearest the side it turns to, and lies on its middle row.
        Assert.True(PointOf(shot, calendar.Previous, left: true), "the previous month's arrow points left");
        Assert.True(PointOf(shot, calendar.Next, left: false), "the next month's arrow points right");
    }

    /// <summary>Whether the arrow's ink farthest to the left (or right) in <paramref name="box"/> lies near the box's middle row.</summary>
    static bool PointOf(SKBitmap shot, Box box, bool left)
    {
        var ground = shot.PixelAt((int)box.X + 1, (int)box.Y + 1);
        var middle = box.Y + box.Height / 2;
        var columns = Enumerable.Range((int)box.X, (int)box.Width);
        foreach (var x in left ? columns : columns.Reverse())
        {
            var ink = Enumerable.Range((int)box.Y, (int)box.Height).Where(y => Far(shot.PixelAt(x, y), ground)).ToList();
            if (ink.Count > 0)
                return ink.All(y => Math.Abs(y + 0.5 - middle) < 2.5);
        }
        return false;
    }

    static bool Far(SKColor a, SKColor b) => Math.Abs(a.Red - b.Red) + Math.Abs(a.Green - b.Green) + Math.Abs(a.Blue - b.Blue) > 90;

    [Fact]
    public void A_time_without_a_day_waits_for_one()
    {
        var (screen, part, host) = Moment(null);
        Type(screen, part.Time!, "8:15");
        Press(screen, Key.Enter);
        screen.Focus(null);
        Assert.Null(host.Values["task.due"]);
        Assert.Null(part.BadTime);
        Assert.Empty(host.Log);
        Assert.Equal("8:15", part.Time!.Text);
    }

    [Fact]
    public void Enter_in_the_time_takes_it_and_then_the_form_submits()
    {
        var host = new DatePartHost();
        host.Values["task.due"] = new DateTime(2026, 10, 20, 14, 5, 0);
        host.Fields["task.due"] = new FieldFacts("Due", new FieldFormat(FormatKind.DateTime));
        var (screen, form, parts) = TextWindow.InForm(Due, host);
        var part = (DatePart)parts[0];
        form.Submitted = () => host.Log.Add("submit");
        Type(screen, part.Time!, "9:30");
        Press(screen, Key.Enter);
        Assert.Equal(new DateTime(2026, 10, 20, 9, 30, 0), host.Values["task.due"]);
        Assert.Equal("submit", host.Log[^1]);
        Assert.Equal("run due change dueChanged", host.Log[^2]);
    }

    [Fact]
    public void Clearing_the_day_empties_the_moment()
    {
        var (screen, part, host) = Moment(new DateTime(2026, 10, 20, 14, 5, 0));
        Type(screen, part.Editor, "");
        screen.Focus(null);
        Assert.Null(host.Values["task.due"]);
        Assert.Null(part.Value);
        Assert.Equal("", part.Editor.Text);
        Assert.Equal("", part.Time!.Text);
        Assert.Equal("run due change dueChanged", host.Log[^1]);
    }

    [Fact]
    public void It_draws_closed_and_open_without_failing()
    {
        var (screen, part, _) = Moment(new DateTime(2026, 10, 20, 14, 5, 0));
        using (Shot.Of(screen, 800, 400)) { }
        ClickButton(screen, part);
        using var shot = Shot.Of(screen, 800, 400);
        Assert.Single(screen.Overlays);
    }
}
