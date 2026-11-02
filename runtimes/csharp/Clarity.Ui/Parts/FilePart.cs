// A file part (U1c), the same as on every stack: the name of a record's
// file, or that it has none, with buttons to open it in the program the
// system uses for its kind, save a copy of it, choose another and remove
// it, named in Clarity's own words. A stored file is fetched only to open it
// or save a copy (RecordFetch). Choosing and removing are changes, each
// running its change handlers; opening and saving a copy are not. A
// read-only one neither chooses nor removes.

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("file")]
public sealed class FilePart : Part
{
    const double Gap = 4;

    /// <summary>Opens the file in the program the system uses for its kind.</summary>
    public ContainerButton OpenButton { get; private set; } = null!;

    /// <summary>Saves a copy of the file where the person chooses.</summary>
    public ContainerButton SaveButton { get; private set; } = null!;

    /// <summary>Chooses a file with the system's dialog.</summary>
    public ContainerButton ChooseButton { get; private set; } = null!;

    /// <summary>Removes the file.</summary>
    public ContainerButton RemoveButton { get; private set; } = null!;

    /// <summary>A copy was saved: where it went.</summary>
    public event Action<string>? CopySaved;


    /// <summary>The record's file; none when it has none.</summary>
    public RecordFile? File => Bound is { } path ? Host.Read(path) as RecordFile : null;

    public bool IsEmpty => File is null;

    /// <summary>What shows of the file: its name, or that there is none.</summary>
    public string Shown => File?.Name ?? Locale.Translate("noFile", "No file");

    protected internal override void Made()
    {
        OpenButton = new ContainerButton(() => Locale.Translate("openFile", "Open"), () => _ = OpenIt())
        {
            Shows = () => File is not null,
            Usable = () => Enabled,
        };
        SaveButton = new ContainerButton(() => Locale.Translate("saveFileCopy", "Save a copy"), () => _ = SaveCopy())
        {
            Shows = () => File is not null,
            Usable = () => Enabled,
        };
        ChooseButton = new ContainerButton(() => Locale.Translate("chooseFile", "Choose a file"), () => _ = Choose())
        {
            Shows = () => !ReadOnly,
            Usable = () => Enabled,
        };
        RemoveButton = new ContainerButton(() => Locale.Translate("removeFile", "Remove the file"), Remove)
        {
            Shows = () => !ReadOnly,
            Usable = () => Enabled && File is not null,
        };
        AddChild(OpenButton);
        AddChild(SaveButton);
        AddChild(ChooseButton);
        AddChild(RemoveButton);
        Host.Changed += Placed;
    }

    /// <summary>The file changed: the buttons that show now take their places.</summary>
    void Placed()
    {
        Arrange(Bounds);
        Invalidate();
    }

    /// <summary>A file chosen: it replaces what the record had, a change.</summary>
    public void Take(string chosen, byte[] bytes)
    {
        if (ReadOnly || Bound is not { } path)
            return;
        Host.Write(Spec.Name ?? "", path, new RecordFile(RecordFiles.CleanName(chosen), Picture.New(bytes)));
        Fire("change");
    }

    /// <summary>The file removed, a change when there was one.</summary>
    public void Remove()
    {
        if (ReadOnly || File is null || Bound is not { } path)
            return;
        Host.Write(Spec.Name ?? "", path, null);
        Fire("change");
    }

    /// <summary>The file's bytes: new ones as chosen, a stored one fetched; none for none, or when fetching failed.</summary>
    public async Task<byte[]?> Bytes()
    {
        try
        {
            return await RecordFetch.Bytes(Host, Bound, File?.Content);
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{Spec.Name ?? Spec.Kind} cannot fetch its file: {e.Message}");
            return null;
        }
    }

    /// <summary>Opens the file in the program the system uses for its kind.</summary>
    public async Task OpenIt()
    {
        if (File is not { } file || await Bytes() is not { } bytes)
            return;
        Guarded(() => Host.Files.Show(file.Name, bytes));
    }

    /// <summary>Saves a copy where the person chooses, and tells where it went.</summary>
    public async Task SaveCopy()
    {
        if (File is not { } file || await Bytes() is not { } bytes)
            return;
        try
        {
            if (await Host.Files.SaveFile(Locale.Translate("saveFileCopy", "Save a copy"), file.Name, bytes) is { } place)
            {
                CopySaved?.Invoke(place);
                Host.Say(Level.Info, Text.Plain(Locale.Translate("fileCopySaved", "A copy is in your Downloads folder")));
            }
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{Spec.Name ?? Spec.Kind} cannot save a copy: {e.Message}");
        }
    }

    /// <summary>Asks the system's dialog for a file, and takes the one chosen.</summary>
    async Task Choose()
    {
        if (ReadOnly || !Enabled)
            return;
        try
        {
            if (await Host.Files.OpenFile(Locale.Translate("chooseFile", "Choose a file")) is { } chosen)
                Take(chosen.Name, chosen.Bytes);
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{Spec.Name ?? Spec.Kind} cannot choose a file: {e.Message}");
        }
    }

    void Guarded(Action action)
    {
        try
        {
            action();
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{Spec.Name ?? Spec.Kind} cannot open its file: {e.Message}");
        }
    }

    /// <summary>The name on the left, the buttons that show on the right, each as wide as its words.</summary>
    public override void Arrange(Box box)
    {
        Bounds = box;
        var right = box.Right;
        foreach (var button in new[] { RemoveButton, ChooseButton, SaveButton, OpenButton })
        {
            if (!button.IsShown)
            {
                button.Arrange(new Box(right, box.Y, 0, box.Height));
                continue;
            }
            var width = Math.Min(button.WidthFor(FontsNow, LookNow), Math.Max(0, right - box.X));
            right -= width;
            button.Arrange(new Box(right, box.Y, width, box.Height));
            right -= Gap;
        }
        NameBox = new Box(box.X, box.Y, Math.Max(0, right - box.X), box.Height);
    }

    /// <summary>Where the file's name shows.</summary>
    public Box NameBox { get; private set; }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        // Which buttons show depends on the file: placed again as it is now.
        Arrange(Bounds);
        var look = painter.Look;
        using (PressLooks.Dimmed(painter, Enabled))
        {
            painter.Text(Shown, NameBox.Inset(look.Padding), look.Color(IsEmpty ? "muted" : "text"), look.Size("text"));
            base.Draw(painter);
        }
    }
}
