// A folder part (X2), the same as on every stack: the path of a folder
// setting, where a file store keeps its files. It is typed like any text,
// taken when the input is left or Enter is pressed, and only a text that
// differs from the value is a change; or it is chosen with the system's
// folder picker through the "..." button inside its right edge, which says
// what it does in Clarity's own words when the pointer rests on it and takes
// no Tab stop, and a chosen folder is taken at once. Where nothing picks a
// folder (a browser) and on a read-only one, there is no button. A bound one writes
// its field; an unbound named one keeps its text in its state.

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("folder")]
public sealed class FolderPart : Part, ITakesTyped
{
    const double ButtonWidth = 28;

    TextEditor editor = null!;
    string own = "";
    string? shown;

    /// <summary>Where the person types the path.</summary>
    public TextEditor Editor => editor;

    /// <summary>The button that opens the system's folder picker.</summary>
    public InputSideButton ChooseButton { get; private set; } = null!;

    /// <summary>What the button does, in Clarity's own words: its name and the title of the picker.</summary>
    public string ChooseName => Locale.Translate("chooseFolder", "Choose a folder");


    /// <summary>The path it shows: its field, or the text it keeps unbound.</summary>
    public string Value => Bound is { } path
        ? Convert.ToString(Host.Read(path), System.Globalization.CultureInfo.InvariantCulture) ?? ""
        : State?.Text ?? own;

    protected internal override void Made()
    {
        editor = new TextEditor
        {
            Placeholder = Spec.Text("placeholder") is { } placeholder ? Host.Text(placeholder) : null,
            Enabled = () => Enabled,
            ReadOnly = () => ReadOnly,
            Marked = () => State?.HasError == true,
            Entered = Enter,
        };
        editor.FocusMoved += focused =>
        {
            if (focused)
                Fire("enter");
            else
            {
                Take();
                Fire("leave");
            }
        };
        ChooseButton = new InputSideButton(InputIcon.More, () => _ = Choose())
        {
            Name = ChooseName,
            Shows = () => !ReadOnly && Host.Files.PicksFolders,
            Enabled = () => Enabled,
        };
        AddChild(editor);
        AddChild(ChooseButton);
        Host.Changed += Sync;
        Sync();
    }

    /// <summary>A value set from elsewhere (code, the settings read again) shows as it is.</summary>
    void Sync()
    {
        var now = Value;
        if (now == shown)
            return;
        shown = now;
        editor.Text = now;
    }

    /// <summary>Takes what is typed: a change when it differs from the value.</summary>
    public void TakeTyped() => Take();

    /// <summary>Takes what is typed; see <see cref="TakeTyped"/>.</summary>
    public void Take()
    {
        Sync();
        var typed = editor.Text;
        if (typed == shown)
            return;
        shown = typed;
        if (Bound is { } path)
            Host.Write(Spec.Name ?? "", path, typed);
        else if (State is { } state)
        {
            state.Text = typed;
            Host.Edited(Spec.Name!);
        }
        else
            own = typed;
        Fire("change");
    }

    /// <summary>Takes a folder the picker gave: a change when it differs from the value.</summary>
    public void Chosen(string path)
    {
        if (ReadOnly || !Enabled)
            return;
        editor.Text = path;
        Take();
    }

    bool Enter()
    {
        Take();
        return PartTexts.Submit(this);
    }

    /// <summary>Asks the system's folder picker, and takes the folder chosen.</summary>
    async Task Choose()
    {
        if (ReadOnly || !Enabled)
            return;
        try
        {
            if (await Host.Files.ChooseFolder(ChooseName) is { Length: > 0 } folder)
                Chosen(folder);
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"{Spec.Name ?? Spec.Kind} cannot choose a folder: {e.Message}");
        }
    }

    /// <summary>The input, with the button inside its right edge when it shows.</summary>
    public override void Arrange(Box box)
    {
        Bounds = box;
        var shows = ChooseButton.IsShown;
        editor.RightInset = shows ? ButtonWidth + 2 : 0;
        editor.Arrange(box);
        ChooseButton.Arrange(shows ? new Box(box.Right - 2 - ButtonWidth, box.Y + 2, ButtonWidth, Math.Max(0, box.Height - 4)) : new Box(box.Right, box.Y, 0, box.Height));
    }

    public override void Draw(Painter painter)
    {
        Sync();
        base.Draw(painter);
    }
}
