// An image: a picture of the project's resources folder by its name (its
// `source`, or a text key that names one), fitted into its place in
// proportion and centered. A picture that is not there shows its name in a
// dashed frame. Bitmaps (PNG, JPEG, WebP, GIF's first frame) are drawn as
// they are; an SVG picture is drawn from its shapes at the size it is shown,
// so it stays sharp however large (PictureArt). The host gives the picture's
// bytes (a generated application's embedded resources).

using SkiaSharp;

namespace Clarity.Ui;

[PartKind("image")]
public sealed class ImagePart : Part
{
    string? loadedName;
    PictureArt? loaded;

    public override bool Grows => true;

    /// <summary>The picture's name: its source, else what its caption says in the language shown.</summary>
    public string? Named => Spec.Text("source") is { Length: > 0 } source ? source
        : Spec.Text("caption") is { Length: > 0 } caption ? Host.Text(caption)
        : null;

    protected internal override void Made() => Host.Changed += Invalidate;

    /// <summary>The picture of <paramref name="bytes"/>; null when there are none, or they are no picture. An SVG picture is drawn from its shapes.</summary>
    public static SKBitmap? Picture(string? name, byte[]? bytes)
    {
        if (string.IsNullOrEmpty(name) || bytes is null)
            return null;
        try
        {
            return name.EndsWith(".svg", StringComparison.OrdinalIgnoreCase) ? Drawn(bytes) : SKBitmap.Decode(bytes);
        }
        catch (Exception e) when (e is IOException or ArgumentException or InvalidOperationException or System.Xml.XmlException or FormatException or NullReferenceException)
        {
            return null;
        }
    }

    // The longer side an SVG picture is drawn at, at least, so it stays sharp when shown larger than it names.
    const float SvgSide = 512;

    /// <summary>An SVG picture drawn into a bitmap of its own proportions, its longer side at least <see cref="SvgSide"/>; null when it is no SVG.</summary>
    static SKBitmap? Drawn(byte[] bytes)
    {
        using var svg = new Svg.Skia.SKSvg();
        using var stream = new MemoryStream(bytes);
        if (svg.Load(stream) is not { } picture)
            return null;
        var size = picture.CullRect;
        if (size.Width <= 0 || size.Height <= 0)
            return null;
        var scale = Math.Clamp(SvgSide / Math.Max(size.Width, size.Height), 1, 8);
        var bitmap = new SKBitmap(new SKImageInfo((int)Math.Ceiling(size.Width * scale), (int)Math.Ceiling(size.Height * scale), SKColorType.Rgba8888, SKAlphaType.Premul));
        using var canvas = new SKCanvas(bitmap);
        canvas.Clear(SKColors.Transparent);
        canvas.Scale(scale);
        canvas.Translate(-size.Left, -size.Top);
        canvas.DrawPicture(picture);
        return bitmap;
    }

    /// <summary>The picture it shows, as the host gives its bytes, loaded once per name.</summary>
    public PictureArt? Shown
    {
        get
        {
            var name = Named;
            if (name != loadedName)
            {
                loaded?.Dispose();
                loadedName = name;
                loaded = string.IsNullOrEmpty(name) ? null : PictureArt.Of(name, Host.Resource(name));
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
        if (Shown is { } art)
        {
            art.Draw(painter.Canvas, Fit(Bounds, art.Width, art.Height).Rect);
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

/// <summary>
/// A picture as a part or a canvas draws it: a bitmap as it is, or an SVG
/// picture kept as its shapes and drawn at the size it is shown, so it is
/// sharp at any size (Avalonia drew it at twice the size shown, up to 4096
/// pixels; shapes need no such limit).
/// </summary>
public sealed class PictureArt : IDisposable
{
    readonly SKBitmap? bitmap;
    readonly Svg.Skia.SKSvg? svg;
    readonly SKRect cull;

    PictureArt(SKBitmap bitmap) => (this.bitmap, Width, Height) = (bitmap, bitmap.Width, bitmap.Height);

    PictureArt(Svg.Skia.SKSvg svg, SKRect cull) => (this.svg, this.cull, Width, Height) = (svg, cull, cull.Width, cull.Height);

    /// <summary>Its own size: a bitmap's pixels, an SVG's drawn shapes.</summary>
    public double Width { get; }

    public double Height { get; }

    /// <summary>Whether it is drawn from shapes (an SVG), not pixels.</summary>
    public bool Vector => svg is not null;

    /// <summary>The picture of <paramref name="bytes"/> named <paramref name="name"/>; none when there are none or they are no picture.</summary>
    public static PictureArt? Of(string? name, byte[]? bytes)
    {
        if (string.IsNullOrEmpty(name) || bytes is null)
            return null;
        if (!name.EndsWith(".svg", StringComparison.OrdinalIgnoreCase))
            return ImagePart.Picture(name, bytes) is { } decoded ? new PictureArt(decoded) : null;
        var svg = new Svg.Skia.SKSvg();
        try
        {
            using var stream = new MemoryStream(bytes);
            if (svg.Load(stream) is { CullRect: { Width: > 0, Height: > 0 } cull })
                return new PictureArt(svg, cull);
        }
        catch (Exception e) when (e is IOException or ArgumentException or InvalidOperationException or System.Xml.XmlException or FormatException or NullReferenceException)
        {
            // Fail safe: an SVG that does not read is no picture.
        }
        svg.Dispose();
        return null;
    }

    /// <summary>Draws it stretched to <paramref name="into"/>.</summary>
    public void Draw(SKCanvas canvas, SKRect into)
    {
        if (into.Width <= 0 || into.Height <= 0)
            return;
        if (bitmap is not null)
        {
            using var image = SKImage.FromBitmap(bitmap);
            canvas.DrawImage(image, into, new SKSamplingOptions(SKFilterMode.Linear, SKMipmapMode.Linear));
            return;
        }
        if (svg?.Picture is not { } picture)
            return;
        canvas.Save();
        canvas.ClipRect(into);
        canvas.Translate(into.Left, into.Top);
        canvas.Scale(into.Width / cull.Width, into.Height / cull.Height);
        canvas.Translate(-cull.Left, -cull.Top);
        canvas.DrawPicture(picture);
        canvas.Restore();
    }

    public void Dispose()
    {
        bitmap?.Dispose();
        svg?.Dispose();
    }
}
