// What a generated window gives its parts beyond its values (IPartHost): the
// file dialogs its host made, the stored files of its records' fields, its
// messages, the commands the generator writes for each canvas and custom
// part, its engines beside the screen, and the keys the person set. Each is
// found by the name the C# generator gives it
// (crates/clarity-gen/src/csharp/windows.rs).

using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Windows.Input;
using Clarity.Runtime;
using Pointer = Clarity.Runtime.Pointer;

namespace Clarity.Ui;

public sealed partial class ViewModelHost
{
    /// <summary>The file dialogs the window's host made; one that picks nothing where it made none.</summary>
    public IFilePicker Files { get; init; } = new NoFilePicker();

    /// <summary>
    /// The stored file of a record's field through the members the generator
    /// writes for it: <c>TaskAttachmentPicture</c> for the one the record
    /// keeps, <c>TaskAttachmentFile</c> for a file of the field's store by its
    /// path. Each goes through the view model's Fetched, so a window mirrored
    /// in a browser asks the server for it.
    /// </summary>
    public async Task<byte[]?> Fetch(string bind, string? storePath)
    {
        var dot = bind.IndexOf('.');
        if (dot <= 0 || dot == bind.Length - 1)
            return null;
        var member = Pascal(bind[..dot]) + Pascal(bind[(dot + 1)..]);
        if (storePath is not null)
            return Read(member + "File") is Func<string, Task<byte[]?>> file ? await file(storePath) : null;
        return Read(member + "Picture") is Func<Task<byte[]?>> picture ? await picture() : null;
    }

    public void Say(Level level, Text text) => Model.Say(level, text);

    // -- Canvases and custom parts ---------------------------------------------

    /// <summary>The generator's prefix of each pointer event's command.</summary>
    static readonly Dictionary<string, string> PointerCommands = new()
    {
        ["click"] = "ClickAt_",
        ["drag"] = "DragAt_",
        ["drop"] = "DropAt_",
        ["bend"] = "BendAt_",
    };

    /// <summary>A part's name as the generator's member: in Pascal case, with _ after a name the view model takes.</summary>
    static IEnumerable<string> Members(string part)
    {
        var pascal = Pascal(part);
        yield return pascal;
        yield return pascal + "_";
    }

    ICommand? CommandOf(string prefix, string part)
    {
        foreach (var member in Members(part))
            if (Model.GetType().GetProperty(prefix + member + "Command", BindingFlags.Public | BindingFlags.Instance)?.GetValue(Model) is ICommand command)
                return command;
        return null;
    }

    /// <summary>Runs the canvas's <c>ClickAt_Pad</c> (DragAt_, DropAt_, BendAt_), which runs its handler in the view model's EditOf when it has an undo scope.</summary>
    public bool? Pointed(string part, string @event, Pointer at)
    {
        if (!PointerCommands.TryGetValue(@event, out var prefix) || CommandOf(prefix, part) is not { } command)
            return null;
        if (!command.CanExecute(at))
            return false;
        command.Execute(at);
        return true;
    }

    /// <summary>Runs the canvas's <c>CanvasAt_Pad</c>, which applies the action as one edit.</summary>
    public bool? Acted(string part, JsonObject action)
    {
        if (CommandOf("CanvasAt_", part) is not { } command)
            return null;
        if (!command.CanExecute(action))
            return false;
        command.Execute(action);
        return true;
    }

    /// <summary>The application's <c>ScreenEngines.&lt;Name&gt;</c>, which makes a new engine each call.</summary>
    public Func<IFrames>? Engine(string name)
    {
        var type = Model.GetType();
        var engines = type.Assembly.GetType($"{type.Namespace}.ScreenEngines") ?? type.Assembly.GetTypes().FirstOrDefault(t => t.Name == "ScreenEngines");
        return engines?.GetField(Pascal(name), BindingFlags.Public | BindingFlags.Static)?.GetValue(null) as Func<IFrames>;
    }

    /// <summary>Runs the view model's <c>Told_Gauge</c>, which the generator writes for a part with wired tells.</summary>
    public void Told(string part, string tell, JsonElement[] values)
    {
        foreach (var member in Members(part))
            if (Model.GetType().GetMethod("Told_" + member, BindingFlags.Public | BindingFlags.Instance, [typeof(string), typeof(JsonElement[])]) is { } told)
            {
                told.Invoke(Model, [tell, values]);
                return;
            }
    }

    // -- Keys ------------------------------------------------------------------

    public IReadOnlyList<string> KeysOf(string command, string? designed) => PersonKeys.Of(command, designed);
}
