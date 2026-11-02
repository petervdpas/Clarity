// A place to type text: one line, or several lines that wrap (a memo). It
// has a caret and a selection, takes typed text, the keys that move and
// delete, copying, cutting and pasting through the screen's clipboard, and
// the pointer: a press places the caret, a drag selects, a double click
// selects a word. A long line scrolls sideways; wrapped lines scroll up and
// down. Empty, it shows its placeholder; a password shows dots. The parts
// that edit text (text box, memo, password, number, spin box) hold one, and
// say what they do with the text through its events.

using SkiaSharp;

namespace Clarity.Ui;

public sealed class TextEditor(bool multiline = false) : Element
{
    string text = "";
    int caret;
    int anchor;
    bool dragging;
    bool follow;

    /// <summary>Whether it holds several lines, which wrap; Enter is then a new line.</summary>
    public bool Multiline { get; } = multiline;

    /// <summary>The text it holds. Setting it is no edit: the caret goes to its end.</summary>
    public string Text
    {
        get => text;
        set
        {
            var clean = Clean(value ?? "");
            if (clean == text)
                return;
            text = clean;
            caret = anchor = text.Length;
            layout = null;
            // With the focus the caret shows at its end; else it shows from its start.
            follow = Focused;
            if (!Focused)
                ScrollX = ScrollY = 0;
            Invalidate();
        }
    }

    /// <summary>Where the caret is, as a place in the text (0 is before the first character).</summary>
    public int Caret => caret;

    /// <summary>The selection, from its start to its end; empty when the two are the same.</summary>
    public (int Start, int End) Selection => (Math.Min(caret, anchor), Math.Max(caret, anchor));

    public string SelectedText => text[Selection.Start..Selection.End];

    /// <summary>Shown while it is empty, in the muted color.</summary>
    public string? Placeholder { get; set; }

    /// <summary>A password: each character shows as a dot, and it is not copied or cut.</summary>
    public bool Password
    {
        get => password;
        set
        {
            if (password == value)
                return;
            password = value;
            layout = null;
            Invalidate();
        }
    }
    bool password;

    /// <summary>Whether the person may use it at all; disabled it takes no focus and draws faded.</summary>
    public Func<bool> Enabled { get; set; } = () => true;

    /// <summary>Whether it is read-only: the caret moves and it copies, but nothing is typed.</summary>
    public Func<bool> ReadOnly { get; set; } = () => false;

    /// <summary>Whether it is marked as holding something wrong: its border is the error color.</summary>
    public Func<bool> Marked { get; set; } = () => false;

    /// <summary>How wide the place inside its right edge is that its holder draws in (a password's buttons, a spin box's arrows).</summary>
    public double RightInset { get; set; }

    public TextAlign Align { get; set; } = TextAlign.Left;

    /// <summary>Whether it draws the input's frame; without one it is text to read (a text part over several rows).</summary>
    public bool Framed { get; set; } = true;

    /// <summary>Whether a press or Tab gives it the focus.</summary>
    public bool TakesFocus { get; set; } = true;

    /// <summary>The person changed the text: typed, deleted, cut or pasted.</summary>
    public event Action? Edited;

    /// <summary>Enter in one line: true when the holder took it.</summary>
    public Func<bool>? Entered { get; set; }

    /// <summary>A key, before it does what it does here: true when the holder took it (a spin box's Up and Down).</summary>
    public Func<KeyInput, bool>? KeyFirst { get; set; }

    /// <summary>Whether Tab and Shift+Tab come to <see cref="KeyFirst"/> instead of moving the focus (a table's cell editor goes on to the next cell).</summary>
    public bool TakesTab { get; set; }

    public override bool WantsTab => TakesTab;

    /// <summary>It got (true) or lost (false) the focus.</summary>
    public event Action<bool>? FocusMoved;

    /// <summary>How far its text is scrolled: sideways in one line, down in several.</summary>
    public double ScrollX { get; private set; }

    public double ScrollY { get; private set; }

    public override bool Focusable => TakesFocus && Enabled();

    bool Editable => Enabled() && !ReadOnly();

    /// <summary>Selects from <paramref name="from"/> to <paramref name="to"/>, the caret at <paramref name="to"/>.</summary>
    public void Select(int from, int to)
    {
        anchor = Math.Clamp(from, 0, text.Length);
        caret = Math.Clamp(to, 0, text.Length);
        Moved();
    }

    public void SelectAll() => Select(0, text.Length);

    /// <summary>Text as it can stand here: one line has no line breaks, several lines have only <c>\n</c>.</summary>
    string Clean(string value)
    {
        value = value.Replace("\r\n", "\n").Replace('\r', '\n');
        if (!Multiline)
            value = value.Replace('\n', ' ');
        return value;
    }

    void Moved()
    {
        follow = true;
        Follow();
        Invalidate();
    }

    void Changed()
    {
        layout = null;
        Moved();
        Edited?.Invoke();
    }

    /// <summary>Puts <paramref name="typed"/> where the selection is, as the person typed it.</summary>
    void Insert(string typed)
    {
        var clean = Clean(typed);
        var (start, end) = Selection;
        if (clean.Length == 0 && start == end)
            return;
        text = text[..start] + clean + text[end..];
        caret = anchor = start + clean.Length;
        Changed();
    }

    void DeleteSelection()
    {
        var (start, end) = Selection;
        text = text[..start] + text[end..];
        caret = anchor = start;
        Changed();
    }

    public override void OnText(string typed)
    {
        if (!Editable)
            return;
        // Keys that type no character (Backspace, Escape) come as keys.
        var printable = new string([.. typed.Where(c => !char.IsControl(c) || c == '\t' || (Multiline && c == '\n'))]);
        if (printable.Length > 0)
            Insert(printable);
    }

    public override bool OnKey(KeyInput key)
    {
        if (KeyFirst?.Invoke(key) == true)
            return true;
        var command = key.Control || key.Modifiers.HasFlag(Modifiers.Meta);
        var shift = key.Shift;
        switch (key.Key)
        {
            case Key.Left:
                MoveTo(command ? WordBefore(caret) : caret > 0 && anchor != caret && !shift ? Selection.Start : Before(caret), shift);
                return true;
            case Key.Right:
                MoveTo(command ? WordAfter(caret) : anchor != caret && !shift ? Selection.End : After(caret), shift);
                return true;
            case Key.Home:
                MoveTo(command || !Multiline ? 0 : LineStart(caret), shift);
                return true;
            case Key.End:
                MoveTo(command || !Multiline ? text.Length : LineEnd(caret), shift);
                return true;
            case Key.Up when Multiline:
                MoveTo(Vertically(-1), shift);
                return true;
            case Key.Down when Multiline:
                MoveTo(Vertically(1), shift);
                return true;
            case Key.Backspace:
                if (!Editable)
                    return true;
                if (caret == anchor && caret > 0)
                    anchor = command ? WordBefore(caret) : Before(caret);
                if (caret != anchor)
                    DeleteSelection();
                return true;
            case Key.Delete:
                if (!Editable)
                    return true;
                if (caret == anchor && caret < text.Length)
                    anchor = command ? WordAfter(caret) : After(caret);
                if (caret != anchor)
                    DeleteSelection();
                return true;
            case Key.Enter when Multiline:
                if (Editable)
                    Insert("\n");
                return true;
            case Key.Enter:
                return Entered?.Invoke() ?? false;
            case Key.A when command:
                SelectAll();
                return true;
            case Key.C when command:
                if (caret != anchor && !password)
                    Screen?.Clipboard.SetText(SelectedText);
                return true;
            case Key.X when command:
                if (caret != anchor && !password && Editable)
                {
                    Screen?.Clipboard.SetText(SelectedText);
                    DeleteSelection();
                }
                return true;
            case Key.V when command:
                if (Editable && Screen?.Clipboard.GetText() is { Length: > 0 } pasted)
                    Insert(pasted);
                return true;
        }
        return false;
    }

    void MoveTo(int place, bool extend)
    {
        caret = Math.Clamp(place, 0, text.Length);
        if (!extend)
            anchor = caret;
        Moved();
    }

    int Before(int at) => at <= 0 ? 0 : at >= 2 && char.IsLowSurrogate(text[at - 1]) && char.IsHighSurrogate(text[at - 2]) ? at - 2 : at - 1;

    int After(int at) => at >= text.Length ? text.Length : at + 1 < text.Length && char.IsHighSurrogate(text[at]) && char.IsLowSurrogate(text[at + 1]) ? at + 2 : at + 1;

    static bool IsWordChar(char c) => char.IsLetterOrDigit(c) || c == '_';

    /// <summary>The start of the word before a place: past spaces, then past the word.</summary>
    int WordBefore(int at)
    {
        while (at > 0 && !IsWordChar(text[at - 1]))
            at--;
        while (at > 0 && IsWordChar(text[at - 1]))
            at--;
        return at;
    }

    int WordAfter(int at)
    {
        while (at < text.Length && !IsWordChar(text[at]))
            at++;
        while (at < text.Length && IsWordChar(text[at]))
            at++;
        return at;
    }

    /// <summary>The word at a place, or the one character there when it is no word (a space, a sign).</summary>
    (int Start, int End) WordAt(int at)
    {
        if (text.Length == 0)
            return (0, 0);
        var i = Math.Min(at, text.Length - 1);
        if (!IsWordChar(text[i]) && i > 0 && IsWordChar(text[i - 1]))
            i--;
        if (!IsWordChar(text[i]))
            return (i, After(i));
        var start = i;
        while (start > 0 && IsWordChar(text[start - 1]))
            start--;
        var end = i;
        while (end < text.Length && IsWordChar(text[end]))
            end++;
        return (start, end);
    }

    // -- The pointer --------------------------------------------------------

    public override void OnPointerPressed(PointerInput at)
    {
        if (!Enabled())
            return;
        var place = PlaceAt(at.X, at.Y);
        if (at.Clicks >= 3)
        {
            Select(0, text.Length);
            return;
        }
        if (at.Clicks == 2)
        {
            var (start, end) = WordAt(place);
            Select(start, end);
            return;
        }
        dragging = true;
        MoveTo(place, at.Modifiers.HasFlag(Modifiers.Shift));
    }

    public override void OnPointerMoved(PointerInput at)
    {
        if (dragging && Pressed)
            MoveTo(PlaceAt(at.X, at.Y), extend: true);
    }

    public override void OnPointerReleased(PointerInput at) => dragging = false;

    /// <summary>The wheel scrolls lines that do not fit; a positive turn shows what is further down.</summary>
    public override bool OnWheel(double dx, double dy)
    {
        if (!Multiline)
            return false;
        var lines = Layout();
        var most = Math.Max(0, lines.Lines.Count * lines.Height - Inner.Height);
        if (most <= 0)
            return false;
        var was = ScrollY;
        ScrollY = Math.Clamp(ScrollY + dy, 0, most);
        follow = false;
        Invalidate();
        return ScrollY != was;
    }

    public override void OnFocusChanged(bool focused)
    {
        dragging = false;
        FocusMoved?.Invoke(focused);
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        if (box.Width != Bounds.Width)
            layout = null;
        Bounds = box;
        Follow();
    }

    // -- Lines, as they are laid out ----------------------------------------

    /// <summary>One line as drawn: where it starts and ends in the text, without a line break that ends it.</summary>
    readonly record struct Line(int Start, int End);

    /// <summary>The lines, how high each is, and where each place of the text is across its line.</summary>
    sealed record Laid(List<Line> Lines, double Height, double[] Across, double Width, Fonts Fonts, double Size);

    Laid? layout;

    double Size => LookNow.Size("text");

    double SideGap => Framed ? LookNow.Padding * 2 : 0;

    /// <summary>The place inside the frame the text is drawn in.</summary>
    Box Inner
    {
        get
        {
            var top = Multiline && Framed ? LookNow.Padding + LookNow.Border : 0;
            return new Box(Bounds.X + SideGap, Bounds.Y + top, Math.Max(1, Bounds.Width - 2 * SideGap - RightInset), Math.Max(1, Bounds.Height - 2 * top));
        }
    }

    /// <summary>What is drawn for the text: the text itself, or a dot for each character of a password.</summary>
    string Shown => password ? new string('\u2022', text.Length) : text;

    Laid Layout()
    {
        if (layout is { } laid && Math.Abs(laid.Width - Inner.Width) < 0.5 && laid.Fonts == FontsNow && laid.Size == Size)
            return laid;
        using var font = FontsNow.Font("text", Size, false);
        var shown = Shown;
        // Where each place is from the start of the text, as each character's width adds up.
        var from = new double[shown.Length + 1];
        var widths = new Dictionary<string, double>();
        for (var i = 0; i < shown.Length; i++)
        {
            var one = char.IsHighSurrogate(shown[i]) && i + 1 < shown.Length ? shown.Substring(i, 2) : shown[i].ToString();
            var w = one == "\n" ? 0 : widths.TryGetValue(one, out var known) ? known : widths[one] = font.MeasureText(one);
            from[i + 1] = from[i] + w;
            if (one.Length == 2)
            {
                from[i + 2] = from[i + 1];
                i++;
            }
        }
        var lines = Multiline ? Wrap(shown, from, Inner.Width) : [new Line(0, shown.Length)];
        // Across is measured from the start of each place's own line.
        var across = new double[shown.Length + 1];
        foreach (var line in lines)
            for (var i = line.Start; i <= line.End; i++)
                across[i] = from[i] - from[line.Start];
        return layout = new Laid(lines, font.Spacing, across, Inner.Width, FontsNow, Size);
    }

    /// <summary>
    /// Lines of at most <paramref name="width"/>: each paragraph broken after
    /// the last space that fits, or within a word longer than the line.
    /// </summary>
    static List<Line> Wrap(string shown, double[] from, double width)
    {
        var lines = new List<Line>();
        var start = 0;
        while (true)
        {
            var newline = shown.IndexOf('\n', start);
            var end = newline < 0 ? shown.Length : newline;
            var lineStart = start;
            while (from[end] - from[lineStart] > width)
            {
                var cut = lineStart + 1;
                while (cut < end && from[cut + 1] - from[lineStart] <= width)
                    cut++;
                // After the last space within the line, when there is one.
                var space = shown.LastIndexOf(' ', cut - 1, cut - lineStart);
                var breakAt = space >= lineStart && space + 1 > lineStart && space + 1 < end ? space + 1 : cut;
                lines.Add(new Line(lineStart, breakAt));
                lineStart = breakAt;
            }
            lines.Add(new Line(lineStart, end));
            if (newline < 0)
                return lines;
            start = newline + 1;
        }
    }

    int LineOf(Laid laid, int place)
    {
        for (var i = laid.Lines.Count - 1; i >= 0; i--)
            if (laid.Lines[i].Start <= place)
                return i;
        return 0;
    }

    int LineStart(int place)
    {
        var laid = Layout();
        return laid.Lines[LineOf(laid, place)].Start;
    }

    int LineEnd(int place)
    {
        var laid = Layout();
        return laid.Lines[LineOf(laid, place)].End;
    }

    /// <summary>The place a line up or down, as near across as the caret is.</summary>
    int Vertically(int by)
    {
        var laid = Layout();
        var line = LineOf(laid, caret);
        var to = line + by;
        if (to < 0)
            return 0;
        if (to >= laid.Lines.Count)
            return text.Length;
        return Nearest(laid, laid.Lines[to], laid.Across[caret]);
    }

    static int Nearest(Laid laid, Line line, double x)
    {
        var best = line.Start;
        for (var i = line.Start; i <= line.End; i++)
        {
            if (Math.Abs(laid.Across[i] - x) < Math.Abs(laid.Across[best] - x))
                best = i;
            if (laid.Across[i] > x)
                break;
        }
        return best;
    }

    /// <summary>The place nearest a point, from the element's top left.</summary>
    public int PlaceAt(double x, double y)
    {
        var laid = Layout();
        var inner = Inner;
        var line = Multiline ? Math.Clamp((int)Math.Floor((y - (inner.Y - Bounds.Y) + ScrollY) / laid.Height), 0, laid.Lines.Count - 1) : 0;
        var across = x - (inner.X - Bounds.X) + ScrollX - AlignShift(laid, laid.Lines[line]);
        return Nearest(laid, laid.Lines[line], across);
    }

    /// <summary>How far a line moves right for its alignment, when it fits.</summary>
    double AlignShift(Laid laid, Line line)
    {
        if (Align == TextAlign.Left || Multiline)
            return 0;
        var spare = Inner.Width - laid.Across[line.End];
        return spare <= 0 ? 0 : Align == TextAlign.Right ? spare : spare / 2;
    }

    /// <summary>Scrolls so the caret shows, after it moved or the text changed.</summary>
    void Follow()
    {
        if (Bounds.Width <= 0)
            return;
        var laid = Layout();
        var inner = Inner;
        if (Multiline)
        {
            var most = Math.Max(0, laid.Lines.Count * laid.Height - inner.Height);
            if (follow)
            {
                var top = LineOf(laid, caret) * laid.Height;
                if (top < ScrollY)
                    ScrollY = top;
                else if (top + laid.Height > ScrollY + inner.Height)
                    ScrollY = top + laid.Height - inner.Height;
            }
            ScrollY = Math.Clamp(ScrollY, 0, most);
            ScrollX = 0;
        }
        else
        {
            var full = laid.Across[text.Length];
            var most = Math.Max(0, full - inner.Width + 1);
            if (follow)
            {
                var x = laid.Across[caret];
                if (x < ScrollX)
                    ScrollX = x;
                else if (x > ScrollX + inner.Width - 1)
                    ScrollX = x - inner.Width + 1;
            }
            ScrollX = Math.Clamp(ScrollX, 0, most);
            ScrollY = 0;
        }
        follow = false;
    }

    // -- Drawing ------------------------------------------------------------

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var state = !Enabled() ? null : Focused ? "focus" : Hovered ? "hover" : null;
        var input = look.Part("input", state);
        var radius = input.Radius ?? look.Radius("input");
        var background = input.Background ?? look.Color("input");
        var border = Marked() ? look.Color("error") : input.Border ?? look.Color("strong_border");
        var faded = !Enabled();
        if (Framed)
        {
            painter.Fill(Bounds, Faded(background, faded), radius);
            painter.Stroke(Bounds, Faded(border, faded), look.Border, radius);
            if (Focused)
                painter.Stroke(Bounds, look.Color("focus"), 2, radius);
        }

        Follow();
        var laid = Layout();
        var inner = Inner;
        var textColor = Faded(Framed ? input.Text ?? look.Color("text") : look.Color("text"), faded);
        using var font = painter.Fonts.Font("text", Size, false);
        using var paint = new SKPaint { Color = textColor, IsAntialias = true };
        using var clip = painter.Clip(inner with { X = inner.X - 1, Width = inner.Width + 2 });
        var metrics = font.Metrics;
        double Top(int line) => Multiline ? inner.Y + line * laid.Height - ScrollY : inner.Y + (inner.Height - laid.Height) / 2;
        double Baseline(int line) => Top(line) + (laid.Height - (metrics.Descent - metrics.Ascent)) / 2 - metrics.Ascent;

        if (text.Length == 0 && !string.IsNullOrEmpty(Placeholder))
        {
            using var muted = new SKPaint { Color = Faded(look.Color("muted"), faded), IsAntialias = true };
            var x = Align == TextAlign.Left || Multiline ? inner.X : Align == TextAlign.Right ? inner.Right - font.MeasureText(Placeholder) : inner.X + (inner.Width - font.MeasureText(Placeholder)) / 2;
            painter.Canvas.DrawText(Placeholder, (float)x, (float)Baseline(0), SKTextAlign.Left, font, muted);
        }

        var shown = Shown;
        var (start, end) = Selection;
        for (var i = 0; i < laid.Lines.Count; i++)
        {
            var line = laid.Lines[i];
            var top = Top(i);
            if (top + laid.Height < inner.Y || top > inner.Bottom)
                continue;
            var left = inner.X - ScrollX + AlignShift(laid, line);
            // The selection behind the text, while it has the focus.
            if (Focused && start != end && start <= line.End && end >= line.Start)
            {
                var from = laid.Across[Math.Max(start, line.Start)];
                var to = laid.Across[Math.Min(end, line.End)];
                // A selected line break shows as a little space.
                if (end > line.End && line.End < text.Length)
                    to += Size / 3;
                painter.Fill(new Box(left + from, top, Math.Max(1, to - from), laid.Height), look.Color("selection"));
            }
            if (line.End > line.Start)
                painter.Canvas.DrawText(shown[line.Start..line.End], (float)left, (float)Baseline(i), SKTextAlign.Left, font, paint);
        }

        if (Focused && Enabled())
        {
            var line = LineOf(laid, caret);
            var x = inner.X - ScrollX + AlignShift(laid, laid.Lines[line]) + laid.Across[caret];
            painter.Fill(new Box(Math.Round(x), Top(line), 1, laid.Height), textColor);
        }
    }

    static SKColor Faded(SKColor color, bool faded) => faded ? color.WithAlpha((byte)(color.Alpha * 0.55)) : color;
}
