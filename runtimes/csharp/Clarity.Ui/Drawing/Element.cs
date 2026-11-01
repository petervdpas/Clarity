// What a window is made of: elements in a tree, each with its place, which
// draw themselves and take what the person does. A part (G7c) is an element;
// so are the plate and the line of text everything is drawn with.

namespace Clarity.Ui.Drawing;

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
    }

    protected void RemoveChild(Element child)
    {
        if (children.Remove(child))
            child.Parent = null;
    }

    /// <summary>The screen it is on; none before it is shown.</summary>
    public Screen? Screen => Owner ?? Parent?.Screen;

    /// <summary>Whether it takes the focus: pressed, or reached with Tab.</summary>
    public virtual bool Focusable => false;

    public bool Hovered { get; internal set; }
    public bool Pressed { get; internal set; }
    public bool Focused { get; internal set; }

    /// <summary>Asks the screen to draw again.</summary>
    public void Invalidate() => Screen?.Invalidate();

    /// <summary>Takes its place; a container places its children in it.</summary>
    public virtual void Arrange(Box box) => Bounds = box;

    public virtual void Draw(Painter painter)
    {
        foreach (var child in children)
            child.Draw(painter);
    }

    /// <summary>Pressed and released with the main button.</summary>
    public virtual void OnClick() { }

    /// <summary>A key, while it has the focus; true when it took the key.</summary>
    public virtual bool OnKey(KeyInput key) => false;

    /// <summary>Text typed while it has the focus.</summary>
    public virtual void OnText(string text) { }

    /// <summary>The deepest element at a point: the last drawn first.</summary>
    internal Element? At(double x, double y)
    {
        if (!Bounds.Contains(x, y))
            return null;
        for (var i = children.Count - 1; i >= 0; i--)
            if (children[i].At(x, y) is { } inside)
                return inside;
        return this;
    }

    internal IEnumerable<Element> InOrder()
    {
        yield return this;
        foreach (var child in children)
            foreach (var inner in child.InOrder())
                yield return inner;
    }
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
        var placed = GridLayout.Place(places, Columns, box.Width, box.Height, gap);
        Content = placed.Content;
        for (var i = 0; i < Children.Count; i++)
            Children[i].Arrange(placed.Boxes[i].Offset(box.X, box.Y));
    }

    public override void Draw(Painter painter)
    {
        using (painter.Clip(Bounds))
            base.Draw(painter);
    }
}
