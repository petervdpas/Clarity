// The pictures of the application's resources folder, by name: an SVG drawn
// as vectors, any other picture as a bitmap. A canvas's image shapes, the
// image part and a key that names a picture all show them through here
// (conformance/drawing.json).

using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data.Converters;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform;
using Avalonia.Svg.Skia;
using SkiaSharp;

namespace Clarity.Avalonia;

public static class ResourcePictures
{
    /// <summary>Each picture once loaded, by name; null for one that is not there.</summary>
    static readonly Dictionary<string, IImage?> Loaded = [];

    /// <summary>The picture named in the resources folder; null when it is not there or no picture.</summary>
    public static IImage? Of(string? name)
    {
        if (string.IsNullOrEmpty(name))
            return null;
        if (Loaded.TryGetValue(name, out var known))
            return known;
        IImage? picture = null;
        var uri = MarkdownView.ResourceUri(MarkdownView.AppAssembly, name);
        try
        {
            if (AssetLoader.Exists(uri))
            {
                using var stream = AssetLoader.Open(uri);
                picture = name.EndsWith(".svg", StringComparison.OrdinalIgnoreCase) ? Svg(stream) : new Bitmap(stream);
            }
        }
        catch (Exception e) when (e is ArgumentException or InvalidOperationException or IOException)
        {
            picture = null;
        }
        Loaded[name] = picture;
        return picture;
    }

    /// <summary>An SVG picture; null when it draws nothing.</summary>
    internal static IImage? Svg(Stream stream)
    {
        var source = SvgSource.LoadFromStream(stream);
        return source.Picture is null ? null : new SvgPicture(source);
    }

    /// <summary>A picture by its name, as a key gives it in the language shown.</summary>
    public static readonly IValueConverter Named = new FuncValueConverter<string?, IImage?>(Of);
}

/// <summary>An image of a picture of the resources folder by its name: `image source "logo.svg"`.</summary>
public class ResourceImage : Image
{
    public static readonly StyledProperty<string?> PictureProperty = AvaloniaProperty.Register<ResourceImage, string?>(nameof(Picture));

    public string? Picture
    {
        get => GetValue(PictureProperty);
        set => SetValue(PictureProperty, value);
    }

    protected override Type StyleKeyOverride => typeof(Image);

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == PictureProperty)
            Source = ResourcePictures.Of(Picture);
    }
}

/// <summary>
/// An SVG drawn by Skia into a bitmap the size it is shown, twice over for
/// screens that scale, and shown as any other picture. Avalonia.Svg.Skia's
/// own image draws through a lease of the renderer's canvas, which on
/// Windows lost the part's scale and clip: the picture at its own size,
/// past its cell (2026-10-28).
/// </summary>
sealed class SvgPicture(SvgSource source) : IImage
{
    // The source owns the picture, so it is kept as long as the picture is.
    readonly SvgSource source = source;
    SKPicture picture => source.Picture!;
    const int Sharper = 2;
    const int Largest = 4096;
    WriteableBitmap? drawn;

    public Size Size => new(picture.CullRect.Width, picture.CullRect.Height);

    public void Draw(DrawingContext context, Rect sourceRect, Rect destRect)
    {
        if (Size.Width <= 0 || Size.Height <= 0 || sourceRect.Width <= 0 || sourceRect.Height <= 0)
            return;
        var scale = Math.Max(destRect.Width / sourceRect.Width, destRect.Height / sourceRect.Height) * Sharper;
        var width = (int)Math.Clamp(Math.Ceiling(Size.Width * scale), 1, Largest);
        var height = (int)Math.Clamp(Math.Ceiling(Size.Height * scale), 1, Largest);
        var bitmap = Bitmap(width, height);
        var (x, y) = (width / Size.Width, height / Size.Height);
        context.DrawImage(bitmap, new Rect(sourceRect.X * x, sourceRect.Y * y, sourceRect.Width * x, sourceRect.Height * y), destRect);
    }

    /// <summary>The picture drawn at this many pixels, kept until it is shown at another size.</summary>
    WriteableBitmap Bitmap(int width, int height)
    {
        if (drawn is { } kept && kept.PixelSize.Width == width && kept.PixelSize.Height == height)
            return kept;
        var info = new SKImageInfo(width, height, SKColorType.Bgra8888, SKAlphaType.Premul);
        var bitmap = new WriteableBitmap(new PixelSize(width, height), new Vector(96, 96), global::Avalonia.Platform.PixelFormat.Bgra8888, global::Avalonia.Platform.AlphaFormat.Premul);
        using (var pixels = bitmap.Lock())
        using (var surface = SKSurface.Create(info, pixels.Address, pixels.RowBytes))
        {
            var canvas = surface.Canvas;
            canvas.Clear(SKColors.Transparent);
            canvas.Scale(width / picture.CullRect.Width, height / picture.CullRect.Height);
            canvas.Translate(-picture.CullRect.Left, -picture.CullRect.Top);
            canvas.DrawPicture(picture);
            canvas.Flush();
        }
        drawn?.Dispose();
        drawn = bitmap;
        return bitmap;
    }
}
