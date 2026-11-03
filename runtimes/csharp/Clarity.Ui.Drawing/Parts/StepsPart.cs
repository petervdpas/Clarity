// A wizard, the steps part (DESIGN 5.15, conformance/steps.json): one step
// at a time, the titles of all steps above with the current one marked, and
// Clarity's own Back, Next and Finish below. The step it is on, counted from
// 1, is its part's state's Step, so code reads and sets it. Next runs the
// handler of `on next` on the step shown, then moves one step on, unless the
// handler refused (the step stays) or set the step itself (that step wins).
// Back goes one step back and runs nothing. Finish runs `on finish`. Next
// and Finish are the window's default button: Enter presses them.

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("steps")]
public sealed class StepsPart : ContainerPart, IDefaultButton
{
    readonly List<(PageSpec Page, GridPanel Grid)> steps = [];
    long stepHere = 1;

    /// <summary>How high the line of titles and the line of buttons are.</summary>
    public const double TitlesHeight = 28;
    public const double ButtonsHeight = 32;

    public ContainerButton Back { get; }
    public ContainerButton Next { get; }
    public ContainerButton Finish { get; }

    public IReadOnlyList<GridPanel> Pages => [.. steps.Select(s => s.Grid)];

    public StepsPart()
    {
        Back = new ContainerButton(() => Locale.Translate("back", "Back"), GoBack) { Shows = () => Shown(Step, steps.Count) > 1, Usable = () => Enabled };
        Next = new ContainerButton(() => Locale.Translate("next", "Next"), GoNext) { Accent = true, Shows = () => Shown(Step, steps.Count) < steps.Count, Usable = () => Enabled };
        Finish = new ContainerButton(() => Locale.Translate("finish", "Finish"), () => Fire("finish")) { Accent = true, Shows = () => Shown(Step, steps.Count) >= steps.Count, Usable = () => Enabled };
    }

    /// <summary>The step shown when <paramref name="step"/> is asked of <paramref name="count"/>: the nearest there is.</summary>
    public static long Shown(long step, int count) => Math.Clamp(step, 1, Math.Max(count, 1));

    /// <summary>The step it is on, from 1: the part's state's, or its own without a name.</summary>
    public long Step
    {
        get => State?.Step ?? stepHere;
        set
        {
            if (State is { } state)
                state.Step = value;
            else
                stepHere = value;
            Invalidate();
        }
    }

    protected internal override void Made()
    {
        base.Made();
        foreach (var page in Spec.Pages)
            steps.Add((page, Grid(page.Children, Columns)));
        AddChild(Back);
        AddChild(Next);
        AddChild(Finish);
    }

    /// <summary>Next: the handler runs on the step shown, then the wizard moves on unless it refused or chose the step.</summary>
    public void GoNext()
    {
        var before = Shown(Step, steps.Count);
        Step = before;
        if (!Ask("next"))
            return;
        if (Shown(Step, steps.Count) == before && before < steps.Count)
            Step = before + 1;
    }

    /// <summary>Next, or Finish on the last step, is its window's default button: Enter presses it when it shows and can be pressed.</summary>
    public bool TakesEnter => ShowsInTree && Enabled && (Next.IsShown || Finish.IsShown);

    public void PressDefault() => (Next.IsShown ? Next : Finish).Press();

    /// <summary>Back: one step back; nothing runs and what was entered stays.</summary>
    public void GoBack()
    {
        var shown = Shown(Step, steps.Count);
        if (shown > 1)
            Step = shown - 1;
    }

    /// <summary>Runs the handlers of <paramref name="event"/>: false when one refused.</summary>
    bool Ask(string @event)
    {
        foreach (var call in Spec.On(@event))
            if (!Host.Run(Spec.Name, call))
                return false;
        return true;
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var look = LookNow;
        var pad = look.Padding;
        var shown = (int)Shown(Step, steps.Count);
        var top = box.Y + TitlesHeight + 2 * pad;
        var bottom = box.Bottom - ButtonsHeight - 2 * pad;
        for (var i = 0; i < steps.Count; i++)
        {
            var on = i + 1 == shown;
            Show(steps[i].Grid, on);
            if (on)
                steps[i].Grid.Arrange(new Box(box.X, top, box.Width, Math.Max(0, bottom - top)));
        }
        // The buttons on the right, the last one at the edge.
        var right = box.Right;
        foreach (var button in new[] { Finish, Next, Back })
        {
            if (!button.IsShown)
            {
                button.Arrange(new Box(right, box.Bottom - ButtonsHeight, 0, ButtonsHeight));
                continue;
            }
            var width = Math.Max(80, button.WidthFor(Screen?.Fonts, look));
            right -= width;
            button.Arrange(new Box(right, box.Bottom - ButtonsHeight, width, ButtonsHeight));
            right -= 2 * pad;
        }
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        var shown = (int)Shown(Step, steps.Count);
        var x = Bounds.X;
        var size = look.Size("text");
        for (var i = 0; i < steps.Count; i++)
        {
            var title = Shown(steps[i].Page.Title);
            var current = i + 1 == shown;
            var color = look.Color(current ? "accent" : i + 1 < shown ? "text" : "muted");
            var width = painter.Fonts.Measure(title, "text", size, current);
            painter.Text(title, new Box(x, Bounds.Y, Math.Max(0, Math.Min(width + 1, Bounds.Right - x)), TitlesHeight), color, size, "text", current);
            x += width + 6 * look.Padding;
        }
        painter.Fill(new Box(Bounds.X, Bounds.Y + TitlesHeight, Bounds.Width, look.Border), look.Color("strong_border"));
        base.Draw(painter);
    }
}
