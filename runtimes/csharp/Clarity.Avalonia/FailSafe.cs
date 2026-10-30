// Fail safe: a control whose drawing fails shows why in its own place, in
// red, instead of ending the application; the rest of the window keeps working.

using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;

namespace Clarity.Avalonia;

public static class FailSafe
{
    /// <summary>Runs <paramref name="draw"/>; when it fails, draws the reason over the control and gives it back, else null.</summary>
    public static string? Render(Control control, DrawingContext context, Action draw)
    {
        try
        {
            draw();
            return null;
        }
        catch (Exception e)
        {
            Console.Error.WriteLine($"{control.Name ?? control.GetType().Name} could not be drawn: {e}");
            var shown = new FormattedText($"{control.Name ?? control.GetType().Name}: {e.Message}", CultureInfo.CurrentUICulture, FlowDirection.LeftToRight, Typeface.Default, 12, Brushes.IndianRed)
            {
                MaxTextWidth = Math.Max(1, control.Bounds.Width - 8),
            };
            context.DrawRectangle(null, new Pen(Brushes.IndianRed, 1, DashStyle.Dash), new Rect(control.Bounds.Size).Deflate(0.5));
            context.DrawText(shown, new Point(4, 4));
            return e.Message;
        }
    }
}
