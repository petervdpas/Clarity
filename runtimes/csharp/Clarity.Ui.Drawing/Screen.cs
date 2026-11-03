// A window's screen: its elements, drawn in the theme on whatever the host
// gives (the desktop's window, a browser page's canvas, a picture in a
// test), and what the person does sent to the element it is for. The host
// draws again when the screen says it changed (Invalidated), and on every
// frame while an element animates (Animating), and at the time an element
// asked to wake (NextWake, then WakeDue). Every host sends the pointer, the
// wheel, keys going down and up and typed text through the same methods here.

using SkiaSharp;

namespace Clarity.Ui;

public sealed class Screen : IDisposable
{
    Element? hovered;
    Element? pressed;

    public Screen(Element root, ThemeLook look)
    {
        Root = root;
        root.Owner = this;
        Look = look;
        Fonts = new Fonts(look);
        root.Attach();
    }

    /// <summary>The time now, as the screen's clock tells it; a test sets its own.</summary>
    public Func<DateTime> Now { get; set; } = Clarity.Runtime.Clock.Now;

    public Element Root { get; }
    public ThemeLook Look { get; private set; }
    public Fonts Fonts { get; private set; }

    /// <summary>Its size in its own units, and how many pixels one unit is.</summary>
    public double Width { get; private set; }
    public double Height { get; private set; }
    public float Scale { get; private set; } = 1;

    public Element? Focused { get; private set; }

    /// <summary>Something changed: the host draws again.</summary>
    public event Action? Invalidated;

    public void Invalidate() => Invalidated?.Invoke();

    public void Resize(double width, double height, float scale)
    {
        Width = width;
        Height = height;
        Scale = scale <= 0 ? 1 : scale;
        Root.ArrangeSafely(new Box(0, 0, width, height));
        foreach (var overlay in overlays.Where(o => o.Covers))
            overlay.Place = Whole;
        Invalidate();
    }

    /// <summary>The whole screen, where an overlay that covers it lies.</summary>
    public Box Whole => new(0, 0, Width, Height);

    /// <summary>Another theme: the light or dark of the system changed, or the person chose one.</summary>
    public void Show(ThemeLook look)
    {
        Look = look;
        Fonts = new Fonts(look);
        Invalidate();
    }

    readonly HashSet<Element> animating = [];

    /// <summary>How many elements animate now (a busy indicator turning): while above zero the host draws every frame.</summary>
    public int Animating => animating.Count;

    /// <summary>
    /// <paramref name="element"/> animates (or no longer does): it asks for
    /// the next frame. One that is no longer shown on this screen stops by itself.
    /// </summary>
    public void Animate(Element element, bool on = true)
    {
        if (on)
            animating.Add(element);
        else
            animating.Remove(element);
    }

    readonly Dictionary<Element, DateTime> wakes = [];

    /// <summary>
    /// <paramref name="element"/> wakes at <paramref name="at"/> (its
    /// OnWake runs then), once: a trend that moves on every second instead of
    /// every frame. Asking again moves its time; one that left the screen is not woken.
    /// </summary>
    public void Wake(Element element, DateTime at)
    {
        wakes[element] = at;
        WakesChanged?.Invoke();
    }

    /// <summary>When the next element or tooltip wakes; none when none waits. The host calls <see cref="WakeDue"/> then.</summary>
    public DateTime? NextWake
    {
        get
        {
            DateTime? next = tipDue;
            foreach (var at in wakes.Values)
                if (next is null || at < next)
                    next = at;
            return next;
        }
    }

    /// <summary>The time of <see cref="NextWake"/> changed: a host that waits for it waits anew.</summary>
    public event Action? WakesChanged;

    /// <summary>Wakes every element whose time came by <paramref name="now"/>, and shows the tooltip whose pause is over; drawing does it too.</summary>
    public void WakeDue(DateTime now)
    {
        foreach (var (element, at) in wakes.ToList())
        {
            if (at > now)
                continue;
            wakes.Remove(element);
            if (element.Attached && element.Screen == this)
                element.Safely(element.OnWake);
        }
        if (tipDue is { } due && due <= now && tipWaiting is { } text)
        {
            tipDue = null;
            // Only for what is under the pointer now: another window may show there since.
            if (TipOf(At(tipPoint.X, tipPoint.Y), tipPoint.X, tipPoint.Y) != text)
            {
                tipWaiting = null;
                return;
            }
            Tip = new Tooltip(text, TipPlace(text));
            Invalidate();
        }
    }

    public void Draw(SKCanvas canvas)
    {
        WakeDue(Now());
        animating.RemoveWhere(e => e.Screen != this || !e.ShowsInTree);
        canvas.Save();
        canvas.Scale(Scale);
        canvas.Clear(Look.Color("background"));
        Root.ArrangeSafely(new Box(0, 0, Width, Height));
        var painter = new Painter(canvas, Look, Fonts);
        Root.DrawSafely(painter);
        foreach (var overlay in overlays.ToList())
        {
            overlay.Content.ArrangeSafely(overlay.Place);
            if (overlay.Content.IsShown)
                overlay.Content.DrawSafely(painter);
        }
        if (Tip is { } tip)
            DrawTip(painter, tip);
        canvas.Restore();
    }

    // -- Tooltips ------------------------------------------------------------

    /// <summary>How long the pointer rests before a tooltip shows.</summary>
    public static readonly TimeSpan TipPause = TimeSpan.FromMilliseconds(600);

    const double TipPad = 6;

    string? tipWaiting;
    DateTime? tipDue;
    (double X, double Y) tipPoint;

    /// <summary>A tooltip as it shows: its text and its place.</summary>
    public sealed record Tooltip(string Text, Box Place);

    /// <summary>The tooltip shown now; none while none shows.</summary>
    public Tooltip? Tip { get; private set; }

    /// <summary>What the element at a point, or one it is in, says about that place.</summary>
    static string? TipOf(Element? at, double x, double y)
    {
        for (var e = at; e is not null; e = e.Parent)
            if (e.TooltipAt(x, y) is { Length: > 0 } text)
                return text;
        return null;
    }

    /// <summary>The pointer rests on <paramref name="text"/> from now (none: on nothing that says something).</summary>
    void Rest(string? text, double x, double y)
    {
        if (text == tipWaiting)
            return;
        HideTip();
        tipWaiting = text;
        tipPoint = (x, y);
        if (text is not null)
        {
            tipDue = Now() + TipPause;
            WakesChanged?.Invoke();
        }
    }

    /// <summary>The tooltip goes, and none waits: the pointer left, pressed, the wheel turned or a key was pressed.</summary>
    void HideTip()
    {
        tipWaiting = null;
        tipDue = null;
        if (Tip is null)
            return;
        Tip = null;
        Invalidate();
    }

    /// <summary>Where a tooltip shows: under the pointer, inside the screen.</summary>
    Box TipPlace(string text)
    {
        var size = Look.Size("small");
        var width = Fonts.Measure(text, "text", size, false) + 2 * TipPad;
        var height = size + 2 * TipPad + 2;
        var x = Math.Max(0, Math.Min(tipPoint.X, Width - width));
        var y = tipPoint.Y + 20 + height > Height ? tipPoint.Y - 8 - height : tipPoint.Y + 20;
        return new Box(x, Math.Max(0, y), width, height);
    }

    /// <summary>A tooltip in Clarity's look: the tooltip part's colors, else a panel with a strong border.</summary>
    void DrawTip(Painter painter, Tooltip tip)
    {
        var look = Look.Part("tooltip", null);
        var radius = look.Radius ?? Look.Radius("input");
        painter.Fill(tip.Place, look.Background ?? Look.Color("panel"), radius);
        painter.Stroke(tip.Place, look.Border ?? Look.Color("strong_border"), Look.Border, radius);
        painter.Text(tip.Text, tip.Place.Inset(TipPad), look.Text ?? Look.Color("text"), Look.Size("small"));
    }

    readonly List<Overlay> overlays = [];

    /// <summary>What lies over the window, the top one last.</summary>
    public IReadOnlyList<Overlay> Overlays => overlays;

    /// <summary>Copying and pasting: the system's, once the host gives it.</summary>
    public IClipboard Clipboard { get; set; } = new OwnClipboard();

    public void Open(Overlay overlay)
    {
        HideTip();
        if (overlay.Covers)
            overlay.Place = Whole;
        overlay.Content.Owner = this;
        overlay.Content.ArrangeSafely(overlay.Place);
        overlays.Add(overlay);
        overlay.Content.Attach();
        Invalidate();
    }

    public void Close(Overlay overlay)
    {
        if (!overlays.Remove(overlay))
            return;
        overlay.Content.Detach();
        overlay.Content.Owner = null;
        if (Focused is { } focused && Within(focused, overlay.Content))
            Focus(null);
        Invalidate();
        overlay.Closed?.Invoke();
    }

    static bool Within(Element element, Element root)
    {
        for (Element? e = element; e is not null; e = e.Parent)
            if (e == root)
                return true;
        return false;
    }

    /// <summary>The deepest element at a point: in the top overlay that has it, else in the window.</summary>
    public Element? At(double x, double y)
    {
        for (var i = overlays.Count - 1; i >= 0; i--)
        {
            if (overlays[i].Content.At(x, y) is { } inside)
                return inside;
            if (overlays[i].Modal)
                return null;
        }
        return Root.At(x, y);
    }

    static PointerInput Input(Element e, double x, double y) => new(x, y, x - e.Bounds.X, y - e.Bounds.Y);

    // -- Pointer shape ---------------------------------------------------------

    Cursor cursor;

    /// <summary>The shape the pointer shows now; each host shows it as its system's.</summary>
    public Cursor Cursor => cursor;

    /// <summary>The pointer's shape changed: the host shows <see cref="Cursor"/>.</summary>
    public event Action? CursorChanged;

    /// <summary>The shape over a point: of what holds the pointer, else of the deepest element there or one it is in; a disabled one shows the plain pointer.</summary>
    Cursor ShapeAt(Element? at, double x, double y)
    {
        var from = pressed ?? at;
        if (from is null || !from.EnabledInTree)
            return Cursor.Default;
        for (var e = from; e is not null; e = e.Parent)
            if (e.Safely(() => e.CursorAt(x, y), null) is { } shape)
                return shape;
        return Cursor.Default;
    }

    void Shape(Cursor shape)
    {
        if (shape == cursor)
            return;
        cursor = shape;
        CursorChanged?.Invoke();
    }

    public void PointerMoved(double x, double y)
    {
        if (pressed is { } holding)
            holding.Safely(() => holding.OnPointerMoved(Input(holding, x, y)));
        var at = At(x, y);
        if (at is not null && at != pressed && at.EnabledInTree)
            at.Safely(() => at.OnPointerMoved(Input(at, x, y)));
        Shape(ShapeAt(at, x, y));
        Rest(pressed is null ? TipOf(at, x, y) : null, x, y);
        if (at == hovered)
            return;
        if (hovered is not null)
            hovered.Hovered = false;
        hovered = at;
        if (at is not null)
            at.Hovered = true;
        Invalidate();
    }

    public void PointerLeft()
    {
        HideTip();
        if (pressed is null)
            Shape(Cursor.Default);
        if (hovered is null)
            return;
        hovered.Hovered = false;
        hovered = null;
        Invalidate();
    }

    public void PointerPressed(double x, double y, PointerButton button, int clicks = 1)
    {
        HideTip();
        // A press outside the overlays closes them, top first, and goes no further.
        if (overlays.Count > 0 && overlays[^1] is { Modal: false } top && !top.Content.Bounds.Contains(x, y) && !top.Place.Contains(x, y) && top.Anchor?.Invoke().Contains(x, y) != true)
        {
            Close(top);
            return;
        }
        PointerMoved(x, y);
        // A press starts no new pause: the tip of what was pressed shows again only once the pointer went elsewhere.
        tipDue = null;
        // What is disabled (or in a disabled part) takes no press, no focus and no menu.
        if (At(x, y) is not { } at || !at.EnabledInTree)
            return;
        if (button == PointerButton.Right)
        {
            ContextPressed(at, x, y);
            return;
        }
        if (button != PointerButton.Left)
            return;
        // Focus by the pointer: what was pressed stays where it is (no scrolling into view), so the release lands on it.
        Pointing = true;
        try
        {
            Focus(Taker(at));
        }
        finally
        {
            Pointing = false;
        }
        pressed = at;
        at.Pressed = true;
        at.Safely(() => at.OnPointerPressed(Input(at, x, y) with { Clicks = clicks }));
        Invalidate();
    }

    /// <summary>Whether the focus moves because of a press of the pointer (not Tab or code): a scrolling window then leaves its scroll alone.</summary>
    public bool Pointing { get; private set; }

    readonly FrameMenus contextMenus = new();

    /// <summary>The menu a right press opened; none while none is open.</summary>
    public FrameMenus ContextMenus => contextMenus;

    /// <summary>
    /// A right press: what takes the focus there takes it (a text box keeps
    /// its selection for Copy), then the element there, or one it is in,
    /// gives its menu, which opens at the pointer (a part's <c>menu</c>).
    /// </summary>
    void ContextPressed(Element at, double x, double y)
    {
        if (Taker(at) is { } taker)
            Focus(taker);
        for (var e = at; e is not null; e = e.Parent)
            if (e.Safely(() => e.ContextMenuAt(x, y), null) is { Count: > 0 } entries)
            {
                contextMenus.Open(this, e, entries, new Box(x, y - 1, 0, 0));
                return;
            }
    }

    public void PointerReleased(double x, double y, PointerButton button)
    {
        if (button != PointerButton.Left || pressed is not { } was)
            return;
        pressed = null;
        was.Pressed = false;
        was.Safely(() => was.OnPointerReleased(Input(was, x, y)));
        Invalidate();
        if (At(x, y) == was)
            was.Safely(was.OnClick);
        Shape(ShapeAt(At(x, y), x, y));
    }

    /// <summary>
    /// The pointer was taken away mid-press: a touch the browser took over,
    /// or the window lost the focus (Alt+Tab during a drag). What holds it
    /// lets go without a release and without a click, so a drag in progress
    /// is dropped, not committed.
    /// </summary>
    public void PointerCancelled()
    {
        if (pressed is not { } was)
            return;
        pressed = null;
        was.Pressed = false;
        was.Safely(was.OnPointerCancelled);
        Invalidate();
        Shape(Cursor.Default);
    }

    /// <summary>
    /// The wheel turned at a point: the deepest element there that scrolls
    /// takes it. In the screen's units: a positive <paramref name="dy"/> shows
    /// what is further down (the content moves up), a positive
    /// <paramref name="dx"/> what is further right, as a browser's wheel event
    /// gives it; every host turns its wheel into this, with the keys
    /// <paramref name="held"/> (Ctrl+wheel zooms a canvas).
    /// </summary>
    public void Wheel(double x, double y, double dx, double dy, Modifiers held = Modifiers.None)
    {
        HideTip();
        // Shift turns a wheel that only goes up and down sideways, as every system's scrolling does.
        if (held.HasFlag(Modifiers.Shift) && dx == 0)
            (dx, dy) = (dy, 0);
        for (var e = At(x, y); e is not null; e = e.Parent)
            if (e.Safely(() => e.OnWheel(dx, dy, held), false))
            {
                Invalidate();
                return;
            }
    }

    static Element? Taker(Element at)
    {
        for (Element? e = at; e is not null; e = e.Parent)
            if (e.Focusable)
                return e.EnabledInTree ? e : null;
        return null;
    }

    /// <summary>The focus moved to the element given (none: nothing has it): a scrolling window shows it.</summary>
    public event Action<Element?>? FocusChanged;

    public void Focus(Element? element)
    {
        if (element == Focused)
            return;
        var was = Focused;
        if (was is not null)
            was.Focused = false;
        Focused = element;
        if (element is not null)
            element.Focused = true;
        was?.Safely(() => was.OnFocusChanged(false));
        element?.Safely(() => element.OnFocusChanged(true));
        FocusChanged?.Invoke(element);
        Invalidate();
    }

    /// <summary>
    /// The window's own keys, asked once the part with the focus did not
    /// take the key (a text box keeps Ctrl+C, Ctrl+V and its arrows, as
    /// Avalonia's controls did before the window's key bindings): a
    /// button's shortcut, Enter for the default button, Escape for the
    /// cancel button, Ctrl+Z. True when it took the key. Not asked while a
    /// modal overlay (a dialog) is on top: its keys are its own. A character
    /// typed after a key nothing took is asked too, as a key with
    /// <see cref="KeyInput.Typed"/> (a <c>*</c> shortcut answers Shift+8 on
    /// any keyboard layout).
    /// </summary>
    public Func<KeyInput, bool>? Keys { get; set; }

    /// <summary>Whether something took the last key pressed; the character it types is then no shortcut.</summary>
    bool keyTaken;

    bool ModalOnTop => overlays.Count > 0 && overlays[^1].Modal;

    /// <summary>A key pressed: true when something on the screen took it, so a browser host leaves the rest (F5, Ctrl+R) to the browser.</summary>
    public bool KeyPressed(KeyInput key)
    {
        HideTip();
        keyTaken = true;
        // A dialog on top (a modal overlay) hears its keys before the part with the focus.
        if (overlays.Count > 0 && overlays[^1] is { Modal: true, Content: IDialogKeys dialog } && dialog.KeyPressed(key))
        {
            Invalidate();
            return true;
        }
        var focused = Focused is { } f && f.EnabledInTree ? f : null;
        // Escape closes the overlay on top, unless the part with the focus takes it (a list that closes itself).
        if (key.Key == Key.Escape && overlays.Count > 0 && !overlays[^1].Modal)
        {
            if (focused is null || !focused.Safely(() => focused.OnKey(key), false))
                Close(overlays[^1]);
            return true;
        }
        if (key.Key == Key.Tab && !key.Control && !key.Modifiers.HasFlag(Modifiers.Alt) && (focused is null || !focused.WantsTab))
        {
            MoveFocus(back: key.Shift);
            return TabOrder().Count > 0;
        }
        if (focused is not null && focused.Safely(() => focused.OnKey(key), false))
        {
            Invalidate();
            return true;
        }
        if (Keys is { } keys && !ModalOnTop && keys(key))
        {
            Invalidate();
            return true;
        }
        keyTaken = false;
        return false;
    }

    /// <summary>
    /// The elements Tab reaches, in Tab order: those that take the focus,
    /// show and are enabled, in the order they are drawn (the window's frame
    /// before its grid), except that a part's <c>taborder</c> sorts it
    /// among its own: lower first, those without one after, in tree order.
    /// </summary>
    public List<Element> TabOrder()
    {
        var root = ModalOnTop ? overlays[^1].Content : Root;
        var takers = root.InOrder().Where(e => e.Focusable && e.ShowsInTree && e.EnabledInTree).ToList();
        return [.. takers.Select((e, i) => (e, i)).OrderBy(p => IndexOf(p.e) ?? int.MaxValue).ThenBy(p => p.i).Select(p => p.e)];
    }

    /// <summary>The Tab place of an element: its own, else that of the part it is in (a text box's editor goes by the text box's <c>taborder</c>).</summary>
    static int? IndexOf(Element element)
    {
        for (Element? e = element; e is not null; e = e.Parent)
            if (e.TabIndex is { } index)
                return index;
        return null;
    }

    /// <summary>The focus goes to the next element in Tab order, or the one before.</summary>
    public void MoveFocus(bool back = false)
    {
        var takers = TabOrder();
        if (takers.Count == 0)
            return;
        var at = Focused is null ? -1 : takers.IndexOf(Focused);
        var next = back ? (at <= 0 ? takers.Count - 1 : at - 1) : (at + 1) % takers.Count;
        Focus(takers[next]);
    }

    /// <summary>The screen is no longer shown (its window closed): its overlays close and every element leaves it, so what they started stops.</summary>
    public void Dispose()
    {
        foreach (var overlay in overlays.ToList())
            Close(overlay);
        wakes.Clear();
        HideTip();
        animating.Clear();
        Root.Detach();
    }

    /// <summary>The window's own keys let go, asked before the part with the focus (Alt for the menu bar's access letters): true when it took the key. Not asked while a modal overlay is on top.</summary>
    public Func<KeyInput, bool>? KeysUp { get; set; }

    /// <summary>A key let go: the window hears it first, then the part with the focus (an engine's held keys). Every host sends it.</summary>
    public void KeyReleased(KeyInput key)
    {
        if (KeysUp is { } keysUp && !(overlays.Count > 0 && overlays[^1].Modal) && keysUp(key))
        {
            Invalidate();
            return;
        }
        if (Focused is { } focused)
            focused.Safely(() => focused.OnKeyUp(key));
    }

    /// <summary>
    /// Text typed: after a key nothing took, a single character is first the
    /// window's (a character shortcut such as <c>*</c>, matched by what the
    /// keyboard typed, not where its key is), else the focused part's.
    /// </summary>
    public void TextTyped(string text)
    {
        var shortcut = !keyTaken && text.Length is 1 or 2 && !char.IsControl(text[0]) && !ModalOnTop && Keys is { } keys
            && keys(new KeyInput(Key.Other, Modifiers.None) { Typed = text });
        keyTaken = false;
        if (shortcut)
        {
            Invalidate();
            return;
        }
        if (Focused is { } focused && focused.EnabledInTree)
            focused.Safely(() => focused.OnText(text));
    }
}
