// A trend (DESIGN 5.26), the same as on the web: the last values of one
// number, taken every so many seconds while its window shows, drawn as
// Clarity's line chart with the newest at the right. It takes a value when
// it is first drawn, and asks the screen to wake it when the next is due
// (Screen.Wake), so it draws once a second rather than every frame.

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("trend")]
public sealed class TrendPart : Part
{
    /// <summary>How many values a trend keeps without <c>points</c>.</summary>
    public const int KeptPoints = 60;

    readonly List<double> history = [];
    DateTime? lastTaken;
    (double X, double Y)? pointer;

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

    /// <summary>The next value is due: it draws again, and drawing takes it.</summary>
    public override void OnWake() => Invalidate();

    public override void OnPointerMoved(PointerInput at)
    {
        pointer = (at.ScreenX, at.ScreenY);
        Invalidate();
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        if (Screen is { } screen)
        {
            var now = screen.Now();
            if (lastTaken is not { } last || now - last >= TimeSpan.FromSeconds(Every))
            {
                Take();
                lastTaken = now;
                screen.Wake(this, now + TimeSpan.FromSeconds(Every));
            }
        }
        ChartDrawing.Draw(painter, Bounds, ChartShape.Line, Shown, Max, Hovered ? pointer : null);
    }
}
