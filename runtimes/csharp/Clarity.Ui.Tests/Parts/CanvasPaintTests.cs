// A paint canvas bound to a record's picture paints on the picture the
// record has: a stored one is fetched on its own (as a picture field fetches
// it), one of the field's file store by its path, and another record's
// again. Until the picture is there a stroke paints nothing, so a stroke
// never sends a blank picture over the one stored.

using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui.Tests;

public class CanvasPaintTests
{
    const string Markup = """<Canvas name="pad" units="{&quot;width&quot;:100,&quot;height&quot;:100}" mode="paint" bind="sheet.picture" />""";

    /// <summary>A PNG 100 by 100, clear but for one red pixel at 80, 20.</summary>
    static byte[] Marked()
    {
        using var bitmap = new SKBitmap(new SKImageInfo(100, 100, SKColorType.Rgba8888, SKAlphaType.Premul));
        bitmap.Erase(SKColors.Transparent);
        bitmap.SetPixel(80, 20, SKColors.Red);
        using var image = SKImage.FromBitmap(bitmap);
        using var png = image.Encode(SKEncodedImageFormat.Png, 100);
        return png.ToArray();
    }

    static (CanvasPartHost Host, Screen Screen) Canvas(Picture? picture, Func<string, string?, Task<byte[]?>> fetcher) =>
        Canvas(picture, fetcher, _ => { });

    /// <summary>A stroke from unit 10, 10 to 20, 20 (the canvas shows 4 pixels a unit, from 200 across).</summary>
    static void Stroke(Screen screen)
    {
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerMoved(280, 80);
        screen.PointerReleased(280, 80, PointerButton.Left);
    }

    static SKBitmap Sent(CanvasPartHost host) => SKBitmap.Decode(CanvasModes.StrokeBytes(Assert.Single(host.Actions).Action));

    [Fact]
    public void A_stored_picture_is_fetched_and_a_stroke_paints_on_it()
    {
        var asked = new List<(string Bind, string? Path)>();
        var (host, screen) = Canvas(Picture.Stored, (bind, path) =>
        {
            asked.Add((bind, path));
            return Task.FromResult<byte[]?>(Marked());
        });
        Assert.Equal([("sheet.picture", (string?)null)], asked);
        Stroke(screen);
        using var after = Sent(host);
        // What the record had stays, with the stroke over it.
        Assert.Equal(SKColors.Red, after.PixelAt(80, 20));
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("text"), after.PixelAt(15, 15));
        // Nothing else changed: it is not fetched again.
        host.Change();
        Assert.Single(asked);
    }

    [Fact]
    public void A_picture_of_the_field_s_file_store_is_fetched_by_its_path()
    {
        string? asked = null;
        var (host, screen) = Canvas(Picture.InStore("2024/sketch.png"), (_, path) =>
        {
            asked = path;
            return Task.FromResult<byte[]?>(Marked());
        });
        Assert.Equal("2024/sketch.png", asked);
        Stroke(screen);
        using var after = Sent(host);
        Assert.Equal(SKColors.Red, after.PixelAt(80, 20));
    }

    [Fact]
    public void Another_record_paints_on_its_own_picture()
    {
        var asked = new List<string>();
        CanvasPartHost? kept = null;
        var (host, _) = Canvas(Picture.Stored, (_, _) =>
        {
            asked.Add((string)kept!.Values["sheet"]!);
            return Task.FromResult<byte[]?>(Marked());
        }, before: h => kept = h);
        host.Values["sheet"] = "second";
        host.Change();
        Assert.Equal(["first", "second"], asked);
    }

    [Fact]
    public void A_stroke_before_the_stored_picture_is_there_paints_nothing_and_sends_nothing()
    {
        var answer = new TaskCompletionSource<byte[]?>();
        var (host, screen) = Canvas(Picture.Stored, (_, _) => answer.Task);
        Stroke(screen);
        Assert.Empty(host.Actions);
        answer.SetResult(Marked());
        Stroke(screen);
        using var after = Sent(host);
        Assert.Equal(SKColors.Red, after.PixelAt(80, 20));
    }

    [Fact]
    public void A_picture_that_does_not_load_leaves_the_canvas_blank_to_paint_on()
    {
        var (host, screen) = Canvas(Picture.New([1, 2, 3]), (_, _) => Task.FromResult<byte[]?>(null));
        Stroke(screen);
        using var after = Sent(host);
        Assert.Equal((100, 100), (after.Width, after.Height));
        Assert.Equal(0, after.PixelAt(80, 20).Alpha);
    }

    static (CanvasPartHost Host, Screen Screen) Canvas(Picture? picture, Func<string, string?, Task<byte[]?>> fetcher, Action<CanvasPartHost> before)
    {
        var host = new CanvasPartHost { Fetcher = fetcher };
        before(host);
        host.Values["sheet"] = "first";
        host.Values["sheet.picture"] = picture;
        var (screen, _) = CanvasPartTests.Alone(Markup, host);
        // Drawn once, as a window shows it.
        using var _shot = Shot.Of(screen, 800, 400);
        return (host, screen);
    }
}
