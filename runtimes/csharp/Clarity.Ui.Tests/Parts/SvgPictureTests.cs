// An SVG picture (an image part's, a canvas image shape's) is drawn from its
// shapes, as a bitmap at least the size it names.

using SkiaSharp;

namespace Clarity.Ui.Tests;

public class SvgPictureTests
{
    const string Figure = """<svg xmlns="http://www.w3.org/2000/svg" width="40" height="80" viewBox="0 0 40 80"><rect x="10" y="10" width="20" height="60" fill="#202020"/></svg>""";

    [Fact]
    public void An_svg_picture_is_drawn_from_its_shapes()
    {
        using var picture = ImagePart.Picture("walk-right-1.svg", System.Text.Encoding.UTF8.GetBytes(Figure));
        Assert.NotNull(picture);
        Assert.True(picture!.Width >= 40 && picture.Height >= 80);
        Assert.Equal(picture.Width * 2, picture.Height);
        // The figure is dark in the middle, and nothing is drawn around it.
        Assert.Equal(new SKColor(0x20, 0x20, 0x20), picture.GetPixel(picture.Width / 2, picture.Height / 2));
        Assert.Equal(0, picture.GetPixel(1, 1).Alpha);
    }

    [Fact]
    public void An_svg_that_is_no_svg_is_no_picture()
    {
        Assert.Null(ImagePart.Picture("broken.svg", System.Text.Encoding.UTF8.GetBytes("not svg")));
    }
}
