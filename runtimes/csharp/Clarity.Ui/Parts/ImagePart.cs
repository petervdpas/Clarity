// An image: a picture of the project's resources folder by its name (its
// `source`, or a text key that names one), fitted into its place in
// proportion and centered. A picture that is not there shows its name in a
// dashed frame. Bitmaps (PNG, JPEG, WebP, GIF's first frame) are drawn;
// SVG pictures are not yet, and show as one that is not there. The host
// gives the picture's bytes (a generated application's embedded resources).

using SkiaSharp;

namespace Clarity.Ui;

[PartKind("image")]
public sealed class ImagePart : Part
{
    string? loadedName;
    SKBitmap? loaded;

    public override bool Grows => true;

    /// <summary>The picture's name: its source, else what its caption says in the language shown.</summary>
    public string? Named => Spec.Text("source") is { Length: > 0 } source ? source
        : Spec.Text("caption") is { Length: > 0 } caption ? Host.Text(caption)
        : null;

    protected internal override void Made() => Host.Changed += Invalidate;

    /// <summary>The picture of <paramref name="bytes"/>; null when there are none, or they are no bitmap. An SVG picture is not drawn yet.</summary>
    public static SKBitmap? Picture(string? name, byte[]? bytes)
    {
        if (string.IsNullOrEmpty(name) || bytes is null || name.EndsWith(".svg", StringComparison.OrdinalIgnoreCase))
            return null;
        try
        {
            return SKBitmap.Decode(bytes);
        }
        catch (Exception e) when (e is IOException or ArgumentException or InvalidOperationException)
        {
            return null;
        }
    }

    /// <summary>The picture it shows, as the host gives its bytes, loaded once per name.</summary>
    public SKBitmap? Shown
    {
        get
        {
            var name = Named;
            if (name != loadedName)
            {
                loaded?.Dispose();
                loadedName = name;
                loaded = string.IsNullOrEmpty(name) ? null : Picture(name, Host.Resource(name));
            }
            return loaded;
        }
    }

    /// <summary>Where a picture <paramref name="width"/> by <paramref name="height"/> is drawn in <paramref name="box"/>: as big as fits, in proportion, centered.</summary>
    public static Box Fit(Box box, double width, double height)
    {
        if (width <= 0 || height <= 0)
            return box with { Width = 0, Height = 0 };
        var scale = Math.Min(box.Width / width, box.Height / height);
        var (w, h) = (width * scale, height * scale);
        return new Box(box.X + (box.Width - w) / 2, box.Y + (box.Height - h) / 2, w, h);
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        var name = Named;
        if (Shown is { } bitmap)
        {
            using var image = SKImage.FromBitmap(bitmap);
            painter.Canvas.DrawImage(image, Fit(Bounds, bitmap.Width, bitmap.Height).Rect, new SKSamplingOptions(SKFilterMode.Linear, SKMipmapMode.Linear));
            return;
        }
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
        painter.Text(name ?? "", Bounds.Inset(2 * look.Padding), look.Color("muted"), look.Size("small"), "mono", align: TextAlign.Center);
    }
}
