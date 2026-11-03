// A calendar over the window: one month as a grid of days under its name,
// with buttons for the month before and after. Its weekday and month names
// are the person's culture's. Today is marked, and so is the chosen day; a
// click on a day picks it. Arrows move the marked day, Page Up and Page Down
// a month, Enter picks it and Escape closes the calendar. A date part opens
// it under its input; a table opens it under a date cell.
//
// As Avalonia's Calendar, it zooms out: a click on the title (or Ctrl+Up)
// shows the year's twelve months, and again the decade's years with one
// before and one after. A click on a year shows its months, a click on a
// month its days; Enter or Ctrl+Down does that for the marked one. In those
// views the buttons and Page Up and Page Down turn a year or a decade, the
// arrows move the mark by one, Up and Down by a row.

using System.Globalization;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>What a calendar shows: a month's days, a year's months or a decade's years.</summary>
public enum CalendarView
{
    Month,
    Year,
    Decade,
}

public sealed class DayCalendar : Element
{
    const double Pad = 8;
    const double Head = 32;
    const double WeekRow = 22;
    const double CellWidth = 32;
    const double CellHeight = 28;

    /// <summary>The year and decade views hold three rows of four.</summary>
    const int Across = 4;

    /// <summary>How wide and high a calendar is.</summary>
    public const double Width = 7 * CellWidth + 2 * Pad;

    public const double Height = Pad + Head + WeekRow + 6 * CellHeight + Pad;

    readonly CultureInfo culture;
    Overlay? overlay;
    object? pressedOn;
    DateOnly? hovered;
    bool titleHovered;

    public DayCalendar(DateOnly? chosen, CultureInfo? culture = null)
    {
        this.culture = culture ?? Locale.Formats.Culture;
        Chosen = chosen;
        Day = chosen ?? Today;
        Month = First(Day);
    }

    /// <summary>The day it was opened with; none for an empty value.</summary>
    public DateOnly? Chosen { get; }

    /// <summary>The day the keys move, which Enter picks.</summary>
    public DateOnly Day { get; private set; }

    /// <summary>The first day of the month it shows.</summary>
    public DateOnly Month { get; private set; }

    /// <summary>What it shows now: the days of a month, the months of a year or the years of a decade.</summary>
    public CalendarView View { get; private set; } = CalendarView.Month;

    /// <summary>The first year of the decade it shows (2020 for 2026).</summary>
    int DecadeStart => Day.Year - Day.Year % 10;

    /// <summary>Today, which it marks.</summary>
    public DateOnly Today { get; set; } = Clock.Today();

    /// <summary>A day was picked.</summary>
    public Action<DateOnly>? Picked { get; set; }

    /// <summary>The element that keeps the focus while it is open (a date part's input), which hands it the keys; none: it takes the focus itself.</summary>
    public Element? Holder { get; init; }

    public override bool Focusable => Holder is null;

    /// <summary>The month's name and year in the person's culture; the year; or the decade as 2020-2029.</summary>
    public string Title => View switch
    {
        CalendarView.Year => Day.Year.ToString(culture),
        CalendarView.Decade => DecadeStart.ToString(culture) + "-" + (DecadeStart + 9).ToString(culture),
        _ => Month.ToString(culture.DateTimeFormat.YearMonthPattern, culture),
    };

    /// <summary>The weekdays' short names, from the culture's first day of the week.</summary>
    public IReadOnlyList<string> Weekdays
    {
        get
        {
            var names = culture.DateTimeFormat.AbbreviatedDayNames;
            var first = (int)culture.DateTimeFormat.FirstDayOfWeek;
            return [.. Enumerable.Range(0, 7).Select(i => names[(first + i) % 7])];
        }
    }

    /// <summary>What its buttons do in the view it shows, in the person's language.</summary>
    public string PreviousName => View switch
    {
        CalendarView.Year => Locale.Translate("previousYear", "Previous year"),
        CalendarView.Decade => Locale.Translate("previousDecade", "Previous decade"),
        _ => Locale.Translate("previousMonth", "Previous month"),
    };

    public string NextName => View switch
    {
        CalendarView.Year => Locale.Translate("nextYear", "Next year"),
        CalendarView.Decade => Locale.Translate("nextDecade", "Next decade"),
        _ => Locale.Translate("nextMonth", "Next month"),
    };

    /// <summary>What a click on the title does, while there is a wider view to show.</summary>
    public string ZoomOutName => Locale.Translate("calendarZoomOut", "Show more");

    /// <summary>The button of the month before.</summary>
    public Box Previous => new(Bounds.X + Pad, Bounds.Y + Pad + 2, Head - 4, Head - 4);

    /// <summary>The button of the month after.</summary>
    public Box Next => new(Bounds.Right - Pad - (Head - 4), Bounds.Y + Pad + 2, Head - 4, Head - 4);

    /// <summary>The title between the buttons: a click shows the wider view.</summary>
    public Box TitleBox => new(Previous.Right, Bounds.Y + Pad, Math.Max(0, Next.X - Previous.Right), Head);

    static DateOnly First(DateOnly day) => new(day.Year, day.Month, 1);

    /// <summary>The year and decade views' cells, under the title where the month's weekdays and days are.</summary>
    double WideCellWidth => 7.0 * CellWidth / Across;

    double WideCellHeight => (WeekRow + 6 * CellHeight) / 3;

    /// <summary>The place (0 to 11) of a day's month or year in the year or decade view; none when that view does not show it.</summary>
    int? PlaceOf(DateOnly day) => View switch
    {
        CalendarView.Year when day.Year == Day.Year => day.Month - 1,
        CalendarView.Decade when day.Year >= DecadeStart - 1 && day.Year <= DecadeStart + 10 => day.Year - DecadeStart + 1,
        _ => null,
    };

    /// <summary>The first day of the month or year at a place of the year or decade view.</summary>
    DateOnly AtPlace(int place) => View == CalendarView.Year
        ? new DateOnly(Day.Year, place + 1, 1)
        : new DateOnly(Math.Clamp(DecadeStart - 1 + place, 1, 9999), 1, 1);

    Box WideCell(int place) => new(Bounds.X + Pad + place % Across * WideCellWidth, Bounds.Y + Pad + Head + place / Across * WideCellHeight, WideCellWidth, WideCellHeight);

    /// <summary>The name a year or decade view shows for the month or year of <paramref name="day"/>: the culture's short month name, or the year.</summary>
    public string NameOf(DateOnly day) => View == CalendarView.Decade
        ? day.Year.ToString(culture)
        : culture.DateTimeFormat.AbbreviatedMonthNames[day.Month - 1];

    /// <summary>The first day of its grid: the start of the week the month starts in.</summary>
    DateOnly GridStart => Month.AddDays(-(((int)Month.DayOfWeek - (int)culture.DateTimeFormat.FirstDayOfWeek + 7) % 7));

    /// <summary>Where a day's cell is (in the year view its month's, in the decade view its year's); none when the view does not show it.</summary>
    public Box? CellOf(DateOnly day)
    {
        if (View != CalendarView.Month)
            return PlaceOf(day) is { } place ? WideCell(place) : null;
        var at = day.DayNumber - GridStart.DayNumber;
        if (at < 0 || at >= 42)
            return null;
        return new Box(Bounds.X + Pad + at % 7 * CellWidth, Bounds.Y + Pad + Head + WeekRow + at / 7 * CellHeight, CellWidth, CellHeight);
    }

    /// <summary>The day at a point (in the year view a month's first day, in the decade view a year's); none outside its grid.</summary>
    DateOnly? DayAt(double x, double y)
    {
        if (View != CalendarView.Month)
        {
            var across = (int)Math.Floor((x - Bounds.X - Pad) / WideCellWidth);
            var down = (int)Math.Floor((y - Bounds.Y - Pad - Head) / WideCellHeight);
            if (across < 0 || across >= Across || down < 0 || down > 2)
                return null;
            return AtPlace(down * Across + across);
        }
        var col = (int)Math.Floor((x - Bounds.X - Pad) / CellWidth);
        var row = (int)Math.Floor((y - Bounds.Y - Pad - Head - WeekRow) / CellHeight);
        if (col < 0 || col > 6 || row < 0 || row > 5)
            return null;
        return GridStart.AddDays(row * 7 + col);
    }

    /// <summary>Moves the marked day by <paramref name="days"/>; the month follows it.</summary>
    public void Move(int days)
    {
        Day = Day.AddDays(days);
        Month = First(Day);
        Invalidate();
    }

    /// <summary>Turns <paramref name="months"/> months on; the marked day goes along.</summary>
    public void Turn(int months)
    {
        Day = Clamped(Day, months);
        Month = First(Day);
        Invalidate();
    }

    /// <summary>A day moved by a number of months, kept within the days a DateOnly holds.</summary>
    static DateOnly Clamped(DateOnly day, int months)
    {
        var target = (day.Year - 1) * 12L + day.Month - 1 + months;
        if (target < 0)
            return DateOnly.MinValue;
        if (target >= 9999L * 12)
            return DateOnly.MaxValue;
        return day.AddMonths(months);
    }

    /// <summary>Turns one step of the view it shows: a month, a year or a decade.</summary>
    void Step(int way) => Turn(way * View switch
    {
        CalendarView.Year => 12,
        CalendarView.Decade => 120,
        _ => 1,
    });

    /// <summary>Moves the mark in the year or decade view by <paramref name="places"/> months or years.</summary>
    void MoveWide(int places) => Turn(places * (View == CalendarView.Decade ? 12 : 1));

    /// <summary>Shows the wider view: the year from a month, the decade from a year.</summary>
    public void ZoomOut()
    {
        if (View == CalendarView.Decade)
            return;
        View = View == CalendarView.Month ? CalendarView.Year : CalendarView.Decade;
        hovered = null;
        Invalidate();
    }

    /// <summary>Shows the narrower view on <paramref name="at"/>: a year's months, a month's days. The marked day keeps its number where it can.</summary>
    public void ZoomIn(DateOnly at)
    {
        if (View == CalendarView.Month)
            return;
        var year = at.Year;
        var month = View == CalendarView.Year ? at.Month : Day.Month;
        Day = new DateOnly(year, month, Math.Min(Day.Day, DateTime.DaysInMonth(year, month)));
        Month = First(Day);
        View = View == CalendarView.Decade ? CalendarView.Year : CalendarView.Month;
        hovered = null;
        Invalidate();
    }

    /// <summary>Picks <paramref name="day"/>.</summary>
    public void Pick(DateOnly day) => Picked?.Invoke(day);

    /// <summary>Closes it, when it lies over a window.</summary>
    public void Close()
    {
        if (overlay is { } open)
            Screen?.Close(open);
    }

    /// <summary>A key, from itself or from its holder: true when it took it.</summary>
    public bool Keyed(KeyInput key)
    {
        if (key.Control && key.Key == Key.Up)
        {
            ZoomOut();
            return true;
        }
        if (key.Control && key.Key == Key.Down)
        {
            ZoomIn(Day);
            return true;
        }
        if (View != CalendarView.Month)
        {
            switch (key.Key)
            {
                case Key.Left:
                    MoveWide(-1);
                    return true;
                case Key.Right:
                    MoveWide(1);
                    return true;
                case Key.Up:
                    MoveWide(-Across);
                    return true;
                case Key.Down:
                    MoveWide(Across);
                    return true;
                case Key.PageUp:
                    Step(-1);
                    return true;
                case Key.PageDown:
                    Step(1);
                    return true;
                case Key.Enter or Key.Space:
                    ZoomIn(Day);
                    return true;
                case Key.Escape:
                    Close();
                    return true;
            }
            return false;
        }
        switch (key.Key)
        {
            case Key.Left:
                Move(-1);
                return true;
            case Key.Right:
                Move(1);
                return true;
            case Key.Up:
                Move(-7);
                return true;
            case Key.Down:
                Move(7);
                return true;
            case Key.PageUp:
                Turn(-1);
                return true;
            case Key.PageDown:
                Turn(1);
                return true;
            case Key.Home:
                Move(Month.DayNumber - Day.DayNumber);
                return true;
            case Key.End:
                Move(Month.AddMonths(1).AddDays(-1).DayNumber - Day.DayNumber);
                return true;
            case Key.Enter or Key.Space:
                Pick(Day);
                return true;
            case Key.Escape:
                Close();
                return true;
        }
        return false;
    }

    public override bool OnKey(KeyInput key) => Keyed(key);

    public override void OnPointerPressed(PointerInput at)
    {
        pressedOn = Previous.Contains(at.ScreenX, at.ScreenY) ? "previous"
            : Next.Contains(at.ScreenX, at.ScreenY) ? "next"
            : TitleBox.Contains(at.ScreenX, at.ScreenY) ? "title"
            : DayAt(at.ScreenX, at.ScreenY) is { } day ? day
            : null;
        if (Holder is not null)
            Screen?.Focus(Holder);
    }

    public override void OnClick()
    {
        switch (pressedOn)
        {
            case "previous":
                Step(-1);
                break;
            case "next":
                Step(1);
                break;
            case "title":
                ZoomOut();
                break;
            case DateOnly day when View == CalendarView.Month:
                Pick(day);
                break;
            case DateOnly day:
                ZoomIn(day);
                break;
        }
        pressedOn = null;
    }

    /// <summary>Its buttons say what they turn; the title says it shows more while it can.</summary>
    public override string? TooltipAt(double x, double y) =>
        Previous.Contains(x, y) ? PreviousName
        : Next.Contains(x, y) ? NextName
        : TitleBox.Contains(x, y) && View != CalendarView.Decade ? ZoomOutName
        : null;

    public override void OnPointerMoved(PointerInput at)
    {
        var day = DayAt(at.ScreenX, at.ScreenY);
        var onTitle = TitleBox.Contains(at.ScreenX, at.ScreenY);
        if (day != hovered || onTitle != titleHovered)
        {
            hovered = day;
            titleHovered = onTitle;
            Invalidate();
        }
    }

    /// <summary>Where a calendar lies: below <paramref name="under"/>, else above it when there is more room there.</summary>
    public static Box Below(Screen screen, Box under)
    {
        var x = Math.Max(0, Math.Min(under.X, screen.Width - Width));
        var below = under.Bottom + 2;
        if (below + Height > screen.Height && under.Y - 2 - Height >= 0)
            return new Box(x, under.Y - 2 - Height, Width, Height);
        return new Box(x, below, Width, Height);
    }

    /// <summary>
    /// Opens a calendar under <paramref name="under"/> on the month of
    /// <paramref name="day"/> (today's when none): a picked day closes it and
    /// goes to <paramref name="picked"/>. Without a <paramref name="holder"/>
    /// it takes the focus and its keys itself; a press on
    /// <paramref name="anchor"/> (else <paramref name="under"/>) is no press outside.
    /// </summary>
    public static Overlay Open(Screen screen, Box under, DateOnly? day, Action<DateOnly> picked, CultureInfo? culture = null, Element? holder = null, Func<Box>? anchor = null, Action? closed = null)
    {
        var calendar = new DayCalendar(day, culture) { Holder = holder };
        var overlay = new Overlay(calendar, Below(screen, under))
        {
            Anchor = anchor ?? (() => under),
            Closed = closed,
        };
        calendar.overlay = overlay;
        calendar.Picked = d =>
        {
            screen.Close(overlay);
            picked(d);
        };
        screen.Open(overlay);
        screen.Focus(holder ?? calendar);
        return overlay;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var plain = look.Part("list", null);
        var radius = look.Radius("input");
        var text = plain.Text ?? look.Color("text");
        var muted = look.Color("muted");
        var size = look.Size("text");
        var small = look.Size("small");
        painter.Fill(Bounds, plain.Background ?? look.Color("panel"), radius);

        var title = TitleBox;
        if (View != CalendarView.Decade && hovered is null && titleHovered)
            painter.Fill(title, look.Color("raised"), radius);
        painter.Text(Title, title, text, size, "text", bold: true, TextAlign.Center);
        Chevron(painter, Previous, -1, text);
        Chevron(painter, Next, 1, text);
        if (View != CalendarView.Month)
        {
            DrawWide(painter, text, muted, radius);
            painter.Stroke(Bounds, plain.Border ?? look.Color("border"), look.Border, radius);
            return;
        }

        var names = Weekdays;
        for (var i = 0; i < 7; i++)
            painter.Text(names[i], new Box(Bounds.X + Pad + i * CellWidth, Bounds.Y + Pad + Head, CellWidth, WeekRow), muted, small, "text", false, TextAlign.Center);

        var start = GridStart;
        for (var i = 0; i < 42; i++)
        {
            var day = start.AddDays(i);
            var cell = CellOf(day)!.Value.Inset(1);
            var inMonth = day.Month == Month.Month;
            var color = inMonth ? text : muted;
            if (day == Chosen)
            {
                painter.Fill(cell, look.Color("accent"), radius);
                color = look.Color("on_accent");
            }
            else if (day == Day)
                painter.Fill(cell, look.Color("selection"), radius);
            else if (day == hovered)
                painter.Fill(cell, look.Color("raised"), radius);
            if (day == Day && day == Chosen)
                painter.Stroke(cell, look.Color("focus"), 2, radius);
            if (day == Today)
                painter.Stroke(cell.Inset(1), day == Chosen ? look.Color("on_accent") : look.Color("accent"), 1, radius);
            painter.Text(day.Day.ToString(culture), cell, color, small, "text", day == Today, TextAlign.Center);
        }
        painter.Stroke(Bounds, plain.Border ?? look.Color("border"), look.Border, radius);
    }

    /// <summary>
    /// The year view's months or the decade view's years in three rows of
    /// four: the chosen one filled in the accent, the marked one in the
    /// selection, the current one ringed. The years outside the decade are muted.
    /// </summary>
    void DrawWide(Painter painter, SKColor text, SKColor muted, double radius)
    {
        var look = painter.Look;
        var size = look.Size("text");
        for (var place = 0; place < 12; place++)
        {
            var at = AtPlace(place);
            var cell = WideCell(place).Inset(3);
            var same = View == CalendarView.Year
                ? (Func<DateOnly, bool>)(d => d.Year == at.Year && d.Month == at.Month)
                : d => d.Year == at.Year;
            var chosen = Chosen is { } c && same(c);
            var color = View == CalendarView.Decade && (place == 0 || place == 11) ? muted : text;
            if (chosen)
            {
                painter.Fill(cell, look.Color("accent"), radius);
                color = look.Color("on_accent");
            }
            else if (same(Day))
                painter.Fill(cell, look.Color("selection"), radius);
            else if (hovered is { } h && same(h))
                painter.Fill(cell, look.Color("raised"), radius);
            if (chosen && same(Day))
                painter.Stroke(cell, look.Color("focus"), 2, radius);
            var current = same(Today);
            if (current)
                painter.Stroke(cell.Inset(1), chosen ? look.Color("on_accent") : look.Color("accent"), 1, radius);
            painter.Text(NameOf(at), cell, color, size, "text", current, TextAlign.Center);
        }
    }

    /// <summary>A month button's arrow: to the left for the month before, to the right for the one after.</summary>
    static void Chevron(Painter painter, Box box, int way, SKColor color)
    {
        using var pen = new SkiaSharp.SKPaint { Color = color, IsAntialias = true, Style = SkiaSharp.SKPaintStyle.Stroke, StrokeWidth = 1.6f, StrokeCap = SkiaSharp.SKStrokeCap.Round };
        var cx = (float)(box.X + box.Width / 2);
        var cy = (float)(box.Y + box.Height / 2);
        // The point of the arrow lies on the side it turns to.
        painter.Canvas.DrawLine(cx - 2 * way, cy - 4, cx + 2 * way, cy, pen);
        painter.Canvas.DrawLine(cx + 2 * way, cy, cx - 2 * way, cy + 4, pen);
    }
}
