// A picture field: an image part bound to a record's picture, the same as
// on every stack (runtimes/web/src/kinds/PictureField.svelte). The picture
// shows fitted in proportion and centered; without one, its empty text in a
// dashed frame. A stored picture is fetched on its own (RecordFetch), and
// again when another record shows, since every stored picture is the same
// value; a new one shows from its bytes until the record is saved. Its
// buttons to load a picture from a file and to remove it show while the
// pointer or the focus is on it. A file that is no picture is refused and
// marks the field; a read-only one has no buttons. Loading and removing
// write the field.

using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

[PartVariant("image")]
public sealed class PictureFieldPart : Part
{
    const double Gap = 4;

    /// <summary>The kinds of file the load button offers, as Pictures tells them.</summary>
    static readonly string[] Patterns = ["*.png", "*.jpg", "*.jpeg", "*.gif", "*.bmp", "*.webp"];

    SKBitmap? shown;
    Picture? fetchedFor;
    object? recordFor;
    bool asked;
    int asks;

    /// <summary>An image with a bind shows a record's picture.</summary>
    public static bool Fits(PartSpec spec, IPartHost host) => spec.Text("bind") is { Length: > 0 };

    public override bool Grows => true;

    /// <summary>Loads a picture from a file.</summary>
    public ContainerButton LoadButton { get; private set; } = null!;

    /// <summary>Removes the picture.</summary>
    public ContainerButton RemoveButton { get; private set; } = null!;


    /// <summary>The field's picture; none when it has none.</summary>
    public Picture? Value => Bound is { } path ? Host.Read(path) as Picture : null;

    /// <summary>The picture shown; none while there is none.</summary>
    public SKBitmap? Shown
    {
        get
        {
            Fetch();
            return shown;
        }
    }

    public bool IsEmpty => Shown is null;

    /// <summary>What it shows without a picture, in Clarity's own words.</summary>
    public string EmptyText => Locale.Translate("noPicture", "No picture");

    /// <summary>The last file loaded was no picture.</summary>
    public bool Problem { get; private set; }

    /// <summary>Whether the person may load or remove a picture.</summary>
    public bool CanLoad => !ReadOnly && Enabled;

    /// <summary>Its buttons show: the pointer or the focus is on it.</summary>
    public bool Revealed => InOrder().Any(e => e.Hovered || e.Focused);

    protected internal override void Made()
    {
        LoadButton = new ContainerButton(() => Locale.Translate("loadPicture", "Load a picture"), () => _ = Choose())
        {
            Shows = () => CanLoad,
        };
        RemoveButton = new ContainerButton(() => Locale.Translate("removePicture", "Remove the picture"), Remove)
        {
            Shows = () => CanLoad && Value is not null,
        };
        AddChild(LoadButton);
        AddChild(RemoveButton);
        Host.Changed += Heard;
        Fetch();
    }

    void Heard()
    {
        Fetch();
        Arrange(Bounds);
        Invalidate();
    }

    /// <summary>Fetches the picture when the field or its record changed; an answer to an older ask is left.</summary>
    void Fetch()
    {
        var picture = Value;
        var record = RecordFetch.Record(Host, Bound);
        if (asked && ReferenceEquals(picture, fetchedFor) && ReferenceEquals(record, recordFor))
            return;
        asked = true;
        (fetchedFor, recordFor) = (picture, record);
        _ = Show(picture, ++asks);
    }

    async Task Show(Picture? picture, int ask)
    {
        byte[]? bytes;
        try
        {
            bytes = await RecordFetch.Bytes(Host, Bound, picture);
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{Spec.Name ?? Spec.Kind} cannot fetch its picture: {e.Message}");
            bytes = null;
        }
        if (ask != asks)
            return;
        shown?.Dispose();
        shown = ImagePart.Picture("picture", bytes);
        Invalidate();
    }

    /// <summary>Takes the bytes as the new picture when they are one; false when they are not, or it may not.</summary>
    public bool Load(byte[] bytes)
    {
        if (!CanLoad || Bound is not { } path)
            return false;
        Problem = Pictures.KindOf(bytes) is null;
        Invalidate();
        if (Problem)
            return false;
        Host.Write(Spec.Name ?? "", path, Picture.New(bytes));
        return true;
    }

    /// <summary>The picture removed.</summary>
    public void Remove()
    {
        if (!CanLoad || Bound is not { } path)
            return;
        Problem = false;
        if (Value is not null)
            Host.Write(Spec.Name ?? "", path, null);
    }

    /// <summary>Asks the system's dialog for a picture, and loads the one chosen.</summary>
    async Task Choose()
    {
        if (!CanLoad)
            return;
        try
        {
            var title = Locale.Translate("loadPicture", "Load a picture");
            if (await Host.Files.OpenFile(title, new FileFilter(title, Patterns)) is { } chosen)
                Load(chosen.Bytes);
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{Spec.Name ?? Spec.Kind} cannot load a picture: {e.Message}");
        }
    }

    /// <summary>Its buttons in a row, centered at its bottom, each as wide as its words.</summary>
    public override void Arrange(Box box)
    {
        Bounds = box;
        var look = LookNow;
        var height = Math.Min(look.Size("text") + 4 * look.Padding, Math.Max(0, box.Height - 2 * Gap));
        var buttons = new[] { LoadButton, RemoveButton };
        var widths = buttons.Select(b => b.IsShown ? b.WidthFor(FontsNow, look) : 0).ToArray();
        var total = widths.Sum() + Gap * Math.Max(0, widths.Count(w => w > 0) - 1);
        var x = box.X + (box.Width - total) / 2;
        var y = box.Bottom - Gap - height;
        for (var i = 0; i < buttons.Length; i++)
        {
            buttons[i].Arrange(new Box(x, y, widths[i], height));
            if (widths[i] > 0)
                x += widths[i] + Gap;
        }
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        if (Shown is { } bitmap)
        {
            using var image = SKImage.FromBitmap(bitmap);
            painter.Canvas.DrawImage(image, ImagePart.Fit(Bounds, bitmap.Width, bitmap.Height).Rect, new SKSamplingOptions(SKFilterMode.Linear, SKMipmapMode.Linear));
        }
        else
        {
            using var dashed = new SKPaint
            {
                Color = look.Color("strong_border"),
                IsAntialias = true,
                Style = SKPaintStyle.Stroke,
                StrokeWidth = look.Border,
                PathEffect = SKPathEffect.CreateDash([4, 4], 0),
            };
            var radius = look.Radius("panel");
            painter.Canvas.DrawRoundRect(Bounds.Inset(look.Border / 2.0).Rect, radius, radius, dashed);
            painter.Text(EmptyText, Bounds.Inset(2 * look.Padding), look.Color("muted"), look.Size("small"), align: TextAlign.Center);
        }
        // A file that was no picture marks the field in the error color.
        if (Problem)
            painter.Stroke(Bounds, look.Color("error"), 2, look.Radius("panel"));
        if (Revealed)
            base.Draw(painter);
    }
}
