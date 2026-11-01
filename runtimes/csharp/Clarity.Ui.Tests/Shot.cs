// A screen drawn into pixels, as a window shows it, for the tests to look at.

namespace Clarity.Ui.Tests;

static class Shot
{
    public static SKBitmap Of(Screen screen, int width, int height, float scale = 1)
    {
        screen.Resize(width / scale, height / scale, scale);
        var bitmap = new SKBitmap(new SKImageInfo(width, height, SKColorType.Rgba8888, SKAlphaType.Premul));
        using var canvas = new SKCanvas(bitmap);
        screen.Draw(canvas);
        return bitmap;
    }
}
