// The tags part, the same as on every stack (conformance/list-fields.json):
// a list field's items as chips, each with a button that removes it, and a
// place to add one after them: a box to type text, a number or a day in
// (taken on Enter or when the box is left), or for a list of an enum a box
// that opens the values the list has not. A typed item is trimmed; empty
// text and an item the list has add nothing, and text that is no value of
// the list's kind stays as typed and is marked. Every change gives the
// record a new list of its own type, so its property tells it changed.
// Enter in the box takes the item and then submits the form it is in. A
// list of days has a calendar button inside the box's right edge (Down in
// the box opens the calendar too). Each chip's remove button says what it
// does, is a Tab stop and is pressed by Enter or Space; when it goes, the
// focus goes on to the next chip's button, else to the place to add. A tags
// part bound to no field keeps its own list: a named one in its part state,
// so the list stays when the window is made again (another language).

using System.Collections;
using System.Globalization;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

[PartKind("tags")]
public sealed class TagsPart : Part, ITakesTyped
{
    const double ChipHeight = 24;
    const double Space = 4;
    const double EditorWidth = 120;
    const double ButtonWidth = 26;

    readonly List<TagChip> chips = [];
    TextEditor? editor;
    TagAdder? adder;
    Overlay? open;
    IList? shownList;
    IList? ownHere;
    int shownCount = -1;

    /// <summary>The chips, in the order of the items.</summary>
    public IReadOnlyList<TagChip> Chips => chips;

    /// <summary>The box a text, a number or a day is typed in; none for a list of an enum, or one that is read only.</summary>
    public TextEditor? Editor => editor;

    /// <summary>The box that opens an enum's values the list has not; none for other lists.</summary>
    public TagAdder Adder => adder ?? throw new InvalidOperationException("only a list of an enum offers its values");

    /// <summary>Whether what is typed is no value of the list's kind: the box is marked, the list stays.</summary>
    public bool Bad { get; private set; }


    /// <summary>The record's list, or its own when it is bound to no field; none when it has none.</summary>
    public IList? Value => Bound is { } path ? Host.Read(path) as IList : Own;

    /// <summary>The list of a part bound to no field: its state's when it is named, else its own.</summary>
    IList? Own
    {
        get => State is { } state ? state.Values.GetValueOrDefault(OwnList) as IList : ownHere;
        set
        {
            if (State is { } state)
                state.SetValue(OwnList, value);
            else
                ownHere = value;
        }
    }

    /// <summary>The name its own list goes by in its part state.</summary>
    const string OwnList = "tags";

    /// <summary>The button inside the box of a list of days that opens the calendar; none for other lists.</summary>
    public CalendarButton? CalendarButton { get; private set; }

    /// <summary>The type of the list's items: text, a whole number, a decimal, a day or an enum.</summary>
    public Type ItemType
    {
        get
        {
            var type = Value?.GetType() ?? (Bound is { } path ? Host.TypeOf(path) : null);
            var item = type?.IsGenericType == true ? type.GetGenericArguments()[0] : type?.GetElementType();
            return item is null ? typeof(string) : Nullable.GetUnderlyingType(item) ?? item;
        }
    }

    /// <summary>An enum's values with their labels, as the host offers them; else the enum's names.</summary>
    IReadOnlyList<Choice> AllChoices
    {
        get
        {
            var offered = Bound is { } path ? Host.Choices(path) : [];
            if (offered.Count > 0 || !ItemType.IsEnum)
                return offered;
            return [.. Enum.GetValues(ItemType).Cast<object>().Select(v => new Choice(v, v.ToString()!))];
        }
    }

    /// <summary>The values of an enum the list has not, to add.</summary>
    public IReadOnlyList<Choice> Offered
    {
        get
        {
            var items = Value?.Cast<object?>().ToList() ?? [];
            return [.. AllChoices.Where(c => !items.Any(i => PressLooks.Same(i, c.Value)))];
        }
    }

    bool Usable => Enabled && !ReadOnly;

    protected internal override void Made()
    {
        PressLooks.Watch(this, Host, State);
        Host.Changed += Sync;
        Sync();
    }

    /// <summary>An item as people read it: an enum's value by its label, the rest as the screen shows a value.</summary>
    string Label(object? item) =>
        AllChoices.FirstOrDefault(c => PressLooks.Same(c.Value, item))?.Label ?? ShownText.Of(item);

    /// <summary>Builds its chips and the place to add again when the list (or whether it may change) is another.</summary>
    void Sync()
    {
        var list = Value;
        var count = list?.Count ?? 0;
        var usable = Usable;
        if (ReferenceEquals(list, shownList) && count == shownCount && (editor is not null || adder is not null) == usable)
            return;
        shownList = list;
        shownCount = count;
        foreach (var chip in chips)
            RemoveChild(chip);
        chips.Clear();
        var items = list?.Cast<object?>().ToList() ?? [];
        for (var i = 0; i < items.Count; i++)
        {
            var at = i;
            TagChip? chip = null;
            chip = new TagChip(Label(items[i]), usable ? () => Removed(chip!, at) : null, () => Usable);
            chips.Add(chip);
            AddChild(chip);
        }
        if (!usable)
        {
            if (editor is not null)
                RemoveChild(editor);
            if (adder is not null)
                RemoveChild(adder);
            if (CalendarButton is not null)
                RemoveChild(CalendarButton);
            editor = null;
            adder = null;
            CalendarButton = null;
        }
        else if (ItemType.IsEnum)
        {
            if (adder is null)
            {
                adder = new TagAdder(this) { Name = Locale.Translate("addTag", "Add") };
                AddChild(adder);
            }
        }
        else if (editor is null)
        {
            editor = new TextEditor
            {
                Placeholder = Locale.Translate("addTag", "Add"),
                Enabled = () => Enabled,
                Marked = () => Bad,
                Entered = () =>
                {
                    Take();
                    return PartTexts.Submit(this);
                },
            };
            editor.FocusMoved += focused =>
            {
                if (focused)
                    return;
                // A press in its own calendar takes the focus for a moment and gives it back.
                if (Screen?.Focused is not null)
                    CloseCalendar();
                Take();
            };
            editor.KeyFirst = DayKey;
            AddChild(editor);
            if (ItemType == typeof(DateOnly))
            {
                editor.RightInset = ButtonWidth + 2;
                CalendarButton = new CalendarButton(ToggleCalendar)
                {
                    Name = Locale.Translate("openCalendar", "Calendar"),
                    Enabled = () => Usable,
                };
                AddChild(CalendarButton);
            }
        }
        else
        {
            // The box stays after the chips, so Tab reaches it last.
            RemoveChild(editor);
            AddChild(editor);
            if (CalendarButton is not null)
            {
                RemoveChild(CalendarButton);
                AddChild(CalendarButton);
            }
        }
        if (Bounds.Width > 0)
            Arrange(Bounds);
        Invalidate();
    }

    /// <summary>The calendar a list of days picks a day from, while it is open.</summary>
    public DayCalendar? Calendar { get; private set; }

    Overlay? calendarOverlay;

    /// <summary>For a list of days: opens the calendar under the box, which keeps the focus; the day picked is added.</summary>
    void OpenCalendar()
    {
        if (ItemType != typeof(DateOnly) || !Usable || calendarOverlay is not null || editor is null || Screen is not { } screen)
            return;
        var under = editor;
        calendarOverlay = DayCalendar.Open(screen, under.Bounds, null, day => Add(day), Host.Formats.Culture, holder: under, anchor: () => under.Bounds, closed: () =>
        {
            calendarOverlay = null;
            Calendar = null;
            Invalidate();
        });
        Calendar = (DayCalendar)calendarOverlay.Content;
    }

    /// <summary>The calendar button: opens the calendar, or closes it when it is open.</summary>
    void ToggleCalendar()
    {
        if (calendarOverlay is null)
            OpenCalendar();
        else
            CloseCalendar();
    }

    void CloseCalendar()
    {
        if (calendarOverlay is { } open)
            Screen?.Close(open);
    }

    /// <summary>Keys in the box of a list of days: while the calendar is open they work it; Down opens it.</summary>
    bool DayKey(KeyInput key)
    {
        if (Calendar is { } calendar)
            return calendar.Keyed(key);
        if (key.Key == Key.Down && ItemType == typeof(DateOnly))
        {
            OpenCalendar();
            return Calendar is not null;
        }
        return false;
    }

    /// <summary>Takes what is typed: an item the list has not, of its kind.</summary>
    public void TakeTyped() => Take();

    /// <summary>Takes what is typed; see <see cref="TakeTyped"/>.</summary>
    public void Take()
    {
        if (editor is null || !Usable)
            return;
        var typed = editor.Text.Trim();
        if (typed == "")
        {
            editor.Text = "";
            Bad = false;
            return;
        }
        var type = ItemType;
        object? item = type switch
        {
            _ when type == typeof(long) => long.TryParse(typed, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n) ? n : null,
            _ when type == typeof(int) => int.TryParse(typed, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n) ? n : null,
            _ when type == typeof(decimal) => decimal.TryParse(typed, NumberStyles.AllowLeadingSign | NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var d) ? d : null,
            _ when type == typeof(DateOnly) => Host.Formats.Read(new FieldFormat(FormatKind.Date), typed) as DateOnly?,
            _ => typed,
        };
        if (item is null)
        {
            Bad = true;
            Invalidate();
            return;
        }
        Bad = false;
        editor.Text = "";
        Add(item);
    }

    /// <summary>Adds <paramref name="item"/> when the list has it not: a change.</summary>
    public void Add(object? item)
    {
        if (!Usable || Value is { } list && list.Cast<object?>().Any(i => PressLooks.Same(i, item)))
            return;
        var next = Copy();
        next.Add(item);
        Offer(next);
    }

    /// <summary>Removes the item at <paramref name="at"/>: a change.</summary>
    public void RemoveAt(int at)
    {
        if (!Usable || Value is not { } list || at < 0 || at >= list.Count)
            return;
        var next = Copy();
        next.RemoveAt(at);
        Offer(next);
    }

    /// <summary>
    /// A chip's button removed its item. When that button had the focus (a
    /// key pressed it, or a click), the focus goes on to the button of the
    /// chip that is now in its place, else to the place to add.
    /// </summary>
    void Removed(TagChip chip, int at)
    {
        var hadFocus = chip.Remove is { } button && Screen?.Focused == button;
        RemoveAt(at);
        if (!hadFocus || Screen is not { } screen || chips.Contains(chip))
            return;
        Element? next = at < chips.Count ? chips[at].Remove : null;
        screen.Focus(next ?? (Element?)editor ?? adder);
    }

    /// <summary>A new list of the record's own type, with its items; a part bound to no field starts a list of text.</summary>
    IList Copy()
    {
        var next = Value is { } list ? (IList)Activator.CreateInstance(list.GetType())! : Bound is null ? new List<string>() : new List<object?>();
        foreach (var item in Value ?? Array.Empty<object?>())
            next.Add(item);
        return next;
    }

    void Offer(IList next)
    {
        if (Bound is { } path)
            Host.Write(Spec.Name ?? "", path, next);
        else
            Own = next;
        Fire("change");
        Sync();
    }

    /// <summary>Opens the enum's values the list has not below the add box, or closes them.</summary>
    internal void ToggleOffered()
    {
        if (open is not null)
        {
            Close();
            return;
        }
        if (!Usable || adder is null || Screen is not { } screen)
            return;
        var offered = Offered;
        if (offered.Count == 0)
            return;
        var list = new DropdownList([.. offered.Select(c => c.Label)], 0, at =>
        {
            Close();
            if (at >= 0 && at < offered.Count)
                Add(offered[at].Value);
        })
        { Holder = adder };
        var width = Math.Max(adder.Bounds.Width, 160);
        open = new Overlay(list, DropdownList.Below(screen, adder.Bounds, width, offered.Count))
        {
            Anchor = () => adder?.Bounds ?? default,
            Closed = () => open = null,
        };
        screen.Open(open);
    }

    void Close()
    {
        if (open is { } was)
            Screen?.Close(was);
        open = null;
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var fonts = FontsNow;
        var look = LookNow;
        var size = look.Size("text");
        // Each line is as high as an input box, as the place to add is; the chips sit in its middle.
        var line = Math.Max(ChipHeight, Math.Min(box.Height, GridLayout.RowHeight));
        var chipTop = (line - ChipHeight) / 2;
        var x = box.X;
        var y = box.Y;
        foreach (var chip in chips)
        {
            var width = chip.WidthFor(fonts, size);
            if (x > box.X && x + width > box.Right)
            {
                x = box.X;
                y += line + Space;
            }
            chip.Arrange(new Box(x, y + chipTop, width, ChipHeight));
            x += width + Space;
        }
        Element? place = (Element?)editor ?? adder;
        if (place is not null)
        {
            if (x > box.X && x + EditorWidth > box.Right)
            {
                x = box.X;
                y += line + Space;
            }
            var at = new Box(x, y, Math.Max(EditorWidth, box.Right - x), line);
            place.Arrange(at);
            CalendarButton?.Arrange(new Box(at.Right - 2 - ButtonWidth, at.Y + 2, ButtonWidth, Math.Max(0, at.Height - 4)));
        }
    }

    public override void Draw(Painter painter)
    {
        Sync();
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        using (painter.Clip(Bounds))
            base.Draw(painter);
    }
}

/// <summary>An item of a tags part: its label in a round plate, with a button that removes it unless the list is read only.</summary>
public sealed class TagChip : Element
{
    const double Pad = 8;
    const double ButtonSide = 18;

    public TagChip(string label, Action? remove, Func<bool>? usable = null)
    {
        Label = label;
        if (remove is not null)
        {
            Remove = new TagRemove(remove) { Name = Locale.Translate("removeTag", "Remove"), Usable = usable ?? (() => true) };
            AddChild(Remove);
        }
    }

    public string Label { get; }

    /// <summary>The button that removes its item; none when the list is read only.</summary>
    public TagRemove? Remove { get; }

    /// <summary>How wide it is with its label in the theme's text size.</summary>
    internal double WidthFor(Fonts fonts, double size) =>
        Pad + fonts.Measure(Label, "text", size, false) + (Remove is null ? Pad : 2 + ButtonSide + 3);

    public override void Arrange(Box box)
    {
        Bounds = box;
        Remove?.Arrange(new Box(box.Right - 3 - ButtonSide, box.Y + (box.Height - ButtonSide) / 2, ButtonSide, ButtonSide));
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var values = look.Part("tags", null);
        painter.Fill(Bounds, values.Background ?? look.Color("raised"), Bounds.Height / 2);
        painter.Stroke(Bounds, values.Border ?? look.Color("border"), look.Border, Bounds.Height / 2);
        var text = Bounds with { X = Bounds.X + Pad, Width = Math.Max(0, Bounds.Width - Pad - (Remove is null ? Pad : ButtonSide + 5)) };
        painter.Text(Label, text, values.Text ?? look.Color("text"), values.Size ?? look.Size("text"), values.Font ?? "text", values.Bold);
        base.Draw(painter);
    }
}

/// <summary>A chip's small round button with a cross, which removes its item: a Tab stop that Enter and Space press, and its name as its tooltip.</summary>
public sealed class TagRemove(Action clicked) : Element
{
    /// <summary>What it does, in the person's language.</summary>
    public string Name { get; init; } = "";

    /// <summary>Whether the list may change now.</summary>
    public Func<bool> Usable { get; init; } = () => true;

    public override bool Focusable => Usable();

    /// <summary>It says what it does when the pointer rests on it.</summary>
    public override string? TooltipAt(double x, double y) => Name.Length > 0 ? Name : null;

    public override void OnClick()
    {
        if (Usable())
            clicked();
    }

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is not (Key.Enter or Key.Space) || key.Control)
            return false;
        OnClick();
        return true;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        if (Hovered || Pressed)
            painter.Fill(Bounds, look.Color(Pressed ? "accent_soft" : "border"), Bounds.Height / 2);
        if (Focused)
            PressLooks.FocusRing(painter, Bounds, Bounds.Height / 2);
        using var pen = new SKPaint { Color = look.Color("muted"), IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.5f, StrokeCap = SKStrokeCap.Round };
        var cx = (float)(Bounds.X + Bounds.Width / 2);
        var cy = (float)(Bounds.Y + Bounds.Height / 2);
        painter.Canvas.DrawLine(cx - 3.5f, cy - 3.5f, cx + 3.5f, cy + 3.5f, pen);
        painter.Canvas.DrawLine(cx + 3.5f, cy - 3.5f, cx - 3.5f, cy + 3.5f, pen);
    }
}

/// <summary>A tags part's place to add one of an enum's values: a box with its word and an arrow, which opens the values the list has not.</summary>
public sealed class TagAdder(TagsPart part) : Element
{
    /// <summary>Its word, in the person's language.</summary>
    public string Name { get; init; } = "";

    public override bool Focusable => part.Enabled;

    public override void OnClick() => part.ToggleOffered();

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is Key.Enter or Key.Space or Key.Down)
        {
            part.ToggleOffered();
            return true;
        }
        return false;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var values = look.Part("input", Focused ? "focus" : null);
        var radius = values.Radius ?? look.Radius("input");
        painter.Fill(Bounds, values.Background ?? look.Color("input"), radius);
        painter.Stroke(Bounds, Hovered ? values.Border ?? look.Color("strong_border") : values.Border ?? look.Color("border"), look.Border, radius);
        if (Focused)
            PressLooks.FocusRing(painter, Bounds, radius);
        var pad = look.Padding * 2;
        painter.Text(Name, Bounds with { X = Bounds.X + pad, Width = Math.Max(0, Bounds.Width - 2 * pad - 18) }, look.Color("muted"), look.Size("text"));
        PressLooks.DownArrow(painter, new Box(Bounds.Right - pad - 18, Bounds.Y, 18, Bounds.Height), values.Text ?? look.Color("text"));
    }
}
