// A scroll box: a grid of its parts as wide as it spans, in a place of its
// own that scrolls when the parts go further down than its rows.

namespace Clarity.Ui;

[PartKind("scroll")]
public sealed class ScrollPart : ContainerPart
{
    public ScrollView View { get; private set; } = null!;

    public GridPanel Inner { get; private set; } = null!;

    protected internal override void Made()
    {
        base.Made();
        Inner = Grid(Spec.Children, Columns);
        View = new ScrollView(Inner);
        AddChild(View);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        View.Arrange(box);
    }
}
