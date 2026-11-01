// A window's client area: the window's grid, in the ScrollViewer around it,
// is as high as the window, so rows that grow share its height and a list in
// them scrolls in its own rows. When the window is lower than the grid's rows
// at their least height, the grid takes that height and the window scrolls.
// A ScrollViewer alone would let each part ask for all the height it wants,
// and a long list would make the window as long as the list.

using Avalonia;
using Avalonia.Controls;
using Avalonia.VisualTree;

namespace Clarity.Avalonia;

public sealed class WindowClient : Decorator
{
    ScrollViewer? scroll;

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        scroll = this.FindAncestorOfType<ScrollViewer>();
        if (scroll is not null)
            scroll.PropertyChanged += ScrollChanged;
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        if (scroll is not null)
            scroll.PropertyChanged -= ScrollChanged;
        scroll = null;
        base.OnDetachedFromVisualTree(e);
    }

    void ScrollChanged(object? sender, AvaloniaPropertyChangedEventArgs e)
    {
        if (e.Property == ScrollViewer.ViewportProperty)
            InvalidateMeasure();
    }

    protected override Size MeasureOverride(Size available)
    {
        if (Child is null)
            return default;
        var width = double.IsInfinity(available.Width) ? scroll?.Viewport.Width ?? 0 : available.Width;
        var least = Least(Child);
        var window = scroll?.Viewport.Height ?? 0;
        var height = double.IsFinite(window) && window > least ? window : least;
        Child.Measure(new Size(width, height));
        return new Size(width, height);
    }

    /// The height of the window's grid with its rows at their least: a fixed
    /// row as it is written, a row that grows at its least height, with the
    /// spacing between rows and the grid's margin.
    public static double Least(Control child)
    {
        if (child is not Grid grid || grid.RowDefinitions.Count == 0)
            return 0;
        var rows = grid.RowDefinitions.Sum(r => r.Height.IsAbsolute ? Math.Max(r.Height.Value, r.MinHeight) : r.MinHeight);
        return rows + grid.RowSpacing * (grid.RowDefinitions.Count - 1) + grid.Margin.Top + grid.Margin.Bottom;
    }

    protected override Size ArrangeOverride(Size final)
    {
        Child?.Arrange(new Rect(final));
        return final;
    }
}
