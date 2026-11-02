// The dropdown (DESIGN 7.9, conformance/forms.json): a closed box showing the
// label of its value's choice, which opens its list over the window below
// it. Only the person picking another choice is a change; what code sets is
// not. Its choices are an enum's values or the records a reference can point
// at (an empty one first), as the host offers them, or the project's
// languages and offered themes for the settings `language` and `theme`.
// Closed, Up and Down pick the choice before or after at once and a typed
// letter jumps to the next choice that starts with it, as a system's
// dropdown does; Enter, Space, Alt+Down or F4 open the list. The list is
// also a split button's menu.

using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>A plain dropdown of choices.</summary>
[PartKind("dropdown")]
public sealed class DropdownPart : Part
{
    Overlay? open;
    DropdownList? list;

    public override bool Focusable => Enabled;


    protected internal override void Made() => PressLooks.Watch(this, Host, State);

    /// <summary>What it offers: the host's choices of its bound field, or the settings' languages and themes.</summary>
    public IReadOnlyList<Choice> Choices => OfferedFor(Host, Bound);

    /// <summary>The choices of a part bound to <paramref name="bound"/>: the host's, else the project's languages or themes for those settings.</summary>
    internal static IReadOnlyList<Choice> OfferedFor(IPartHost host, string? bound)
    {
        if (bound is null)
            return [];
        var offered = host.Choices(bound);
        if (offered.Count > 0)
            return offered;
        return bound switch
        {
            "settings.language" => Choice.LanguagesOffered,
            "settings.theme" => Choice.ThemesOffered,
            _ => [],
        };
    }

    public object? Value => Bound is { } bound ? Host.Read(bound) : null;

    /// <summary>Where its value is among its choices; -1 when it is none of them.</summary>
    public int Chosen
    {
        get
        {
            var choices = Choices;
            var value = Value;
            for (var i = 0; i < choices.Count; i++)
                if (PressLooks.Same(choices[i].Value, value))
                    return i;
            return -1;
        }
    }

    /// <summary>The label it shows: its choice's.</summary>
    public string Shown => Chosen is var at and >= 0 ? Choices[at].Label : "";

    /// <summary>Its list, while it is open.</summary>
    public Overlay? List => open;

    /// <summary>The choice the list marks, which Enter picks.</summary>
    public int Highlighted => list?.Highlighted ?? -1;

    bool Usable => Enabled && !ReadOnly && Bound is not null;

    /// <summary>A click on its box opens its list, or closes it when it is open.</summary>
    public override void OnClick()
    {
        if (open is null)
            Open();
        else
            Close();
    }

    /// <summary>Opens its list below it, its value's choice marked.</summary>
    public void Open()
    {
        if (!Usable || open is not null || Screen is not { } screen)
            return;
        var choices = Choices;
        list = new DropdownList([.. choices.Select(c => c.Label)], Chosen, at =>
        {
            Close();
            Pick(at);
        })
        { Holder = this };
        open = new Overlay(list, DropdownList.Below(screen, Bounds, Bounds.Width, choices.Count))
        {
            // A press on its own box is no press outside: the click closes the list.
            Anchor = () => Bounds,
            Closed = () =>
            {
                open = null;
                list = null;
            },
        };
        screen.Open(open);
        list.Reveal();
    }

    public void Close()
    {
        if (open is { } was)
            Screen?.Close(was);
        open = null;
        list = null;
        Invalidate();
    }

    /// <summary>The person picked the choice at <paramref name="at"/>: a change when it differs from its value.</summary>
    public void Pick(int at)
    {
        var choices = Choices;
        if (!Usable || at < 0 || at >= choices.Count || PressLooks.Same(choices[at].Value, Value))
            return;
        Host.Write(Spec.Name ?? "", Bound!, choices[at].Value);
        Fire("change");
        Invalidate();
    }

    public override bool OnKey(KeyInput key)
    {
        if (list is null)
        {
            if (!Usable || key.Control)
                return false;
            var alt = key.Modifiers.HasFlag(Modifiers.Alt);
            if (key.Key is Key.Enter or Key.Space or Key.F4 || (alt && key.Key is Key.Down or Key.Up))
            {
                Open();
                return true;
            }
            if (alt)
                return false;
            var count = Choices.Count;
            var now = Chosen;
            switch (key.Key)
            {
                case Key.Down:
                    Pick(Math.Min(count - 1, now + 1));
                    return true;
                case Key.Up:
                    Pick(now < 0 ? 0 : Math.Max(0, now - 1));
                    return true;
                case Key.Home:
                    Pick(0);
                    return true;
                case Key.End:
                    Pick(count - 1);
                    return true;
            }
            return false;
        }
        switch (key.Key)
        {
            case Key.Down:
                list.Move(1);
                return true;
            case Key.Up:
                list.Move(-1);
                return true;
            case Key.PageDown:
                list.Move(DropdownList.MostShown);
                return true;
            case Key.PageUp:
                list.Move(-DropdownList.MostShown);
                return true;
            case Key.Home:
                list.Move(-list.Count);
                return true;
            case Key.End:
                list.Move(list.Count);
                return true;
            case Key.Enter or Key.Space:
                var at = list.Highlighted;
                Close();
                Pick(at);
                return true;
            case Key.Escape:
                Close();
                return true;
        }
        return false;
    }

    /// <summary>A typed letter: the next choice whose label starts with it, picked when the list is closed, marked when it is open.</summary>
    public override void OnText(string text)
    {
        if (!Usable || text.Trim().Length == 0)
            return;
        var choices = Choices;
        if (choices.Count == 0)
            return;
        var from = list?.Highlighted ?? Chosen;
        for (var step = 1; step <= choices.Count; step++)
        {
            var at = ((from < 0 ? -1 : from) + step) % choices.Count;
            if (!choices[at].Label.StartsWith(text.Trim(), StringComparison.CurrentCultureIgnoreCase))
                continue;
            if (list is not null)
                list.Mark(at);
            else
                Pick(at);
            return;
        }
    }

    public override void OnFocusChanged(bool focused)
    {
        // Tab moved the focus on; a press in its own list takes the focus from it for a moment and gives it back.
        if (!focused && Screen?.Focused is not null)
            Close();
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var values = look.Part("input", Focused || open is not null ? "focus" : null);
        var radius = values.Radius ?? look.Radius("input");
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        var line = State?.HasError == true ? look.Color("error") : Hovered && Enabled ? values.Border ?? look.Color("strong_border") : values.Border ?? look.Color("border");
        painter.Fill(Bounds, values.Background ?? look.Color("input"), radius);
        painter.Stroke(Bounds, line, look.Border, radius);
        if (Focused)
            PressLooks.FocusRing(painter, Bounds, radius);
        var pad = look.Padding * 2;
        var text = Bounds with { X = Bounds.X + pad, Width = Math.Max(0, Bounds.Width - 2 * pad - 18) };
        painter.Text(Shown, text, values.Text ?? look.Color("text"), values.Size ?? look.Size("text"), values.Font ?? "text", values.Bold);
        PressLooks.DownArrow(painter, new Box(Bounds.Right - pad - 18, Bounds.Y, 18, Bounds.Height), values.Text ?? look.Color("text"));
    }
}

/// <summary>A dropdown's open list, or a split button's menu: one line per label over the window; a press on one picks it.</summary>
public sealed class DropdownList : Element
{
    /// <summary>How high a line is.</summary>
    public const double Line = 28;

    /// <summary>How many lines show at most; the rest scroll.</summary>
    public const int MostShown = 8;

    readonly IReadOnlyList<string> labels;
    readonly Action<int> picked;
    int highlighted;
    int pressedAt = -1;
    double top;

    public DropdownList(IReadOnlyList<string> labels, int highlighted, Action<int> picked)
    {
        this.labels = labels;
        this.highlighted = highlighted;
        this.picked = picked;
    }

    /// <summary>The part it belongs to, which keeps the focus while the person picks.</summary>
    public Element? Holder { get; init; }

    /// <summary>Whether Up past the first line goes to the last, and Down past the last to the first (a typed dropdown's list).</summary>
    public bool Wraps { get; init; }

    /// <summary>Each line's cells, shown in columns as wide as their widest cell (a search's records with their fields); none: each line is its label.</summary>
    public IReadOnlyList<IReadOnlyList<string>>? Cells { get; init; }

    const double ColumnGap = 16;

    /// <summary>Where each cell of line <paramref name="line"/> is drawn, on the screen; none without cells.</summary>
    public IReadOnlyList<Box> CellBoxes(int line)
    {
        if (Cells is not { } cells || line < 0 || line >= cells.Count)
            return [];
        var look = LookNow;
        var fonts = FontsNow;
        var size = look.Part("list", null).Size ?? look.Size("text");
        var pad = look.Padding * 2;
        var count = cells.Max(c => c.Count);
        var widths = Enumerable.Range(0, count).Select(i => cells.Max(c => i < c.Count ? fonts.Measure(c[i], "text", size, false) : 0)).ToList();
        var y = Bounds.Y + 1 + line * Line - top;
        var x = Bounds.X + 1 + pad;
        var boxes = new List<Box>();
        for (var i = 0; i < cells[line].Count; i++)
        {
            var right = i == count - 1 ? Bounds.Right - 1 - pad : x + widths[i];
            boxes.Add(new Box(x, y, Math.Max(0, Math.Min(right, Bounds.Right - 1 - pad) - x), Line));
            x += widths[i] + ColumnGap;
        }
        return boxes;
    }

    public int Count => labels.Count;

    public int Highlighted => highlighted;

    /// <summary>Where a list of <paramref name="count"/> lines lies: below <paramref name="under"/>, else above it when there is more room there.</summary>
    public static Box Below(Screen screen, Box under, double width, int count)
    {
        var height = Math.Max(1, Math.Min(count, MostShown)) * Line + 2;
        var x = Math.Max(0, Math.Min(under.X, screen.Width - width));
        var below = under.Bottom + 2;
        if (below + height > screen.Height && under.Y - 2 - height >= 0)
            return new Box(x, under.Y - 2 - height, width, height);
        return new Box(x, below, width, height);
    }

    double Scrollable => Math.Max(0, labels.Count * Line - (Bounds.Height - 2));

    /// <summary>The line at a height in its own place; -1 for none.</summary>
    int LineAt(double y)
    {
        var at = (int)Math.Floor((y - 1 + top) / Line);
        return at >= 0 && at < labels.Count ? at : -1;
    }

    /// <summary>Marks the line <paramref name="by"/> lines on, and scrolls it into view.</summary>
    public void Move(int by)
    {
        if (labels.Count == 0)
            return;
        var next = highlighted < 0 ? (by > 0 ? by - 1 : labels.Count + by) : highlighted + by;
        // A single step past an end goes round when it wraps; a page or Home and End stop at the ends.
        if (Wraps && Math.Abs(by) == 1 && highlighted >= 0)
            next = (next + labels.Count) % labels.Count;
        highlighted = Math.Clamp(next, 0, labels.Count - 1);
        Reveal();
    }

    /// <summary>Marks the line <paramref name="at"/>, and scrolls it into view.</summary>
    public void Mark(int at)
    {
        if (at < 0 || at >= labels.Count)
            return;
        highlighted = at;
        Reveal();
    }

    /// <summary>Scrolls the marked line into view.</summary>
    public void Reveal()
    {
        if (highlighted < 0)
            return;
        var shown = Bounds.Height - 2;
        if (highlighted * Line < top)
            top = highlighted * Line;
        else if ((highlighted + 1) * Line > top + shown)
            top = (highlighted + 1) * Line - shown;
        top = Math.Clamp(top, 0, Scrollable);
        Invalidate();
    }

    public override void OnPointerMoved(PointerInput at)
    {
        var line = LineAt(at.Y);
        if (line >= 0 && line != highlighted)
        {
            highlighted = line;
            Invalidate();
        }
    }

    public override void OnPointerPressed(PointerInput at)
    {
        pressedAt = LineAt(at.Y);
        if (Holder is not null)
            Screen?.Focus(Holder);
    }

    public override void OnClick()
    {
        if (pressedAt >= 0)
            picked(pressedAt);
    }

    public override bool OnWheel(double dx, double dy)
    {
        var next = Math.Clamp(top + dy, 0, Scrollable);
        if (next == top)
            return false;
        top = next;
        return true;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var plain = look.Part("list", null);
        var radius = look.Radius("input");
        painter.Fill(Bounds, plain.Background ?? look.Color("panel"), radius);
        using (painter.Clip(Bounds.Inset(1)))
        {
            for (var i = (int)(top / Line); i < labels.Count && i * Line - top < Bounds.Height; i++)
            {
                var line = new Box(Bounds.X + 1, Bounds.Y + 1 + i * Line - top, Bounds.Width - 2, Line);
                var values = i == highlighted ? look.Part("list", "hover") : plain;
                if (i == highlighted)
                    painter.Fill(line, values.Background ?? look.Color("selection"));
                var pad = look.Padding * 2;
                var color = values.Text ?? look.Color("text");
                if (Cells is { } cells && i < cells.Count)
                {
                    // The first cell in the text's color, the fields after it muted.
                    var boxes = CellBoxes(i);
                    for (var c = 0; c < boxes.Count; c++)
                        painter.Text(cells[i][c], boxes[c], c == 0 ? color : look.Color("muted"), values.Size ?? look.Size("text"), values.Font ?? "text", values.Bold);
                }
                else
                    painter.Text(labels[i], line with { X = line.X + pad, Width = Math.Max(0, line.Width - 2 * pad) }, color, values.Size ?? look.Size("text"), values.Font ?? "text", values.Bold);
            }
        }
        painter.Stroke(Bounds, plain.Border ?? look.Color("border"), look.Border, radius);
    }
}
