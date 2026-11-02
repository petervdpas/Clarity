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
        Root.Arrange(new Box(0, 0, width, height));
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
                element.OnWake();
        }
        if (tipDue is { } due && due <= now && tipWaiting is { } text)
        {
            tipDue = null;
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
        Root.Arrange(new Box(0, 0, Width, Height));
        var painter = new Painter(canvas, Look, Fonts);
        Root.Draw(painter);
        foreach (var overlay in overlays.ToList())
        {
            overlay.Content.Arrange(overlay.Place);
            if (overlay.Content.IsShown)
                overlay.Content.Draw(painter);
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
        overlay.Content.Arrange(overlay.Place);
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

    public void PointerMoved(double x, double y)
    {
        if (pressed is { } holding)
            holding.OnPointerMoved(Input(holding, x, y));
        var at = At(x, y);
        if (at is not null && at != pressed)
            at.OnPointerMoved(Input(at, x, y));
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
        if (button != PointerButton.Left || At(x, y) is not { } at)
            return;
        Focus(Taker(at));
        pressed = at;
        at.Pressed = true;
        at.OnPointerPressed(Input(at, x, y) with { Clicks = clicks });
        Invalidate();
    }

    public void PointerReleased(double x, double y, PointerButton button)
    {
        if (button != PointerButton.Left || pressed is not { } was)
            return;
        pressed = null;
        was.Pressed = false;
        was.OnPointerReleased(Input(was, x, y));
        Invalidate();
        if (At(x, y) == was)
            was.OnClick();
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
        for (var e = At(x, y); e is not null; e = e.Parent)
            if (e.OnWheel(dx, dy, held))
            {
                Invalidate();
                return;
            }
    }

    static Element? Taker(Element at)
    {
        for (Element? e = at; e is not null; e = e.Parent)
            if (e.Focusable)
                return e;
        return null;
    }

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
        was?.OnFocusChanged(false);
        element?.OnFocusChanged(true);
        Invalidate();
    }

    /// <summary>
    /// The window's own keys, asked before the part with the focus (a
    /// button's shortcut, Enter for the default button, Escape for the cancel
    /// button, Ctrl+Z): true when it took the key. Not asked while a modal
    /// overlay (a dialog) is on top: its keys are its own.
    /// </summary>
    public Func<KeyInput, bool>? Keys { get; set; }

    public void KeyPressed(KeyInput key)
    {
        HideTip();
        // A dialog on top (a modal overlay) hears its keys before the part with the focus.
        if (overlays.Count > 0 && overlays[^1] is { Modal: true, Content: IDialogKeys dialog } && dialog.KeyPressed(key))
        {
            Invalidate();
            return;
        }
        // Escape closes the overlay on top, unless the part with the focus takes it (a list that closes itself).
        if (key.Key == Key.Escape && overlays.Count > 0 && !overlays[^1].Modal)
        {
            if (Focused is null || !Focused.OnKey(key))
                Close(overlays[^1]);
            return;
        }
        if (Keys is { } keys && !(overlays.Count > 0 && overlays[^1].Modal) && keys(key))
        {
            Invalidate();
            return;
        }
        if (key.Key == Key.Tab && !key.Control && (Focused is null || !Focused.WantsTab))
        {
            var root = overlays.Count > 0 && overlays[^1].Modal ? overlays[^1].Content : Root;
            var takers = root.InOrder().Where(e => e.Focusable && e.ShowsInTree).ToList();
            if (takers.Count == 0)
                return;
            var at = Focused is null ? -1 : takers.IndexOf(Focused);
            var next = key.Shift ? (at <= 0 ? takers.Count - 1 : at - 1) : (at + 1) % takers.Count;
            Focus(takers[next]);
            return;
        }
        Focused?.OnKey(key);
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

    /// <summary>A key let go: the part with the focus hears it (an engine's held keys). Every host sends it.</summary>
    public void KeyReleased(KeyInput key) => Focused?.OnKeyUp(key);

    public void TextTyped(string text) => Focused?.OnText(text);
}
