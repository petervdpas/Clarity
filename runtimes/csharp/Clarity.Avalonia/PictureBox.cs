// A picture field: the picture of a record, with a button to load a new
// one from a file and one to remove it, shown while the pointer is on it. A
// stored picture is fetched on its own (Fetch); a new one shows from its
// bytes until the record is saved.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform.Storage;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public sealed class PictureBox : Grid
{
    public static readonly StyledProperty<Picture?> PictureProperty =
        AvaloniaProperty.Register<PictureBox, Picture?>(nameof(Picture), defaultBindingMode: BindingMode.TwoWay);

    /// <summary>
    /// The record whose picture this is: every stored picture is the same
    /// value, so another record must show its own.
    /// </summary>
    public static readonly StyledProperty<object?> RecordProperty = AvaloniaProperty.Register<PictureBox, object?>(nameof(Record));

    /// <summary>Gives the stored picture of the record, fetched when it shows.</summary>
    public static readonly StyledProperty<Func<byte[]?>?> FetchProperty = AvaloniaProperty.Register<PictureBox, Func<byte[]?>?>(nameof(Fetch));

    public static readonly StyledProperty<string> EmptyTextProperty = AvaloniaProperty.Register<PictureBox, string>(nameof(EmptyText), "");
    public static readonly StyledProperty<string> LoadTextProperty = AvaloniaProperty.Register<PictureBox, string>(nameof(LoadText), "");
    public static readonly StyledProperty<string> RemoveTextProperty = AvaloniaProperty.Register<PictureBox, string>(nameof(RemoveText), "");
    public static readonly StyledProperty<bool> IsReadOnlyProperty = AvaloniaProperty.Register<PictureBox, bool>(nameof(IsReadOnly));

    public Picture? Picture { get => GetValue(PictureProperty); set => SetValue(PictureProperty, value); }
    public Func<byte[]?>? Fetch { get => GetValue(FetchProperty); set => SetValue(FetchProperty, value); }

    /// <summary>Gives a file of the field's store: a picture linked from a file before its record is saved.</summary>
    public static readonly StyledProperty<Func<string, byte[]?>?> FetchFileProperty = AvaloniaProperty.Register<PictureBox, Func<string, byte[]?>?>(nameof(FetchFile));

    public Func<string, byte[]?>? FetchFile { get => GetValue(FetchFileProperty); set => SetValue(FetchFileProperty, value); }
    public object? Record { get => GetValue(RecordProperty); set => SetValue(RecordProperty, value); }
    public string EmptyText { get => GetValue(EmptyTextProperty); set => SetValue(EmptyTextProperty, value); }
    public string LoadText { get => GetValue(LoadTextProperty); set => SetValue(LoadTextProperty, value); }
    public string RemoveText { get => GetValue(RemoveTextProperty); set => SetValue(RemoveTextProperty, value); }
    public bool IsReadOnly { get => GetValue(IsReadOnlyProperty); set => SetValue(IsReadOnlyProperty, value); }

    readonly Image image = new() { Stretch = Stretch.Uniform };
    readonly TextBlock empty = new() { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center, Classes = { "placeholder" } };
    readonly Button load = new();
    readonly Button remove = new();
    readonly StackPanel actions = new()
    {
        Orientation = Orientation.Horizontal,
        Spacing = 4,
        Margin = new Thickness(4),
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Bottom,
        Opacity = 0,
    };

    /// <summary>The picture shown; null while there is none.</summary>
    public Bitmap? Shown => image.Source as Bitmap;

    public bool IsEmpty => Shown is null;

    public PictureBox()
    {
        Classes.Add("picture");
        Background = Brushes.Transparent;
        actions.Children.Add(load);
        actions.Children.Add(remove);
        Children.Add(empty);
        Children.Add(image);
        Children.Add(actions);
        load.Click += async (_, _) => await Choose();
        remove.Click += (_, _) => Remove();
        PointerEntered += (_, _) => actions.Opacity = IsReadOnly ? 0 : 1;
        PointerExited += (_, _) => actions.Opacity = 0;
        Show();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == PictureProperty || change.Property == FetchProperty || change.Property == FetchFileProperty || change.Property == RecordProperty || change.Property == EmptyTextProperty
            || change.Property == LoadTextProperty || change.Property == RemoveTextProperty || change.Property == IsReadOnlyProperty)
            Show();
    }

    void Show()
    {
        var bytes = Picture switch
        {
            null => null,
            { Bytes: { } b } => b,
            { FilePath: { } path } => FetchFile?.Invoke(path),
            _ => Fetch?.Invoke(),
        };
        (image.Source as IDisposable)?.Dispose();
        image.Source = bytes is null ? null : Decoded(bytes);
        empty.Text = EmptyText;
        empty.IsVisible = image.Source is null;
        load.Content = LoadText;
        remove.Content = RemoveText;
        remove.IsVisible = Picture is not null;
        actions.IsVisible = !IsReadOnly;
    }

    static Bitmap? Decoded(byte[] bytes)
    {
        try
        {
            using var stream = new MemoryStream(bytes);
            return new Bitmap(stream);
        }
        catch (Exception e) when (e is ArgumentException or InvalidOperationException or IOException)
        {
            return null;
        }
    }

    /// <summary>Takes the bytes as the new picture when they are one; false when they are not.</summary>
    public bool Load(byte[] bytes)
    {
        if (Pictures.KindOf(bytes) is null)
        {
            Classes.Set("problem", true);
            return false;
        }
        Classes.Set("problem", false);
        Picture = Picture.New(bytes);
        return true;
    }

    public void Remove()
    {
        Classes.Set("problem", false);
        Picture = null;
    }

    async Task Choose()
    {
        if (TopLevel.GetTopLevel(this)?.StorageProvider is not { } storage)
            return;
        var files = await storage.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            AllowMultiple = false,
            FileTypeFilter = [new FilePickerFileType(LoadText) { Patterns = ["*.png", "*.jpg", "*.jpeg", "*.gif", "*.bmp", "*.webp"] }],
        });
        if (files.Count == 0)
            return;
        await using var stream = await files[0].OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        Load(memory.ToArray());
    }
}
