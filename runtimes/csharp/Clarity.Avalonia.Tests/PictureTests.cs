// A picture of the resources folder fits the part that shows it, an SVG as
// a bitmap does: `Stretch="Uniform"` in a cell smaller than the picture's own
// size draws it smaller, never past the cell (seen on Windows, 2026-10-28).
// Drawn by Skia, so the pixels can be read back.

using System.Text;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class PictureTests
{
    // Red squares far bigger than their cell, written each way an SVG gives its size.
    public static TheoryData<string> Squares =>
    [
        """<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="50 50 200 200"><rect x="50" y="50" width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" width="20cm" height="20cm" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
    ];

    static Color At(WriteableBitmap frame, int x, int y)
    {
        using var buffer = frame.Lock();
        var pixel = new byte[4];
        System.Runtime.InteropServices.Marshal.Copy(buffer.Address + y * buffer.RowBytes + x * 4, pixel, 0, 4);
        return buffer.Format == global::Avalonia.Platform.PixelFormat.Rgba8888
            ? Color.FromArgb(pixel[3], pixel[0], pixel[1], pixel[2])
            : Color.FromArgb(pixel[3], pixel[2], pixel[1], pixel[0]);
    }

    static bool Red(Color c) => c.R > 200 && c.G < 60 && c.B < 60;

    [AvaloniaTheory]
    [MemberData(nameof(Squares))]
    public void An_svg_fits_its_cell(string square)
    {
        var picture = ResourcePictures.Svg(new MemoryStream(Encoding.UTF8.GetBytes(square)));
        var image = new Image { Source = picture, Stretch = Stretch.Uniform };
        var cell = new Border { Width = 40, Height = 40, Child = image, HorizontalAlignment = global::Avalonia.Layout.HorizontalAlignment.Left, VerticalAlignment = global::Avalonia.Layout.VerticalAlignment.Top };
        var window = new Window { Width = 120, Height = 120, Content = cell, Background = Brushes.White };
        window.Show();
        var frame = window.CaptureRenderedFrame();
        Assert.NotNull(frame);
        var bitmap = new WriteableBitmap(frame!.PixelSize, frame.Dpi, frame.Format ?? global::Avalonia.Platform.PixelFormat.Bgra8888, global::Avalonia.Platform.AlphaFormat.Premul);
        using (var locked = bitmap.Lock())
            frame.CopyPixels(new PixelRect(frame.PixelSize), locked.Address, locked.RowBytes * locked.Size.Height, locked.RowBytes);
        Assert.True(Red(At(bitmap, 20, 20)), $"inside the cell: {At(bitmap, 20, 20)}");
        Assert.False(Red(At(bitmap, 80, 80)), $"past the cell: {At(bitmap, 80, 80)}");
        Assert.False(Red(At(bitmap, 20, 80)), $"below the cell: {At(bitmap, 20, 80)}");
        window.Close();
    }
}
