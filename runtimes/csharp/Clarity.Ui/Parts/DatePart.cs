// The date part, the same as on every stack (conformance/forms.json,
// tables.json): a day, or a moment when its field holds a date and time.
// The day shows in the person's format and typed text is read the same way
// (or as 2026-10-20) when the input is left or Enter is pressed; a button at
// its right opens a calendar under it to pick the day from. A moment has its
// time beside the day, typed as hh:mm on a 24-hour clock; picking or typing
// a day keeps the time, and an empty moment starts at midnight. Only the
// person's pick or typed change is a change; the same value again and what
// code sets are not. Text that is no day or no time is marked and changes
// nothing; a cleared day clears the value. Bound, it writes a DateOnly for a
// day and a DateTime for a moment; unbound and named, it keeps the value in
// its state as 2026-10-20 or 2026-10-20T14:05:00.

using System.Globalization;
using System.Text.RegularExpressions;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

[PartKind("date")]
public sealed partial class DatePart : Part, ITakesTyped
{
    const double ButtonWidth = 26;
    const double TimeWidth = 64;
    const double Gap = 4;

    TextEditor editor = null!;
    TextEditor? time;
    string own = "";
    string? shownDay;
    string? shownTime;
    Overlay? open;
    DayCalendar? calendar;

    /// <summary>Where the day is typed.</summary>
    public TextEditor Editor => editor;

    /// <summary>Where a moment's time is typed; none for a day.</summary>
    public TextEditor? Time => time;

    /// <summary>The button that opens the calendar.</summary>
    public CalendarButton Button { get; private set; } = null!;

    /// <summary>Whether it holds a moment: its field is a date and time.</summary>
    public bool IsMoment { get; private set; }

    /// <summary>What was typed as the day while it is no day; null when the input shows its value.</summary>
    public string? Bad { get; private set; }

    /// <summary>What was typed as the time while it is no time.</summary>
    public string? BadTime { get; private set; }

    /// <summary>Its calendar, while it is open.</summary>
    public Overlay? Open => open;


    bool Usable => Enabled && !ReadOnly;

    /// <summary>How the day shows: the field's named format (iso, long) when it has one.</summary>
    FieldFormat DayFormat => new(FormatKind.Date, null, Bound is { } path ? this.FormatOf(path)?.Format : null);

    /// <summary>The value as a moment (a day at midnight); none when it is empty or no date.</summary>
    public DateTime? Value => MomentOf(Bound is { } path ? Host.Read(path) : State?.Text ?? own);

    /// <summary>Its day; none when it is empty.</summary>
    public DateOnly? Day => Value is { } v ? DateOnly.FromDateTime(v) : null;

    /// <summary>A value as a moment: a DateTime, a DateOnly, or stored text (2026-10-20, 2026-10-20T14:05:00).</summary>
    static DateTime? MomentOf(object? value) => value switch
    {
        DateTime t => t,
        DateOnly d => d.ToDateTime(TimeOnly.MinValue),
        DateTimeOffset o => o.DateTime,
        string s when s.Trim().Length > 0 && DateTime.TryParse(s.Trim(), CultureInfo.InvariantCulture, DateTimeStyles.AllowWhiteSpaces, out var t) => t,
        _ => null,
    };

    protected internal override void Made()
    {
        IsMoment = Bound is { } path && this.FormatOf(path)?.Kind == FormatKind.DateTime;
        editor = new TextEditor
        {
            Placeholder = Spec.Text("placeholder") is { } placeholder ? Host.Text(placeholder) : null,
            Enabled = () => Enabled,
            ReadOnly = () => ReadOnly,
            Marked = () => Bad is not null || State?.HasError == true,
            Entered = () =>
            {
                Take();
                return PartTexts.Submit(this);
            },
            KeyFirst = DayKey,
            RightInset = ButtonWidth + 2,
        };
        editor.FocusMoved += focused => Moved(focused, Take);
        AddChild(editor);
        Button = new CalendarButton(Toggle)
        {
            Name = Locale.Translate("openCalendar", "Calendar"),
            Enabled = () => Usable,
        };
        AddChild(Button);
        if (IsMoment)
        {
            time = new TextEditor
            {
                Placeholder = "hh:mm",
                Align = TextAlign.Center,
                Enabled = () => Enabled,
                ReadOnly = () => ReadOnly,
                Marked = () => BadTime is not null || State?.HasError == true,
                Entered = () =>
                {
                    TakeTime();
                    return PartTexts.Submit(this);
                },
            };
            time.FocusMoved += focused => Moved(focused, TakeTime);
            AddChild(time);
        }
        Host.Changed += Sync;
        Sync();
    }

    /// <summary>One of its inputs got or lost the focus: leaving takes what was typed there.</summary>
    void Moved(bool focused, Action take)
    {
        if (focused)
        {
            Fire("enter");
            return;
        }
        // A press in its own calendar takes the focus for a moment and gives it back.
        if (Screen?.Focused is not null)
            Close();
        take();
        Fire("leave");
    }

    /// <summary>A value set from elsewhere (code, the record read again) shows as it is, and is no longer marked.</summary>
    void Sync()
    {
        var value = Value;
        var day = value is { } v ? this.FormatsOf().Show(DayFormat, DateOnly.FromDateTime(v)) : "";
        if (day != shownDay)
        {
            shownDay = day;
            Bad = null;
            editor.Text = day;
        }
        if (time is null)
            return;
        var clock = value is { } m ? m.ToString("HH:mm", CultureInfo.InvariantCulture) : "";
        if (clock != shownTime)
        {
            shownTime = clock;
            BadTime = null;
            time.Text = clock;
        }
    }

    /// <summary>A key that runs a command has it take the typed day and time first.</summary>
    public void TakeTyped()
    {
        Take();
        TakeTime();
    }

    /// <summary>Takes the typed day: a change when it is a day that differs; cleared, the value is emptied.</summary>
    public void Take()
    {
        Sync();
        var typed = editor.Text;
        if (Bad is null && typed == shownDay)
            return;
        if (typed.Trim().Length == 0)
        {
            if (Bound is { } path && Host.Field(path)?.Required == true)
            {
                Bad = typed;
                Invalidate();
                return;
            }
            Bad = null;
            Offer(null);
            return;
        }
        if (ReadDay(this.FormatsOf().Culture, typed) is not { } day)
        {
            Bad = typed;
            Invalidate();
            return;
        }
        Bad = null;
        Offer(day.ToDateTime(TimeOnly.MinValue) + (IsMoment ? Value?.TimeOfDay ?? TimeSpan.Zero : TimeSpan.Zero));
    }

    /// <summary>Takes a moment's typed time; without a day it waits for one.</summary>
    public void TakeTime()
    {
        if (time is null)
            return;
        Sync();
        var typed = time.Text;
        if (BadTime is null && typed == shownTime)
            return;
        if (ReadTime(typed) is not { } clock)
        {
            BadTime = typed;
            Invalidate();
            return;
        }
        BadTime = null;
        if (Value is { } value)
            Offer(value.Date + clock);
    }

    /// <summary>The person picked <paramref name="day"/> from the calendar: a moment keeps its time.</summary>
    public void Pick(DateOnly day)
    {
        if (!Usable)
            return;
        Bad = null;
        Offer(day.ToDateTime(TimeOnly.MinValue) + (IsMoment ? Value?.TimeOfDay ?? TimeSpan.Zero : TimeSpan.Zero));
    }

    /// <summary>Makes <paramref name="next"/> the value when it differs; else the inputs show the value again.</summary>
    void Offer(DateTime? next)
    {
        if (!IsMoment && next is { } d)
            next = d.Date;
        if (next == (IsMoment ? Value : Value?.Date))
        {
            shownDay = shownTime = null;
            Sync();
            return;
        }
        if (Bound is { } path)
            Host.Write(Spec.Name ?? "", path, next is not { } n ? null : IsMoment ? n : DateOnly.FromDateTime(n));
        else
        {
            var stored = next is not { } n ? "" : n.ToString(IsMoment ? "yyyy-MM-dd'T'HH:mm:ss" : "yyyy-MM-dd", CultureInfo.InvariantCulture);
            if (State is { } state)
            {
                state.Text = stored;
                Host.Edited(Spec.Name!);
            }
            else
                own = stored;
        }
        Sync();
        Fire("change");
        Invalidate();
    }

    /// <summary>
    /// A typed day in <paramref name="culture"/>: as the culture writes it
    /// short or long, as 2026-10-20, or as the culture otherwise reads a
    /// date; none when it is no day.
    /// </summary>
    public static DateOnly? ReadDay(CultureInfo culture, string typed)
    {
        var text = typed.Trim();
        if (text.Length == 0)
            return null;
        var f = culture.DateTimeFormat;
        string[] patterns = ["yyyy-MM-dd", f.ShortDatePattern, f.LongDatePattern, Weekless().Replace(f.LongDatePattern, "").Trim(' ', ',')];
        if (DateOnly.TryParseExact(text, patterns, culture, DateTimeStyles.AllowWhiteSpaces, out var exact))
            return exact;
        return DateOnly.TryParse(text, culture, DateTimeStyles.AllowWhiteSpaces, out var day) ? day : null;
    }

    /// <summary>A typed time, h:mm or hh:mm on a 24-hour clock; empty is midnight; none when it is no time.</summary>
    public static TimeSpan? ReadTime(string typed)
    {
        var text = typed.Trim();
        if (text.Length == 0)
            return TimeSpan.Zero;
        var m = Clock24().Match(text);
        if (!m.Success)
            return null;
        var (hours, minutes) = (int.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture), int.Parse(m.Groups[2].Value, CultureInfo.InvariantCulture));
        return hours < 24 && minutes < 60 ? new TimeSpan(hours, minutes, 0) : null;
    }

    [GeneratedRegex(@"^(\d{1,2}):(\d{2})$")]
    private static partial Regex Clock24();

    [GeneratedRegex("d{4,}")]
    private static partial Regex Weekless();

    /// <summary>Keys in the day's input: while the calendar is open they work it; Down opens it.</summary>
    bool DayKey(KeyInput key)
    {
        if (calendar is not null)
            return calendar.Keyed(key);
        if (key.Key == Key.Down && Usable)
        {
            OpenCalendar();
            return true;
        }
        return false;
    }

    /// <summary>The button: opens the calendar, or closes it when it is open.</summary>
    void Toggle()
    {
        if (open is null)
            OpenCalendar();
        else
            Close();
    }

    /// <summary>Opens the calendar under the day's input, on the value's month; the day's input keeps the focus.</summary>
    public void OpenCalendar()
    {
        if (!Usable || open is not null || Screen is not { } screen)
            return;
        open = DayCalendar.Open(screen, editor.Bounds, Day, picked =>
        {
            Pick(picked);
        }, this.FormatsOf().Culture, holder: editor, anchor: () => Bounds, closed: () =>
        {
            open = null;
            calendar = null;
            Invalidate();
        });
        calendar = (DayCalendar)open.Content;
    }

    public void Close()
    {
        if (open is { } was)
            Screen?.Close(was);
        open = null;
        calendar = null;
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var day = time is null ? box : box with { Width = Math.Max(0, box.Width - TimeWidth - Gap) };
        editor.Arrange(day);
        Button.Arrange(new Box(day.Right - 2 - ButtonWidth, day.Y + 2, ButtonWidth, Math.Max(0, day.Height - 4)));
        time?.Arrange(new Box(day.Right + Gap, box.Y, Math.Min(TimeWidth, box.Width), box.Height));
    }

    public override void Draw(Painter painter)
    {
        Sync();
        base.Draw(painter);
    }
}

/// <summary>The small button at a date input's right that opens its calendar: a calendar sheet.</summary>
public sealed class CalendarButton(Action clicked) : Element
{
    /// <summary>What it does, in the person's language.</summary>
    public string Name { get; set; } = "";

    public Func<bool> Enabled { get; set; } = () => true;

    public override void OnClick()
    {
        if (Enabled())
            clicked();
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        if (Enabled() && (Hovered || Pressed))
            painter.Fill(Bounds, look.Color(Pressed ? "accent_soft" : "raised"), look.Radius("input"));
        using var pen = new SKPaint { Color = look.Color(Enabled() ? "text" : "muted"), IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.4f, StrokeCap = SKStrokeCap.Round };
        var cx = (float)(Bounds.X + Bounds.Width / 2);
        var cy = (float)(Bounds.Y + Bounds.Height / 2);
        var canvas = painter.Canvas;
        canvas.DrawRoundRect(new SKRect(cx - 6, cy - 5, cx + 6, cy + 6), 1.5f, 1.5f, pen);
        canvas.DrawLine(cx - 6, cy - 1, cx + 6, cy - 1, pen);
        canvas.DrawLine(cx - 3, cy - 7, cx - 3, cy - 4, pen);
        canvas.DrawLine(cx + 3, cy - 7, cx + 3, cy - 4, pen);
    }
}
