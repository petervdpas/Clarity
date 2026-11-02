// A calendar over the window: one month as a grid of days under its name,
// with buttons for the month before and after. Its weekday and month names
// are the person's culture's. Today is marked, and so is the chosen day; a
// click on a day picks it. Arrows move the marked day, Page Up and Page Down
// a month, Enter picks it and Escape closes the calendar. A date part opens
// it under its input; a table opens it under a date cell.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class DayCalendar : Element
{
    const double Pad = 8;
    const double Head = 32;
    const double WeekRow = 22;
    const double CellWidth = 32;
    const double CellHeight = 28;

    /// <summary>How wide and high a calendar is.</summary>
    public const double Width = 7 * CellWidth + 2 * Pad;

    public const double Height = Pad + Head + WeekRow + 6 * CellHeight + Pad;

    readonly CultureInfo culture;
    Overlay? overlay;
    object? pressedOn;
    DateOnly? hovered;

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

    /// <summary>Today, which it marks.</summary>
    public DateOnly Today { get; set; } = Clock.Today();

    /// <summary>A day was picked.</summary>
    public Action<DateOnly>? Picked { get; set; }

    /// <summary>The element that keeps the focus while it is open (a date part's input), which hands it the keys; none: it takes the focus itself.</summary>
    public Element? Holder { get; init; }

    public override bool Focusable => Holder is null;

    /// <summary>The month's name and year, in the person's culture.</summary>
    public string Title => Month.ToString(culture.DateTimeFormat.YearMonthPattern, culture);

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

    /// <summary>What its month buttons do, in the person's language.</summary>
    public static string PreviousName => Locale.Translate("previousMonth", "Previous month");

    public static string NextName => Locale.Translate("nextMonth", "Next month");

    /// <summary>The button of the month before.</summary>
    public Box Previous => new(Bounds.X + Pad, Bounds.Y + Pad + 2, Head - 4, Head - 4);

    /// <summary>The button of the month after.</summary>
    public Box Next => new(Bounds.Right - Pad - (Head - 4), Bounds.Y + Pad + 2, Head - 4, Head - 4);

    static DateOnly First(DateOnly day) => new(day.Year, day.Month, 1);

    /// <summary>The first day of its grid: the start of the week the month starts in.</summary>
    DateOnly GridStart => Month.AddDays(-(((int)Month.DayOfWeek - (int)culture.DateTimeFormat.FirstDayOfWeek + 7) % 7));

    /// <summary>Where a day's cell is; none when its grid does not show it.</summary>
    public Box? CellOf(DateOnly day)
    {
        var at = day.DayNumber - GridStart.DayNumber;
        if (at < 0 || at >= 42)
            return null;
        return new Box(Bounds.X + Pad + at % 7 * CellWidth, Bounds.Y + Pad + Head + WeekRow + at / 7 * CellHeight, CellWidth, CellHeight);
    }

    /// <summary>The day at a point; none outside its grid.</summary>
    DateOnly? DayAt(double x, double y)
    {
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
        Day = Day.AddMonths(months);
        Month = First(Day);
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
                Turn(-1);
                break;
            case "next":
                Turn(1);
                break;
            case DateOnly day:
                Pick(day);
                break;
        }
        pressedOn = null;
    }

    public override void OnPointerMoved(PointerInput at)
    {
        var day = DayAt(at.ScreenX, at.ScreenY);
        if (day != hovered)
        {
            hovered = day;
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

        var title = new Box(Previous.Right, Bounds.Y + Pad, Next.X - Previous.Right, Head);
        painter.Text(Title, title, text, size, "text", bold: true, TextAlign.Center);
        Chevron(painter, Previous, -1, text);
        Chevron(painter, Next, 1, text);

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

    /// <summary>A month button's arrow: to the left for the month before, to the right for the one after.</summary>
    static void Chevron(Painter painter, Box box, int way, SkiaSharp.SKColor color)
    {
        using var pen = new SkiaSharp.SKPaint { Color = color, IsAntialias = true, Style = SkiaSharp.SKPaintStyle.Stroke, StrokeWidth = 1.6f, StrokeCap = SkiaSharp.SKStrokeCap.Round };
        var cx = (float)(box.X + box.Width / 2);
        var cy = (float)(box.Y + box.Height / 2);
        // The point of the arrow lies on the side it turns to.
        painter.Canvas.DrawLine(cx - 2 * way, cy - 4, cx + 2 * way, cy, pen);
        painter.Canvas.DrawLine(cx + 2 * way, cy, cx - 2 * way, cy + 4, pen);
    }
}
