// An SVG picture of the resources folder is drawn from its shapes at the
// size it is shown, sharp however large, and fits its place whichever way
// the SVG gives its size (Avalonia's PictureTests.An_svg_fits_its_cell).

using System.Text;
using SkiaSharp;

namespace Clarity.Ui.Tests;

public class ImageSvgTests
{
    // Red squares far bigger than their place, written each way an SVG gives its size.
    public static TheoryData<string> Squares =>
    [
        """<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="50 50 200 200"><rect x="50" y="50" width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" width="20cm" height="20cm" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
        """<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>""",
    ];

    static bool Red(SKColor c) => c.Red > 200 && c.Green < 60 && c.Blue < 60;

    static (Screen Screen, ImagePart Image) Shown(string svg, double width, double height)
    {
        var host = new TestHost();
        host.Resources["square.svg"] = Encoding.UTF8.GetBytes(svg);
        var (screen, part) = CanvasPartTests.Alone("""<Image name="logo" source="square.svg" />""", host, width, height);
        return (screen, (ImagePart)part);
    }

    [Theory]
    [MemberData(nameof(Squares))]
    public void An_svg_fits_its_place(string square)
    {
        // A 40 by 40 place in a 120 by 120 picture, as Avalonia's cell in its window:
        // the points past the place and below it are inside the picture.
        var (screen, image) = Shown(square, 40, 40);
        using var shot = new SKBitmap(new SKImageInfo(120, 120, SKColorType.Rgba8888, SKAlphaType.Premul));
        using (var canvas = new SKCanvas(shot))
            screen.Draw(canvas);
        Assert.True(image.Shown?.Vector);
        Assert.True(Red(shot.PixelAt(20, 20)), $"inside its place: {shot.PixelAt(20, 20)}");
        Assert.False(Red(shot.PixelAt(80, 80)), $"past its place: {shot.PixelAt(80, 80)}");
        Assert.False(Red(shot.PixelAt(20, 80)), $"below its place: {shot.PixelAt(20, 80)}");
    }

    [Fact]
    public void An_svg_shown_large_stays_sharp()
    {
        // Ten units wide, black on the left half: shown 800 pixels wide, its edge is at 400.
        const string halves = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="5" height="10" fill="#000000"/><rect x="5" width="5" height="10" fill="#ffffff"/></svg>""";
        var (screen, _) = Shown(halves, 800, 800);
        using var shot = Shot.Of(screen, 800, 800);
        // A picture drawn small and blown up blurs pixels around its edge; shapes drawn at the size shown do not.
        Assert.Equal(SKColors.Black, shot.PixelAt(398, 400));
        Assert.Equal(SKColors.White, shot.PixelAt(401, 400));
    }
}
