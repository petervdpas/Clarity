// A picture field: an image part bound to a record's picture. A new picture
// shows from its bytes, a stored one is fetched on its own and again for
// another record, one linked from a file of the field's store shows from
// there, an empty one shows its text, and a file that is no picture is
// refused (conformance/pictures.json). Its load and remove buttons show
// while the pointer or the focus is on it.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class PicturePartTests
{
    /// <summary>A PNG of one pixel.</summary>
    static readonly byte[] Pixel = Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==");

    static readonly JsonElement Cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "pictures.json"))).RootElement;

    const string Markup = """<Image at="1,1" span="6" rows="4" name="photo" bind="person.photo" />""";

    static (Screen Screen, PictureFieldPart Part, FilePartHost Host) Make(Picture? picture = null, string markup = Markup)
    {
        var host = new FilePartHost();
        host.Values["person"] = "first";
        host.Values["person.photo"] = picture;
        var (screen, _, parts) = VariantMade.Window(markup, host);
        return (screen, Assert.IsType<PictureFieldPart>(parts[0]), host);
    }

    [Fact]
    public void An_image_with_a_bind_is_a_picture_field_and_one_without_is_an_image()
    {
        var host = new FilePartHost();
        var (_, _, parts) = VariantMade.Window(Markup + """<Image at="7,1" span="6" source="logo.png" />""", host);
        Assert.IsType<PictureFieldPart>(parts[0]);
        Assert.IsType<ImagePart>(parts[1]);
    }

    [Fact]
    public void A_new_picture_shows_from_its_bytes()
    {
        var (_, part, _) = Make(Picture.New(Pixel));
        Assert.NotNull(part.Shown);
        Assert.False(part.IsEmpty);
    }

    [Fact]
    public void A_stored_picture_is_fetched_on_its_own()
    {
        var asked = 0;
        var host = new FilePartHost();
        host.Values["person"] = "first";
        host.Fetcher = (_, _) =>
        {
            asked++;
            return Task.FromResult<byte[]?>(Pixel);
        };
        host.Values["person.photo"] = Picture.Stored;
        var (_, _, parts) = VariantMade.Window(Markup, host);
        var part = Assert.IsType<PictureFieldPart>(parts[0]);
        Assert.NotNull(part.Shown);
        Assert.Equal(1, asked);
        // Nothing else changed: it is not fetched again.
        host.Change();
        Assert.Equal(1, asked);
    }

    /// <summary>Every stored picture is the same value: another record fetches its own.</summary>
    [Fact]
    public void Another_record_shows_its_own_picture()
    {
        var asked = new List<string>();
        var (_, _, host) = Make(Picture.Stored);
        host.Fetcher = (_, _) =>
        {
            asked.Add((string)host.Values["person"]!);
            return Task.FromResult<byte[]?>(Pixel);
        };
        host.Values["person"] = "second";
        host.Change();
        Assert.Equal(["second"], asked);
    }

    /// <summary>A picture linked from a file of the field's store, before it is saved.</summary>
    [Fact]
    public void A_picture_linked_from_a_file_shows_from_its_store()
    {
        string? asked = null;
        var (_, part, host) = Make();
        host.Fetcher = (_, path) =>
        {
            asked = path;
            return Task.FromResult<byte[]?>(Pixel);
        };
        host.Values["person.photo"] = Picture.InStore("2024/cover.png");
        host.Change();
        Assert.Equal("2024/cover.png", asked);
        Assert.NotNull(part.Shown);
    }

    [Fact]
    public void No_picture_shows_its_text()
    {
        var (_, part, host) = Make(Picture.New(Pixel));
        host.Values["person.photo"] = null;
        host.Change();
        Assert.Null(part.Shown);
        Assert.True(part.IsEmpty);
        Assert.Equal(Locale.Translate("noPicture", "No picture"), part.EmptyText);
    }

    [Fact]
    public void A_file_is_loaded_only_when_it_is_a_picture()
    {
        var (_, part, host) = Make();
        foreach (var hex in Cases.GetProperty("notPictures").EnumerateArray())
        {
            Assert.False(part.Load(Convert.FromHexString(hex.GetString()!)));
            Assert.True(part.Problem);
            Assert.Null(host.Values["person.photo"]);
        }
        foreach (var kind in Cases.GetProperty("kinds").EnumerateArray())
        {
            var bytes = Convert.FromHexString(kind.GetProperty("bytes").GetString()!);
            Assert.True(part.Load(bytes));
            Assert.False(part.Problem);
            Assert.Equal(Picture.New(bytes), host.Values["person.photo"]);
        }
        part.Remove();
        Assert.Null(host.Values["person.photo"]);
    }

    [Fact]
    public void Its_load_button_offers_pictures_and_takes_the_one_chosen()
    {
        var (_, part, host) = Make();
        host.Picker.NextFile = new ChosenFile("me.png", Pixel);
        part.LoadButton.Press();
        Assert.Equal(Picture.New(Pixel), host.Values["person.photo"]);
        Assert.Contains("*.png", host.Picker.Filter!.Patterns);
        Assert.Contains("*.webp", host.Picker.Filter!.Patterns);
        Assert.Equal(["write photo person.photo " + Picture.New(Pixel)], host.Log);
        part.RemoveButton.Press();
        Assert.Null(host.Values["person.photo"]);
    }

    [Fact]
    public void Its_buttons_show_while_the_pointer_or_the_focus_is_on_it()
    {
        var (screen, part, _) = Make(Picture.New(Pixel));
        Assert.False(part.Revealed);
        var (x, y) = (part.Bounds.X + 4, part.Bounds.Y + 4);
        screen.PointerMoved(x, y);
        Assert.True(part.Revealed);
        screen.PointerMoved(part.LoadButton.Bounds.X + 2, part.LoadButton.Bounds.Y + 2);
        Assert.True(part.Revealed);
        screen.PointerLeft();
        Assert.False(part.Revealed);
        screen.Focus(part.RemoveButton);
        Assert.True(part.Revealed);
        Assert.Equal(Locale.Translate("loadPicture", "Load a picture"), part.LoadButton.Text);
        Assert.Equal(Locale.Translate("removePicture", "Remove the picture"), part.RemoveButton.Text);
    }

    [Fact]
    public void Remove_shows_only_with_a_picture_and_a_read_only_one_has_no_buttons()
    {
        var (_, part, _) = Make();
        Assert.True(part.LoadButton.IsShown);
        Assert.False(part.RemoveButton.IsShown);
        var (_, fixedPart, host) = Make(Picture.New(Pixel), """<Image at="1,1" span="6" rows="4" name="photo" bind="person.photo" readonly="true" />""");
        Assert.False(fixedPart.LoadButton.IsShown);
        Assert.False(fixedPart.RemoveButton.IsShown);
        Assert.False(fixedPart.Load(Pixel));
        Assert.Empty(host.Log);
    }

    [Fact]
    public void It_draws_its_picture_and_its_empty_text()
    {
        var (screen, part, host) = Make(Picture.New(Pixel));
        screen.PointerMoved(part.Bounds.X + 4, part.Bounds.Y + 4);
        using (Shot.Of(screen, 800, 300)) { }
        host.Values["person.photo"] = null;
        host.Change();
        using var shot = Shot.Of(screen, 800, 300);
        Assert.Equal(800, shot.Width);
    }
}
