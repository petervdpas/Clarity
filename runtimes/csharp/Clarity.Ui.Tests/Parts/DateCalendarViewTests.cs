// The calendar's year and decade views, as Avalonia's Calendar zoomed out
// (DisplayMode): a click on the title shows the year's months and then the
// decade's years; a picked year shows its months and a picked month its days.
// The month buttons turn a month, a year or a decade in each view. Keys:
// Ctrl+Up zooms out, Ctrl+Down or Enter zooms in on the marked month or year,
// arrows move the mark (Up and Down a row) and Escape closes.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class DateCalendarViewTests
{
    const string Due = """<Date at="1,1" span="4" name="due" bind="task.due" onchange="dueChanged" />""";

    static (Screen Screen, DayCalendar Calendar, Func<DateOnly?> Picked) Open(DateOnly day)
    {
        var host = new DatePartHost();
        var (screen, _, _) = Made.Window("", host);
        DateOnly? picked = null;
        var calendar = (DayCalendar)DayCalendar.Open(screen, new Box(10, 10, 100, 24), day, d => picked = d, host.Formats.Culture).Content;
        calendar.Today = new DateOnly(2026, 11, 2);
        return (screen, calendar, () => picked);
    }

    static void Click(Screen screen, Box box)
    {
        screen.PointerPressed(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left);
        screen.PointerReleased(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left);
    }

    static void Press(Screen screen, Key key, Modifiers held = Modifiers.None) => screen.KeyPressed(new KeyInput(key, held));

    [Fact]
    public void A_click_on_the_title_shows_the_year_and_then_the_decade()
    {
        var (screen, calendar, picked) = Open(new DateOnly(2026, 10, 20));
        Assert.Equal(CalendarView.Month, calendar.View);
        Click(screen, calendar.TitleBox);
        Assert.Equal(CalendarView.Year, calendar.View);
        Assert.Equal("2026", calendar.Title);
        Assert.NotNull(calendar.CellOf(new DateOnly(2026, 1, 1)));
        Assert.NotNull(calendar.CellOf(new DateOnly(2026, 12, 1)));
        Assert.Null(calendar.CellOf(new DateOnly(2027, 1, 1)));
        Click(screen, calendar.TitleBox);
        Assert.Equal(CalendarView.Decade, calendar.View);
        Assert.Equal("2020-2029", calendar.Title);
        // The decade shows a year before and after it too.
        Assert.NotNull(calendar.CellOf(new DateOnly(2019, 1, 1)));
        Assert.NotNull(calendar.CellOf(new DateOnly(2030, 1, 1)));
        Assert.Null(calendar.CellOf(new DateOnly(2031, 1, 1)));
        Click(screen, calendar.TitleBox);
        Assert.Equal(CalendarView.Decade, calendar.View);
        Assert.Single(screen.Overlays);
        Assert.Null(picked());
    }

    [Fact]
    public void The_buttons_turn_a_year_in_the_year_view_and_a_decade_in_the_decade_view()
    {
        var (screen, calendar, _) = Open(new DateOnly(2026, 10, 20));
        Click(screen, calendar.TitleBox);
        Click(screen, calendar.Previous);
        Assert.Equal("2025", calendar.Title);
        Click(screen, calendar.Next);
        Click(screen, calendar.Next);
        Assert.Equal("2027", calendar.Title);
        Assert.Equal("Next year", calendar.TooltipAt(calendar.Next.X + 2, calendar.Next.Y + 2));
        Click(screen, calendar.TitleBox);
        Click(screen, calendar.Next);
        Assert.Equal("2030-2039", calendar.Title);
        Click(screen, calendar.Previous);
        Click(screen, calendar.Previous);
        Assert.Equal("2010-2019", calendar.Title);
        Assert.Equal("Previous decade", calendar.TooltipAt(calendar.Previous.X + 2, calendar.Previous.Y + 2));
    }

    [Fact]
    public void Picking_a_year_and_a_month_turns_the_calendar_there()
    {
        var (screen, calendar, picked) = Open(new DateOnly(2026, 10, 31));
        Click(screen, calendar.TitleBox);
        Click(screen, calendar.TitleBox);
        Click(screen, calendar.Next);
        Click(screen, calendar.CellOf(new DateOnly(2033, 1, 1))!.Value);
        Assert.Equal(CalendarView.Year, calendar.View);
        Assert.Equal("2033", calendar.Title);
        Click(screen, calendar.CellOf(new DateOnly(2033, 2, 1))!.Value);
        Assert.Equal(CalendarView.Month, calendar.View);
        Assert.Equal(new DateOnly(2033, 2, 1), calendar.Month);
        // The marked day keeps its number where the month has it, else the month's last day.
        Assert.Equal(new DateOnly(2033, 2, 28), calendar.Day);
        Assert.Null(picked());
        Assert.Single(screen.Overlays);
        Click(screen, calendar.CellOf(new DateOnly(2033, 2, 14))!.Value);
        Assert.Equal(new DateOnly(2033, 2, 14), picked());
    }

    [Fact]
    public void Keys_zoom_out_and_in_and_move_the_marked_month_and_year()
    {
        var host = new DatePartHost();
        host.Values["task.due"] = new DateOnly(2026, 10, 20);
        var (screen, _, parts) = Made.Window(Due, host);
        var part = (DatePart)parts[0];
        screen.Focus(part.Editor);
        Press(screen, Key.Down);
        var calendar = (DayCalendar)Assert.Single(screen.Overlays).Content;
        Press(screen, Key.Up, Modifiers.Control);
        Assert.Equal(CalendarView.Year, calendar.View);
        Press(screen, Key.Right);
        Assert.Equal(new DateOnly(2026, 11, 20), calendar.Day);
        Press(screen, Key.Down);
        Assert.Equal(new DateOnly(2027, 3, 20), calendar.Day);
        Assert.Equal("2027", calendar.Title);
        Press(screen, Key.Up, Modifiers.Control);
        Assert.Equal(CalendarView.Decade, calendar.View);
        Press(screen, Key.Left);
        Assert.Equal(2026, calendar.Day.Year);
        Press(screen, Key.Up);
        Assert.Equal(2022, calendar.Day.Year);
        Press(screen, Key.Enter);
        Assert.Equal(CalendarView.Year, calendar.View);
        Assert.Equal("2022", calendar.Title);
        Press(screen, Key.Down, Modifiers.Control);
        Assert.Equal(CalendarView.Month, calendar.View);
        Assert.Equal(new DateOnly(2022, 3, 1), calendar.Month);
        Assert.Single(screen.Overlays);
        Assert.Empty(host.Log);
        Press(screen, Key.Up, Modifiers.Control);
        Press(screen, Key.Escape);
        Assert.Empty(screen.Overlays);
        Assert.Empty(host.Log);
        Assert.Same(part.Editor, screen.Focused);
    }

    [Fact]
    public void The_year_and_decade_views_mark_the_chosen_one_in_the_accent_and_draw_their_names()
    {
        var (screen, calendar, _) = Open(new DateOnly(2026, 10, 20));
        Click(screen, calendar.TitleBox);
        Assert.Equal("Oct", calendar.NameOf(new DateOnly(2026, 10, 1)));
        var accent = screen.Look.Color("accent");
        using (var shot = Shot.Of(screen, 800, 400))
        {
            Assert.True(TextEditorTests.Inked(shot, calendar.CellOf(new DateOnly(2026, 10, 1))!.Value, accent));
            Assert.False(TextEditorTests.Inked(shot, calendar.CellOf(new DateOnly(2026, 3, 1))!.Value, accent));
        }
        Click(screen, calendar.TitleBox);
        Assert.Equal("2026", calendar.NameOf(new DateOnly(2026, 1, 1)));
        using (var shot = Shot.Of(screen, 800, 400))
        {
            Assert.True(TextEditorTests.Inked(shot, calendar.CellOf(new DateOnly(2026, 1, 1))!.Value, accent));
            Assert.False(TextEditorTests.Inked(shot, calendar.CellOf(new DateOnly(2023, 1, 1))!.Value, accent));
        }
    }

    [Fact]
    public void The_month_names_of_the_year_view_are_the_persons_culture()
    {
        var host = new DatePartHost { Formats = new(CultureInfo.GetCultureInfo("nl-NL"), null) };
        var (screen, _, _) = Made.Window("", host);
        var calendar = (DayCalendar)DayCalendar.Open(screen, new Box(10, 10, 100, 24), new DateOnly(2026, 3, 4), _ => { }, host.Formats.Culture).Content;
        calendar.ZoomOut();
        Assert.Equal("mrt", calendar.NameOf(new DateOnly(2026, 3, 1)));
    }
}
