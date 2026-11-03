// An accordion (U1b, conformance/accordions.json): sections that fold, one
// open at a time. Each section's header is a button; the open section's
// parts fill the room under it. The open section, counted from 1 and 0 for
// none, is its part's state's OpenSection, so code reads and sets it. A press
// on a header opens that section, or folds it when it was the open one, and
// runs the part's handler of `on change`.

namespace Clarity.Ui;

[PartKind("accordion")]
public sealed class AccordionPart : ContainerPart
{
    readonly List<(PageSpec Page, GridPanel Grid, ContainerButton Header)> sections = [];
    long openHere = 1;

    /// <summary>How high a section's header is, and the room between headers.</summary>
    public const double SectionHeight = 32;
    const double Between = 2;

    public IReadOnlyList<ContainerButton> Headers => [.. sections.Select(s => s.Header)];

    public IReadOnlyList<GridPanel> Pages => [.. sections.Select(s => s.Grid)];

    /// <summary>The section shown open when <paramref name="open"/> is asked of <paramref name="count"/>: it, when it is there; else none.</summary>
    public static long Shown(long open, int count) => open >= 1 && open <= count ? open : 0;

    /// <summary>The open section after a press on the header of <paramref name="press"/>: it, or none when it was the open one.</summary>
    public static long AfterPress(long open, long press, int count) => Shown(open, count) == press ? 0 : press;

    /// <summary>The section asked to be open, from 1, 0 for none: the part's state's, or its own without a name.</summary>
    public long Open
    {
        get => State?.OpenSection ?? openHere;
        set
        {
            if (State is { } state)
                state.OpenSection = value;
            else
                openHere = value;
            Invalidate();
        }
    }

    protected internal override void Made()
    {
        base.Made();
        foreach (var page in Spec.Pages)
        {
            var number = sections.Count + 1;
            var header = new ContainerButton(() => Shown(page.Title), () => Press(number)) { Usable = () => Enabled, Chevron = false };
            AddChild(header);
            sections.Add((page, Grid(page.Children, Columns), header));
        }
    }

    /// <summary>A press on the header of section <paramref name="press"/>, from 1.</summary>
    public void Press(long press)
    {
        Open = AfterPress(Open, press, sections.Count);
        Fire("change");
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var pad = LookNow.Padding;
        var open = (int)Shown(Open, sections.Count);
        var headers = sections.Count * SectionHeight + Math.Max(0, sections.Count - 1) * Between;
        var room = Math.Max(0, box.Height - headers - 4 * pad);
        var y = box.Y;
        for (var i = 0; i < sections.Count; i++)
        {
            var (_, grid, header) = sections[i];
            header.Chevron = i + 1 == open;
            header.Arrange(new Box(box.X, y, box.Width, SectionHeight));
            y += SectionHeight + Between;
            var on = i + 1 == open;
            Show(grid, on);
            if (on)
            {
                grid.Arrange(new Box(box.X, y + 2 * pad - Between, box.Width, room));
                y += room + 4 * pad;
            }
        }
    }
}
