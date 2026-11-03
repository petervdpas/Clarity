// What a window is made of: elements in a tree, each with its place, which
// draw themselves and take what the person does. A part (G7c) is an element;
// so are the plate and the line of text everything is drawn with.

namespace Clarity.Ui;

public abstract class Element
{
    readonly List<Element> children = [];
    internal Screen? Owner;

    /// <summary>Where it is on the screen, as its parent placed it.</summary>
    public Box Bounds { get; protected set; }

    public Element? Parent { get; private set; }

    public IReadOnlyList<Element> Children => children;

    protected void AddChild(Element child)
    {
        child.Parent = this;
        children.Add(child);
        if (Attached)
            child.Attach();
    }

    protected void RemoveChild(Element child)
    {
        if (!children.Remove(child))
            return;
        child.Detach();
        child.Parent = null;
    }

    /// <summary>Whether it is on a screen: its tree is the screen's, or an overlay's on it.</summary>
    public bool Attached { get; private set; }

    /// <summary>It joined a screen (its tree was shown, or it was added to one shown): it may start what runs while it is there.</summary>
    protected virtual void OnAttached() { }

    /// <summary>It left its screen (removed, its overlay closed, its window gone): it stops what it started.</summary>
    protected virtual void OnDetached() { }

    /// <summary>It and everything in it join the screen, each told once, a parent first.</summary>
    internal void Attach()
    {
        if (!Attached)
        {
            Attached = true;
            Guard(OnAttached);
        }
        foreach (var child in children.ToList())
            child.Attach();
    }

    /// <summary>It and everything in it leave the screen, each told once, its children first.</summary>
    internal void Detach()
    {
        foreach (var child in children.ToList())
            child.Detach();
        if (!Attached)
            return;
        Attached = false;
        Guard(OnDetached);
    }

    /// <summary>One element failing as it joins or leaves does not stop the others (fail-safe); it says why.</summary>
    void Guard(Action hook)
    {
        try
        {
            hook();
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{GetType().Name} failed as it joined or left its screen: {e.Message}");
        }
    }

    /// <summary>The screen it is on; none before it is shown.</summary>
    public Screen? Screen => Owner ?? Parent?.Screen;

    /// <summary>Whether it takes the focus: pressed, or reached with Tab.</summary>
    public virtual bool Focusable => false;

    /// <summary>Its place in Tab order among those that take the focus (a part's <c>taborder</c>): lower first, none after those with one.</summary>
    public virtual int? TabIndex => null;

    /// <summary>Whether it takes Tab itself (a code editor) instead of Tab moving the focus on.</summary>
    public virtual bool WantsTab => false;

    /// <summary>Whether it shows: one hidden takes no focus, no pointer and draws nothing, and neither does anything in it.</summary>
    public virtual bool IsShown => true;

    /// <summary>Whether the person may use it by its own state; a disabled one takes no pointer, focus or keys, and neither does anything in it.</summary>
    public virtual bool IsEnabled => true;

    /// <summary>Whether the person may use it on its screen: it and every element it is in are enabled (a disabled panel disables its parts).</summary>
    public bool EnabledInTree
    {
        get
        {
            for (Element? e = this; e is not null; e = e.Parent)
                if (!e.IsEnabled)
                    return false;
            return true;
        }
    }

    /// <summary>The room it needs beyond the rows it spans in a grid: a part that holds a grid of its own gives its padding and strip, so the grid inside fits.</summary>
    public virtual double Around => 0;

    /// <summary>Whether it shows on its screen: it and every element it is in show.</summary>
    public bool ShowsInTree
    {
        get
        {
            for (Element? e = this; e is not null; e = e.Parent)
                if (!e.IsShown)
                    return false;
            return true;
        }
    }

    /// <summary>The look of its screen, or Clarity's light one before it is on a screen.</summary>
    public ThemeLook LookNow => Screen?.Look ?? ThemeLook.Clarity(dark: false);

    /// <summary>The fonts of its screen, or those of Clarity's light look before it is on a screen.</summary>
    public Fonts FontsNow => Screen?.Fonts ?? lightFonts.Value;

    static readonly Lazy<Fonts> lightFonts = new(() => new Fonts(ThemeLook.Clarity(dark: false)));

    public bool Hovered { get; internal set; }
    public bool Pressed { get; internal set; }
    public bool Focused { get; internal set; }

    /// <summary>Asks the screen to draw again.</summary>
    public void Invalidate() => Screen?.Invalidate();

    /// <summary>Takes its place; a container places its children in it.</summary>
    public virtual void Arrange(Box box) => Bounds = box;

    /// <summary>Draws itself; an element draws the children that show, each fail-safe.</summary>
    public virtual void Draw(Painter painter)
    {
        foreach (var child in children)
            if (child.IsShown)
                child.DrawSafely(painter);
    }

    /// <summary>Why its drawing, placing or input failed the last time; none while it works.</summary>
    public string? Failed { get; private set; }

    /// <summary>
    /// Draws it, fail-safe: a drawing that fails shows why in its own place,
    /// in the theme's error color, and the rest of the window draws on (as
    /// Avalonia's FailSafe did for every view).
    /// </summary>
    public void DrawSafely(Painter painter)
    {
        // One that could not take its place says why there instead of drawing half placed.
        if (placingFailed)
        {
            DrawFailure(painter);
            return;
        }
        var saved = painter.Canvas.SaveCount;
        try
        {
            Draw(Styled(painter));
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            painter.Canvas.RestoreToCount(saved);
            Fail("drawing", e);
            DrawFailure(painter);
        }
    }

    /// <summary>The painter it draws with: the window's, or one whose look its style changes (a part's <c>style</c>); it may draw what its style puts under it here.</summary>
    protected virtual Painter Styled(Painter painter) => painter;

    /// <summary>Takes its place, fail-safe: one that fails to place what it holds keeps the box and says why.</summary>
    public void ArrangeSafely(Box box)
    {
        try
        {
            Arrange(box);
            placingFailed = false;
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Bounds = box;
            placingFailed = true;
            Fail("placing", e);
        }
    }

    /// <summary>Whether it failed to take its place the last time it was placed.</summary>
    bool placingFailed;

    /// <summary>Runs what the person did on it, fail-safe: a part that fails to take a press or a key says why in its place, and the window keeps working.</summary>
    public T Safely<T>(Func<T> input, T otherwise)
    {
        try
        {
            return input();
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Fail("input", e);
            Invalidate();
            return otherwise;
        }
    }

    /// <summary>As <see cref="Safely{T}(Func{T}, T)"/> for input that answers nothing.</summary>
    public void Safely(Action input) => Safely(() => { input(); return true; }, false);

    void Fail(string doing, Exception e)
    {
        var why = e.InnerException?.Message ?? e.Message;
        if (Failed != why)
            Console.Error.WriteLine($"{GetType().Name} failed in {doing}: {why}");
        Failed = why;
    }

    /// <summary>Why it failed, in its place: a red outline and the reason in small text.</summary>
    void DrawFailure(Painter painter)
    {
        try
        {
            var look = painter.Look;
            using (painter.Clip(Bounds))
            {
                painter.Stroke(Bounds, look.Color("error"), 1, look.Radius("input"));
                painter.Text($"{GetType().Name}: {Failed}", Bounds.Inset(6), look.Color("error"), look.Size("small"));
            }
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{GetType().Name} could not say why it failed: {e.Message}");
        }
    }

    /// <summary>
    /// The items of its menu for a right press at a point on the screen (a
    /// part's <c>menu</c>, a text box's Cut, Copy, Paste and Select all);
    /// none to leave it to the element it is in. The screen opens them as a
    /// menu at the pointer, and an item runs once the menu closed.
    /// </summary>
    public virtual IReadOnlyList<MenuEntry>? ContextMenuAt(double x, double y) => null;

    /// <summary>Pressed and released with the main button.</summary>
    public virtual void OnClick() { }

    /// <summary>The main button went down on it; it holds the pointer until the button goes up.</summary>
    public virtual void OnPointerPressed(PointerInput at) { }

    /// <summary>The pointer moved over it, or anywhere while it holds the pointer (<see cref="Pressed"/>).</summary>
    public virtual void OnPointerMoved(PointerInput at) { }

    /// <summary>The main button went up after it was pressed on it, wherever the pointer is.</summary>
    public virtual void OnPointerReleased(PointerInput at) { }

    /// <summary>
    /// The pointer it held was taken away before the button went up (a touch
    /// the browser took over, the window lost the focus): what the press
    /// started ends without its release, as Avalonia's lost capture did.
    /// </summary>
    public virtual void OnPointerCancelled() { }

    /// <summary>The wheel turned over it: true when it took the turn (it scrolls); else its parent may.</summary>
    public virtual bool OnWheel(double dx, double dy) => false;

    /// <summary>The wheel turned over it with keys <paramref name="held"/> (Ctrl+wheel zooms a canvas); without its own use of them, as <see cref="OnWheel(double, double)"/>.</summary>
    public virtual bool OnWheel(double dx, double dy, Modifiers held) => OnWheel(dx, dy);

    /// <summary>It got or lost the focus.</summary>
    public virtual void OnFocusChanged(bool focused) { }

    /// <summary>A key, while it has the focus; true when it took the key.</summary>
    public virtual bool OnKey(KeyInput key) => false;

    /// <summary>A key let go, while it has the focus (an engine's held keys).</summary>
    public virtual void OnKeyUp(KeyInput key) { }

    /// <summary>Text typed while it has the focus.</summary>
    public virtual void OnText(string text) { }

    /// <summary>The time it asked for came (Screen.Wake): a trend moves on, a tooltip shows.</summary>
    public virtual void OnWake() { }

    /// <summary>What it says about the place at a point on the screen when the pointer rests there (a button's name and key); none for nothing.</summary>
    public virtual string? TooltipAt(double x, double y) => null;

    /// <summary>The pointer's shape over a point on the screen (a link's hand, a text box's I-beam); none to leave it to the element it is in.</summary>
    public virtual Cursor? CursorAt(double x, double y) => null;

    /// <summary>The deepest element at a point: the last drawn first. A part may say a point within its place is not its own.</summary>
    internal Element? At(double x, double y)
    {
        if (!IsShown || !Bounds.Contains(x, y))
            return null;
        for (var i = children.Count - 1; i >= 0; i--)
            if (children[i].At(x, y) is { } inside)
                return inside;
        return this;
    }

    /// <summary>It and every element in it, in the order they are drawn: a parent before its children.</summary>
    public IEnumerable<Element> InOrder()
    {
        yield return this;
        foreach (var child in children)
            foreach (var inner in child.InOrder())
                yield return inner;
    }
}

/// <summary>Finding elements in a tree.</summary>
public static class Elements
{
    /// <summary>The first element of type <typeparamref name="T"/> in <paramref name="root"/> (itself included) that <paramref name="where"/> holds for; none when there is none.</summary>
    public static T? Find<T>(this Element root, Func<T, bool>? where = null) where T : Element =>
        root.InOrder().OfType<T>().FirstOrDefault(e => where?.Invoke(e) ?? true);
}

/// <summary>A filled place in a color role of the theme, with the radius of a radius role.</summary>
public sealed class Plate(string color, string? radius = null) : Element
{
    public override void Draw(Painter painter) =>
        painter.Fill(Bounds, painter.Look.Color(color), radius is null ? 0 : painter.Look.Radius(radius));
}

/// <summary>A line of text, in the theme's text color and size unless told otherwise.</summary>
public sealed class TextLine(string text) : Element
{
    string text = text;

    public string Text
    {
        get => text;
        set
        {
            if (text == value)
                return;
            text = value;
            Invalidate();
        }
    }

    public string Color { get; init; } = "text";
    public string FontRole { get; init; } = "text";
    public string SizeRole { get; init; } = "text";
    public bool Bold { get; init; }
    public TextAlign Align { get; init; }

    public override void Draw(Painter painter) =>
        painter.Text(text, Bounds, painter.Look.Color(Color), painter.Look.Size(SizeRole), FontRole, Bold, Align);
}

/// <summary>Parts at their places in a grid (conformance/grid-layout.json).</summary>
public sealed class GridPanel(int columns, double gap = GridLayout.Gap) : Element
{
    readonly List<GridItem> places = [];

    public int Columns { get; } = columns;

    /// <summary>How high its content is; more than its place when it needs to scroll.</summary>
    public double Content { get; private set; }

    public void Add(Element part, GridItem place)
    {
        AddChild(part);
        places.Add(place);
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        // Each part says the room it needs around its rows now, as its look may have changed.
        var asked = places.Select((p, i) => p with { Around = Math.Max(p.Around, Children[i].Around) }).ToList();
        var placed = GridLayout.Place(asked, Columns, box.Width, box.Height, gap);
        Content = placed.Content;
        for (var i = 0; i < Children.Count; i++)
            Children[i].ArrangeSafely(placed.Boxes[i].Offset(box.X, box.Y));
    }

    public override void Draw(Painter painter)
    {
        using (painter.Clip(Bounds))
            base.Draw(painter);
    }
}
