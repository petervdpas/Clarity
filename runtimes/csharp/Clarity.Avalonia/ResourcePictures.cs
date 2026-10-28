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
                picture = name.EndsWith(".svg", StringComparison.OrdinalIgnoreCase) ? new SvgImage { Source = SvgSource.LoadFromStream(stream) } : new Bitmap(stream);
            }
        }
        catch (Exception e) when (e is ArgumentException or InvalidOperationException or IOException)
        {
            picture = null;
        }
        Loaded[name] = picture;
        return picture;
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
