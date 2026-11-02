// A trend (DESIGN 5.26), the same as on the web: the last values of one
// number, taken every so many seconds while its window is open (hidden
// too, so its history has no gap), drawn as Clarity's line chart with the
// newest at the right. It takes a value when it is first drawn, and asks the
// screen to wake it when the next is due (Screen.Wake), so it draws once a
// second rather than every frame.

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("trend")]
public sealed class TrendPart : Part
{
    /// <summary>How many values a trend keeps without <c>points</c>.</summary>
    public const int KeptPoints = 60;

    readonly List<double> history = [];
    DateTime? lastTaken;

    public override bool Grows => true;

    /// <summary>How many values it keeps: <c>points 60</c>, 60 without it.</summary>
    public int Points => (int)Math.Max(1, Spec.Number("points") ?? KeptPoints);

    /// <summary>Seconds between the values it takes: <c>every 0.5</c>, 1 without it, a tenth at least.</summary>
    public double Every => Math.Max(0.1, ChartPart.ReadNumber(Spec.Text("every")) ?? 1);

    /// <summary>The top of the axis, <c>max 100</c>; none lets it take in the largest value.</summary>
    public double? Max => ChartPart.ReadNumber(Spec.Text("max"));

    /// <summary>The values it has taken, oldest first.</summary>
    public IReadOnlyList<double> History => history;

    /// <summary>Takes the value it follows now, keeping the last <see cref="Points"/>; a value that is no number is not taken.</summary>
    public void Take()
    {
        if (Bound is not { } bound || ChartPart.ReadNumber(Host.Read(bound)) is not { } number)
            return;
        history.Add(number);
        if (history.Count > Points)
            history.RemoveRange(0, history.Count - Points);
    }

    List<ChartShown> Shown => ChartPart.NumbersShown(history.Cast<object?>(), 0);

    /// <summary>Where each value is drawn.</summary>
    public IReadOnlyList<ChartMark> Marks() => ChartDrawing.Marks(Bounds, ChartShape.Line, Shown, Max);

    /// <summary>On its screen it starts taking values at once, shown or hidden, as Avalonia's trend did while its window was open.</summary>
    protected override void OnAttached()
    {
        // Due at once, whatever the screen's clock says.
        Screen?.Wake(this, DateTime.MinValue);
    }

    /// <summary>The next value is due: shown, it draws again and drawing takes it; hidden, it takes it now, so its history has no gap when it shows again.</summary>
    public override void OnWake()
    {
        if (ShowsInTree)
            Invalidate();
        else
            Sample();
    }

    /// <summary>Takes the value when the next is due, and asks the screen to wake it for the one after.</summary>
    void Sample()
    {
        if (Screen is not { } screen)
            return;
        var now = screen.Now();
        if (lastTaken is { } last && now - last < TimeSpan.FromSeconds(Every))
            return;
        Take();
        lastTaken = now;
        screen.Wake(this, now + TimeSpan.FromSeconds(Every));
    }

    /// <summary>A value tells itself as the screen's tooltip, once the pointer rests on it.</summary>
    public override string? TooltipAt(double x, double y) => ChartDrawing.TipAt(Bounds, ChartShape.Line, Shown, Max, x, y) ?? base.TooltipAt(x, y);

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        Sample();
        ChartDrawing.Draw(painter, Bounds, ChartShape.Line, Shown, Max);
    }
}
