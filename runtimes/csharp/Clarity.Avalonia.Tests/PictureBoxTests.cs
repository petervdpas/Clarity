// A picture field on Avalonia: a new picture shows from its bytes, a stored
// one is fetched on its own, an empty one shows its text, and a file that is
// no picture is refused.

using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class PictureBoxTests
{
    /// A PNG of one pixel.
    static readonly byte[] Pixel = Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==");

    [AvaloniaFact]
    public void ANewPictureShowsFromItsBytes()
    {
        var box = new PictureBox { EmptyText = "No picture", Picture = Picture.New(Pixel) };
        Assert.NotNull(box.Shown);
        Assert.False(box.IsEmpty);
    }

    [AvaloniaFact]
    public void AStoredPictureIsFetchedOnItsOwn()
    {
        var asked = 0;
        var box = new PictureBox
        {
            Fetch = () =>
            {
                asked++;
                return Pixel;
            },
        };
        box.Picture = Picture.Stored;
        Assert.NotNull(box.Shown);
        Assert.Equal(1, asked);
    }

    /// Every stored picture is the same value: another record fetches its own.
    [AvaloniaFact]
    public void AnotherRecordShowsItsOwnPicture()
    {
        var asked = new List<string>();
        var box = new PictureBox { Record = "first", Picture = Picture.Stored };
        box.Fetch = () =>
        {
            asked.Add((string)box.Record!);
            return Pixel;
        };
        box.Record = "second";
        Assert.Equal(["first", "second"], asked);
    }

    /// A picture linked from a file of the field's store, before it is saved.
    [AvaloniaFact]
    public void APictureLinkedFromAFileShowsFromItsStore()
    {
        string? asked = null;
        var box = new PictureBox
        {
            FetchFile = path =>
            {
                asked = path;
                return Pixel;
            },
        };
        box.Picture = Picture.InStore("2024/cover.png");
        Assert.Equal("2024/cover.png", asked);
        Assert.NotNull(box.Shown);
    }

    [AvaloniaFact]
    public void NoPictureShowsItsText()
    {
        var box = new PictureBox { EmptyText = "Geen afbeelding", Picture = Picture.New(Pixel) };
        box.Picture = null;
        Assert.Null(box.Shown);
        Assert.True(box.IsEmpty);
        Assert.Contains(box.Children.OfType<TextBlock>(), t => t.Text == "Geen afbeelding" && t.IsVisible);
    }

    [AvaloniaFact]
    public void AFileIsLoadedOnlyWhenItIsAPicture()
    {
        var box = new PictureBox();
        Assert.False(box.Load("%PDF-1.7"u8.ToArray()));
        Assert.Null(box.Picture);
        Assert.True(box.Load(Pixel));
        Assert.Equal(Picture.New(Pixel), box.Picture);
        box.Remove();
        Assert.Null(box.Picture);
    }
}

