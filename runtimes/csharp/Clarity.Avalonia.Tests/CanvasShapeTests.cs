// The shapes canvas modes add to a drawing (DESIGN 5.31): a filled ellipse by
// its center and radii, an open line through points, and an object's box,
// which draws nothing itself. Read back from the pixels the canvas draws.

using Avalonia;
using Avalonia.Headless.XUnit;
using Avalonia.Media.Imaging;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class CanvasShapeTests
{
    /// The pixels a control draws at its size, as BGRA bytes, and its width.
    internal static (byte[] Pixels, int Width) Rendered(global::Avalonia.Controls.Control view, int width, int height)
    {
        view.Measure(new global::Avalonia.Size(width, height));
        view.Arrange(new global::Avalonia.Rect(0, 0, width, height));
        using var bitmap = new RenderTargetBitmap(new PixelSize(width, height));
        bitmap.Render(view);
        var pixels = new byte[width * height * 4];
        var at = System.Runtime.InteropServices.Marshal.AllocHGlobal(pixels.Length);
        try
        {
            bitmap.CopyPixels(new PixelRect(0, 0, width, height), at, pixels.Length, width * 4);
            System.Runtime.InteropServices.Marshal.Copy(at, pixels, 0, pixels.Length);
        }
        finally
        {
            System.Runtime.InteropServices.Marshal.FreeHGlobal(at);
        }
        return (pixels, width);
    }

    /// The red, green and blue of the pixel at x, y.
    internal static (byte R, byte G, byte B) Pixel((byte[] Pixels, int Width) shown, int x, int y)
    {
        var i = (y * shown.Width + x) * 4;
        return (shown.Pixels[i + 2], shown.Pixels[i + 1], shown.Pixels[i]);
    }

    [AvaloniaFact]
    public void A_canvas_draws_an_ellipse_and_a_path_and_an_object_draws_nothing()
    {
        var view = new CanvasView
        {
            UnitsWidth = 100,
            UnitsHeight = 100,
            Drawing =
            [
                new Shape.Clear("#000000"),
                new Shape.Object(1L, 0, 0, 100, 100),
                new Shape.Ellipse(30, 30, 20, 10, "#ff0000"),
                new Shape.Path([10, 80, 90, 80], "#00ff00", 4),
            ],
        };
        var shown = Rendered(view, 100, 100);
        Assert.Equal((255, 0, 0), Pixel(shown, 30, 30));
        Assert.Equal((255, 0, 0), Pixel(shown, 45, 30));
        Assert.Equal((0, 0, 0), Pixel(shown, 30, 45));
        Assert.Equal((0, 255, 0), Pixel(shown, 50, 80));
        Assert.Equal((0, 0, 0), Pixel(shown, 50, 90));
    }
}
