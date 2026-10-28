// Clarity's file input on Avalonia, the same as on the web (U1c): the name
// of a record's file, or that it has none, with buttons to choose one, open
// it in the program the system uses for its kind, save a copy in the
// person's Downloads folder, and remove it. A stored file is fetched only
// to open it or save a copy (Fetch, or FetchFile for one linked from the
// field's store). Choosing and removing are changes; opening and saving a
// copy are not.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Layout;
using Avalonia.Platform.Storage;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public sealed class FileInput : Grid, IInput
{
    public static readonly StyledProperty<RecordFile?> FileProperty =
        AvaloniaProperty.Register<FileInput, RecordFile?>(nameof(File), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<object?> RecordProperty = AvaloniaProperty.Register<FileInput, object?>(nameof(Record));
    public static readonly StyledProperty<Func<Task<byte[]?>>?> FetchProperty = AvaloniaProperty.Register<FileInput, Func<Task<byte[]?>>?>(nameof(Fetch));
    public static readonly StyledProperty<Func<string, Task<byte[]?>>?> FetchFileProperty = AvaloniaProperty.Register<FileInput, Func<string, Task<byte[]?>>?>(nameof(FetchFile));
    public static readonly StyledProperty<string> EmptyTextProperty = AvaloniaProperty.Register<FileInput, string>(nameof(EmptyText), "");
    public static readonly StyledProperty<string> ChooseTextProperty = AvaloniaProperty.Register<FileInput, string>(nameof(ChooseText), "");
    public static readonly StyledProperty<string> OpenTextProperty = AvaloniaProperty.Register<FileInput, string>(nameof(OpenText), "");
    public static readonly StyledProperty<string> SaveTextProperty = AvaloniaProperty.Register<FileInput, string>(nameof(SaveText), "");
    public static readonly StyledProperty<string> RemoveTextProperty = AvaloniaProperty.Register<FileInput, string>(nameof(RemoveText), "");
    public static readonly StyledProperty<bool> IsReadOnlyProperty = AvaloniaProperty.Register<FileInput, bool>(nameof(IsReadOnly));

    /// <summary>The record's file, or null for none.</summary>
    public RecordFile? File { get => GetValue(FileProperty); set => SetValue(FileProperty, value); }
    /// <summary>The record the file belongs to: when it changes, so may its stored file.</summary>
    public object? Record { get => GetValue(RecordProperty); set => SetValue(RecordProperty, value); }
    /// <summary>Fetches the stored file of the record.</summary>
    public Func<Task<byte[]?>>? Fetch { get => GetValue(FetchProperty); set => SetValue(FetchProperty, value); }
    /// <summary>Fetches a file of the field's own store, by its path there.</summary>
    public Func<string, Task<byte[]?>>? FetchFile { get => GetValue(FetchFileProperty); set => SetValue(FetchFileProperty, value); }
    public string EmptyText { get => GetValue(EmptyTextProperty); set => SetValue(EmptyTextProperty, value); }
    public string ChooseText { get => GetValue(ChooseTextProperty); set => SetValue(ChooseTextProperty, value); }
    public string OpenText { get => GetValue(OpenTextProperty); set => SetValue(OpenTextProperty, value); }
    public string SaveText { get => GetValue(SaveTextProperty); set => SetValue(SaveTextProperty, value); }
    public string RemoveText { get => GetValue(RemoveTextProperty); set => SetValue(RemoveTextProperty, value); }
    public bool IsReadOnly { get => GetValue(IsReadOnlyProperty); set => SetValue(IsReadOnlyProperty, value); }

    public event EventHandler? ValueChanged;

    /// <summary>A copy was saved: the file it went to.</summary>
    public event Action<string>? CopySaved;

    readonly TextBlock name = new() { VerticalAlignment = VerticalAlignment.Center, TextTrimming = global::Avalonia.Media.TextTrimming.CharacterEllipsis };
    readonly Button choose = new();
    readonly Button open = new();
    readonly Button save = new();
    readonly Button remove = new();

    /// <summary>What shows of the file: its name, or the empty text.</summary>
    public string Shown => name.Text ?? "";

    public FileInput()
    {
        Classes.Add("file");
        ColumnDefinitions = new ColumnDefinitions("*,Auto,Auto,Auto,Auto");
        ColumnSpacing = 4;
        Children.Add(name);
        Button[] buttons = [choose, open, save, remove];
        for (var i = 0; i < buttons.Length; i++)
        {
            SetColumn(buttons[i], i + 1);
            Children.Add(buttons[i]);
        }
        choose.Click += async (_, _) => await Choose();
        open.Click += (_, _) => _ = OpenIt();
        save.Click += (_, _) => _ = SaveCopy();
        remove.Click += (_, _) => Remove();
        Show();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == FileProperty || change.Property == IsReadOnlyProperty || change.Property == EmptyTextProperty
            || change.Property == ChooseTextProperty || change.Property == OpenTextProperty || change.Property == SaveTextProperty || change.Property == RemoveTextProperty)
            Show();
    }

    void Show()
    {
        name.Text = File?.Name ?? EmptyText;
        name.Classes.Set("empty", File is null);
        choose.Content = ChooseText;
        open.Content = OpenText;
        save.Content = SaveText;
        remove.Content = RemoveText;
        choose.IsVisible = remove.IsVisible = !IsReadOnly;
        open.IsVisible = save.IsVisible = File is not null;
        remove.IsEnabled = File is not null;
    }

    /// <summary>A file chosen: it replaces what the record had, a change.</summary>
    public void Take(string chosen, byte[] bytes)
    {
        if (IsReadOnly)
            return;
        File = new RecordFile(RecordFiles.CleanName(chosen), Picture.New(bytes));
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>The file removed, a change when there was one.</summary>
    public void Remove()
    {
        if (IsReadOnly || File is null)
            return;
        File = null;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>The file's bytes: new ones as chosen, a stored one fetched; null for none.</summary>
    public async Task<byte[]?> Bytes() => File?.Content switch
    {
        null => null,
        { Bytes: { } bytes } => bytes,
        { FilePath: { } path } => FetchFile is { } file ? await file(path) : null,
        _ => Fetch is { } fetch ? await fetch() : null,
    };

    /// <summary>Opens the file in the program the system uses for its kind; in a browser, in a tab of its own.</summary>
    public async Task OpenIt()
    {
        if (File is not { } file || await Bytes() is not { } bytes)
            return;
        if (BrowserPage.Current is { } page)
            page.Show(file.Name, bytes);
        else
            RecordFiles.Open(file.Name, bytes);
    }

    /// <summary>Saves a copy in the person's Downloads folder, and tells where; in a browser, as its download.</summary>
    public async Task SaveCopy()
    {
        if (File is not { } file || await Bytes() is not { } bytes)
            return;
        if (BrowserPage.Current is { } page)
        {
            page.Download(file.Name, bytes);
            CopySaved?.Invoke(file.Name);
        }
        else
            CopySaved?.Invoke(RecordFiles.SaveCopy(file.Name, bytes));
    }

    async Task Choose()
    {
        if (TopLevel.GetTopLevel(this)?.StorageProvider is not { } storage)
            return;
        var files = await storage.OpenFilePickerAsync(new FilePickerOpenOptions { AllowMultiple = false });
        if (files.Count == 0)
            return;
        await using var stream = await files[0].OpenReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        Take(files[0].Name, memory.ToArray());
    }
}
