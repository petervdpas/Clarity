// A window's screen: its elements, drawn in the theme on whatever the host
// gives (the desktop's window, a browser page's canvas, a picture in a
// test), and what the person does sent to the element it is for. The host
// draws again when the screen says it changed (Invalidated), and on every
// frame while an element animates (Animating). Every host sends the pointer,
// the wheel, keys and typed text through the same methods here.

using SkiaSharp;

namespace Clarity.Ui;

public sealed class Screen
{
    Element? hovered;
    Element? pressed;

    public Screen(Element root, ThemeLook look)
    {
        Root = root;
        root.Owner = this;
        Look = look;
        Fonts = new Fonts(look);
    }

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
        Invalidate();
    }

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

    public void Draw(SKCanvas canvas)
    {
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
        canvas.Restore();
    }

    readonly List<Overlay> overlays = [];

    /// <summary>What lies over the window, the top one last.</summary>
    public IReadOnlyList<Overlay> Overlays => overlays;

    /// <summary>Copying and pasting: the system's, once the host gives it.</summary>
    public IClipboard Clipboard { get; set; } = new OwnClipboard();

    public void Open(Overlay overlay)
    {
        overlay.Content.Owner = this;
        overlay.Content.Arrange(overlay.Place);
        overlays.Add(overlay);
        Invalidate();
    }

    public void Close(Overlay overlay)
    {
        if (!overlays.Remove(overlay))
            return;
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
        if (hovered is null)
            return;
        hovered.Hovered = false;
        hovered = null;
        Invalidate();
    }

    public void PointerPressed(double x, double y, PointerButton button, int clicks = 1)
    {
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
    /// gives it; every host turns its wheel into this.
    /// </summary>
    public void Wheel(double x, double y, double dx, double dy)
    {
        for (var e = At(x, y); e is not null; e = e.Parent)
            if (e.OnWheel(dx, dy))
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

    public void TextTyped(string text) => Focused?.OnText(text);
}
