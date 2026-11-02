// How the tests read a pixel of a shot. SKBitmap.GetPixel does not check
// its place: a point past the edge reads memory past the pixels and, when
// that memory is not mapped, the whole test run dies (SIGSEGV), with xUnit
// still saying "Passed!" for the tests it got through. PixelAt fails the
// one test instead. Tests read pixels only through it (guarded by
// PixelsTests.Tests_read_pixels_only_through_PixelAt).

using SkiaSharp;

namespace Clarity.Ui.Tests;

static class Pixels
{
    /// <summary>The color at <paramref name="x"/>, <paramref name="y"/>; a test failure, not a crash, when the point is past the edge.</summary>
    public static SKColor PixelAt(this SKBitmap bitmap, int x, int y)
    {
        if (x < 0 || y < 0 || x >= bitmap.Width || y >= bitmap.Height)
            throw new ArgumentOutOfRangeException(nameof(x), $"the point {x}, {y} is past the edge of a {bitmap.Width} by {bitmap.Height} picture");
        return bitmap.GetPixel(x, y);
    }
}
