// A window's screen: its elements, drawn in the theme on whatever the host
// gives (the desktop's window, a browser page's canvas, a picture in a
// test), and what the person does sent to the element it is for. The host
// draws again when the screen says it changed (Invalidated).

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

    public void Draw(SKCanvas canvas)
    {
        canvas.Save();
        canvas.Scale(Scale);
        canvas.Clear(Look.Color("background"));
        Root.Arrange(new Box(0, 0, Width, Height));
        Root.Draw(new Painter(canvas, Look, Fonts));
        canvas.Restore();
    }

    public Element? At(double x, double y) => Root.At(x, y);

    public void PointerMoved(double x, double y)
    {
        var at = At(x, y);
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

    public void PointerPressed(double x, double y, PointerButton button)
    {
        PointerMoved(x, y);
        if (button != PointerButton.Left || At(x, y) is not { } at)
            return;
        if (Taker(at) is { } taker)
            Focus(taker);
        pressed = at;
        at.Pressed = true;
        Invalidate();
    }

    public void PointerReleased(double x, double y, PointerButton button)
    {
        if (button != PointerButton.Left || pressed is not { } was)
            return;
        pressed = null;
        was.Pressed = false;
        Invalidate();
        if (At(x, y) == was)
            was.OnClick();
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
        if (Focused is not null)
            Focused.Focused = false;
        Focused = element;
        if (element is not null)
            element.Focused = true;
        Invalidate();
    }

    public void KeyPressed(KeyInput key)
    {
        if (key.Key == Key.Tab && !key.Control)
        {
            var takers = Root.InOrder().Where(e => e.Focusable).ToList();
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
