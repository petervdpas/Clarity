// The picture functions through the native Clarity.Image library, as a
// generated program calls them (docs/IMAGE-PLAN.md).

using Clarity.Image;
using Clarity.Runtime;

public class PictureFunctionsTests
{
    /// <summary>Four by two red pixels, as PNG.</summary>
    private static readonly byte[] Red = Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAQAAAACCAYAAAB/qH1jAAAAEklEQVR4nGP4z8DwHxkzoAsAAA8hD/EEN8afAAAAAElFTkSuQmCC");

    private static (long, long) Size(Picture p) => (PictureFunctions.Width(p), PictureFunctions.Height(p));

    [Fact]
    public void EachFunctionGivesANewPicture()
    {
        var p = Picture.New(Red);
        Assert.Equal((8L, 6L), Size(PictureFunctions.Resize(p, 8, 6)));
        Assert.Equal((2L, 1L), Size(PictureFunctions.Crop(p, 1, 1, 2, 5)));
        Assert.Equal((2L, 4L), Size(PictureFunctions.Rotate(p, 90m)));
        Assert.Equal((4L, 2L), Size(PictureFunctions.Flip(p, true)));
        Assert.Equal((4L, 2L), Size(PictureFunctions.Blend(p, PictureFunctions.Resize(p, 1, 1), 1, 1, 50)));
        Assert.Equal((4L, 2L), Size(PictureFunctions.Adjust(p, 30m, -20.5m, 0m, 0m)));
        Assert.Equal((4L, 2L), Size(p));
        Assert.Equal(4L, PictureFunctions.Width(Red));
    }

    [Fact]
    public void NoPictureIsRefusedWithAMessage()
    {
        Assert.Throws<InvalidValueException>(() => PictureFunctions.Resize((Picture?)null, 2, 2));
        Assert.Throws<InvalidValueException>(() => PictureFunctions.Resize(Picture.Stored, 2, 2));
        var e = Assert.Throws<InvalidValueException>(() => PictureFunctions.Resize("no picture"u8.ToArray(), 2, 2));
        Assert.False(string.IsNullOrEmpty(e.Message));
        Assert.Throws<InvalidValueException>(() => PictureFunctions.Resize(Red, 0, 2));
    }
}
