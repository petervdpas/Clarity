// The base of every window's ViewModel. Handlers run through Handle, so what
// goes wrong in one becomes an error message instead of a crash; opening and
// closing windows go through the Shell that shows them.

using System.Collections.ObjectModel;
using System.Runtime.CompilerServices;
using System.Text.Json.Nodes;
using CommunityToolkit.Mvvm.ComponentModel;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public abstract class WindowViewModel(Db? db) : ObservableObject
{
    /// <summary>The databases of the application; none in a mirror.</summary>
    public Db Db { get; } = db!;

    /// <summary>
    /// Whether this is a mirror of a window whose handlers run on a server
    /// (DESIGN 7.6): made without a database, it loads nothing itself and
    /// sends its commands to <see cref="Remote"/>.
    /// </summary>
    public bool IsMirror { get; } = db is null;

    /// <summary>Where a mirror sends its commands.</summary>
    public IRemote? Remote { get; set; }

    /// <summary>The number of this open window, its own: what belongs to it, such as an engine's state per window, goes by it.</summary>
    public long Instance { get; } = OpenWindows.Next();

    /// <summary>The window's title.</summary>
    public virtual string Title => "";

    /// <summary>A dialog opens in a desktop window of its own; other windows open in place.</summary>
    public virtual bool IsDialog => false;

    /// <summary>The size its desktop window opens at (<c>size 320 x 460</c>); none for the standard size.</summary>
    public virtual global::Avalonia.Size? WindowSize => null;

    /// <summary>The messages on screen, newest last; the shell's, once it shows the window.</summary>
    public ObservableCollection<Message> Messages { get; internal set; } = [];

    /// <summary>A window this one asks to open: in place, or in a desktop window of its own.</summary>
    public event Action<WindowViewModel, bool>? OpenRequested;

    /// <summary>The window asks to be closed.</summary>
    public event Action? CloseRequested;

    /// <summary>The window asks a question, when the handler that asks has finished (DESIGN 5.14).</summary>
    public event Action<Question>? AskRequested;

    public void Say(Level level, Text text) => Messages.Add(new Message(level, text));

    /// <summary>The edits of this window (DESIGN 5.29), which Ctrl+Z takes back.</summary>
    public EditBus Edits { get; } = new();

    /// <summary>The window's data as the last edit left it; what an edit changed is the difference.</summary>
    JsonObject? baseline;

    /// <summary>The database step the edit that runs made, 0 when none.</summary>
    long editStep;

    /// <summary>Whether an edit's handler runs, whose changes the edit keeps.</summary>
    bool editing;

    /// <summary>Whether an undo puts data back, which no part's change makes an edit.</summary>
    bool applying;

    /// <summary>The scope of the part that had the focus last, for an Undo in a menu.</summary>
    public string? FocusedScope { get; set; }

    /// <summary>Names of a part's own state (an unbound input's text) start so; no name of the window's data does.</summary>
    const string PartState = "#parts.";

    /// <summary>The name the person's settings go by in an edit; no name of the window's data has a #.</summary>
    const string SettingsState = "#settings";

    /// <summary>The person's settings, which parts may change (bound to <c>settings.x</c>); none in a window without them.</summary>
    protected virtual ISettings? AppSettings => null;

    /// <summary>The window's data, and the parts' own text and ticks, as an edit compares them.</summary>
    JsonObject DataNow()
    {
        var of = Snapshot.Of(this);
        var now = of["data"]!.AsObject();
        foreach (var (name, node) in of["parts"]!.AsObject())
        {
            if (node is not JsonObject part)
                continue;
            var own = new JsonObject();
            foreach (var key in new[] { "text", "checked" })
            {
                if (part[key] is { } value)
                    own[key] = value.DeepClone();
            }
            if (own.Count > 0)
                now[PartState + name] = own;
        }
        // A part bound to a setting changes the settings, not the window's data.
        if (AppSettings is { } settings)
            now[SettingsState] = settings.ToJson();
        return now;
    }

    /// <summary>An edit of <paramref name="part"/> ended: what it changed since the last one goes on the bus.</summary>
    public void Edited(string part, string scope)
    {
        // What a handler or an undo changed is no edit of the person's, though the bound parts follow it.
        if (running > 0 || applying)
            return;
        var now = DataNow();
        var changes = EditBus.Between(baseline ?? now, now);
        // The change handler of a value just changed joins its edit: one change, one Ctrl+Z.
        if (joining is { } joined && joined.Part == part && Edits.LastOf(part) == joined)
        {
            joined.Changes.AddRange(changes);
            if (editStep != 0)
                joined.Step = editStep;
        }
        else
        {
            var edit = new Edit(part, scope, changes, editStep);
            Edits.Push(edit);
            if (!editing)
            {
                joining = edit;
                global::Avalonia.Threading.Dispatcher.UIThread.Post(() => joining = null, global::Avalonia.Threading.DispatcherPriority.Background);
            }
        }
        baseline = now;
        editStep = 0;
    }

    /// <summary>The edit a value change just made, which its change handler joins.</summary>
    Edit? joining;

    /// <summary>
    /// Runs the handler of an edit of a part with undo (a canvas's drag, drop
    /// or bend, a change): as one database step; <paramref name="ends"/> ends the edit.
    /// </summary>
    public void EditOf(string part, string scope, bool ends, Action handler)
    {
        // A mirror's handlers run on the server, which keeps its edits.
        if (Remote is not null)
        {
            handler();
            return;
        }
        baseline ??= DataNow();
        var before = Db?.LastStep ?? 0;
        editing = true;
        try
        {
            Handle(() =>
            {
                if (Db is null)
                    handler();
                else
                    Db.Step(handler);
            }, null, nameof(EditOf));
        }
        finally
        {
            editing = false;
        }
        if (Db is not null && Db.LastStep != before)
            editStep = Db.LastStep;
        if (ends)
            Edited(part, scope);
    }

    /// <summary>Takes back (<paramref name="back"/>) or does again the last edit of <paramref name="scope"/>.</summary>
    public void UndoEdit(string scope, bool back)
    {
        var edit = back ? Edits.Undo(scope) : Edits.Redo(scope);
        if (edit is null)
            return;
        try
        {
            if (edit.Step > 0 && Db is not null)
            {
                if (back)
                    Db.Undo(edit.Step);
                else
                    Db.Redo(edit.Step);
            }
        }
        catch (RefusedException e)
        {
            Edits.Refused(edit, back);
            Say(Level.Error, e.Text);
            return;
        }
        var data = new JsonObject();
        var parts = new JsonObject();
        JsonObject? settings = null;
        // Back from the last change to the first, so the first one's before is what stays; again from the first to the last.
        foreach (var changed in back ? Enumerable.Reverse(edit.Changes) : edit.Changes)
        {
            var value = (back ? changed.Before : changed.After)?.DeepClone();
            if (changed.Name == SettingsState)
                settings = value as JsonObject;
            else if (changed.Name.StartsWith(PartState, StringComparison.Ordinal))
                parts[changed.Name[PartState.Length..]] = value;
            else
                data[changed.Name] = value;
        }
        applying = true;
        try
        {
            Snapshot.Apply(this, new JsonObject { ["data"] = data, ["parts"] = parts });
            if (settings is not null)
                AppSettings?.Apply(settings);
        }
        finally
        {
            applying = false;
        }
        baseline = DataNow();
        Redraw();
    }

    /// <summary><c>-> undo</c> and <c>-> redo</c>: the last edit of the part that had the focus last.</summary>
    protected void UndoFocused(bool back)
    {
        if (FocusedScope is { } scope)
            UndoEdit(scope, back);
    }

    /// <summary>Asks the shell to put a text on the clipboard: <c>copy text</c>.</summary>
    public event Action<Text>? CopyRequested;

    /// <summary><c>copy text</c>: the text, as it shows, goes on the clipboard; the last copy counts.</summary>
    public void Copy(Text text) => CopyRequested?.Invoke(text);

    /// <summary>
    /// Runs the app's <c>on unconnected</c> on this window, which is not shown
    /// (DESIGN 5.27): what it says and the windows it opens go to <paramref name="unconnecting"/>.
    /// </summary>
    public void RunUnconnected(Unconnecting unconnecting, Action<string> handler)
    {
        Messages = unconnecting.Said;
        OpenRequested += (other, _) => unconnecting.Open(other);
        Handle(() => handler(unconnecting.Reason));
    }

    /// <summary>Windows a running handler asked to open, and whether it asked to close this one.</summary>
    readonly List<(WindowViewModel Window, bool NewWindow)> opening = [];
    readonly List<Question> asking = [];

    /// <summary>For a dialog asked for: runs the handler of the window that asked, with what it closes with.</summary>
    Action<object?>? whenClosedWith;
    object? closedWith;
    bool closingWith;
    bool closing;
    int running;

    /// <summary>
    /// Runs a handler; a refusal or another failure is shown as an error message.
    /// What it asks of windows happens when it is done, on every stack alike:
    /// this window closes first, then the windows it opened open. False when
    /// it refused or failed, so a wizard keeps its step.
    /// </summary>
    public bool Handle(Action handler, object?[]? args = null, [CallerMemberName] string command = "")
    {
        // A mirror's handlers run on the server: the command goes there, by its name.
        if (Remote is not null)
        {
            Remote.Command(this, command, args ?? []);
            return true;
        }
        // While the screen waits for a long engine call, it takes no other handler.
        if (LongCall.Waiting)
            return false;
        running++;
        var ran = false;
        try
        {
            // The outermost handler is watched: its engine calls show how far they are.
            if (running == 1)
                Watching.Run(LongCall.Start(), handler);
            else
                handler();
            ran = true;
            // The canvases show what the handler left, drawn anew.
            if (running == 1)
                Redraw();
        }
        catch (RefusedException e)
        {
            Say(Level.Error, e.Text);
        }
        catch (ClarityException e)
        {
            Say(Level.Error, Text.Plain(e.Message));
        }
        catch (Exception e)
        {
            // Fail safe: an error nobody foresaw says why, and the window keeps working.
            Console.Error.WriteLine($"{command} failed: {e}");
            Say(Level.Error, Text.Plain(e.Message));
        }
        finally
        {
            running--;
            if (running == 0)
                LongCall.End();
        }
        if (running > 0)
            return ran;
        // What a handler that is no edit changed is not taken back: the edits start from here.
        if (!editing)
            baseline = DataNow();
        var close = closing;
        var open = opening.ToList();
        var asks = asking.ToList();
        closing = false;
        opening.Clear();
        asking.Clear();
        if (close)
            Closed();
        foreach (var (other, newWindow) in open)
            OpenRequested?.Invoke(other, newWindow);
        // The questions it asked, after what changed is on screen.
        foreach (var question in asks)
            AskRequested?.Invoke(question);
        return ran;
    }

    /// <summary>
    /// Asks <paramref name="question"/> in Clarity's own dialog when the
    /// handler has finished; <paramref name="answered"/> runs the handler the
    /// answer goes to. Cancelling runs nothing.
    /// </summary>
    protected void Ask(Text question, AskForm form, Action<object?> answered)
    {
        var asked = new Question(question, form, answered);
        if (running > 0)
            asking.Add(asked);
        else
            AskRequested?.Invoke(asked);
    }

    /// <summary>The person answered: the handler the answer goes to runs, as any handler does.</summary>
    public void Answer(Question question, object? answer) => Handle(() => question.Answered(answer));

    /// <summary>
    /// A picture or a file of the database, fetched by <paramref name="here"/>;
    /// a mirror asks the server for it by the name of its <paramref name="fetcher"/> (DESIGN 7.6).
    /// </summary>
    protected Task<byte[]?> Fetched(string fetcher, Func<byte[]?> here, params object?[] args) =>
        Remote is { } remote ? remote.Fetch(this, fetcher, args) : Task.FromResult(here());

    /// <summary>The row selected in a table or list.</summary>
    protected static R Selected<R>(PartState part) where R : class =>
        part.SelectedItem as R ?? throw new InvalidValueException($"no row of {part.Name} is selected");

    /// <summary>
    /// Opens a window: in place, on top of this one, or in a desktop window of
    /// its own when the design marks it (`in new window`).
    /// </summary>
    protected void Open(WindowViewModel other, bool newWindow = false)
    {
        if (running > 0)
            opening.Add((other, newWindow));
        else
            OpenRequested?.Invoke(other, newWindow);
    }

    protected void Close()
    {
        if (running > 0)
            closing = true;
        else
            Closed();
    }

    /// <summary>Closes this dialog with <paramref name="value"/> (<c>close with value</c>), which the handler that asked for it gets.</summary>
    protected void CloseWith(object? value)
    {
        closedWith = value;
        closingWith = true;
        Close();
    }

    /// <summary>The window closes; a dialog that closed with a value then gives it to the window that asked for it.</summary>
    void Closed()
    {
        CloseRequested?.Invoke();
        if (!closingWith)
            return;
        closingWith = false;
        whenClosedWith?.Invoke(closedWith);
    }

    /// <summary>
    /// Opens the dialog <paramref name="dialog"/> (<c>ask PickFriend then lend(album)</c>);
    /// what it closes with runs <paramref name="answered"/> on this window, as any
    /// handler runs. Closed any other way, it runs nothing.
    /// </summary>
    protected void Ask(WindowViewModel dialog, Action<object?> answered)
    {
        dialog.whenClosedWith = value => Handle(() => answered(value));
        Open(dialog);
    }

    /// <summary>Ends the application: the built-in <c>quit</c>.</summary>
    protected static void QuitApplication() => DesktopProgram.Current?.Quit();

    /// <summary>Opens Clarity's keys window over the window the person works in: the built-in <c>keys</c> (DESIGN 5.28).</summary>
    protected static void ShowKeys() => DesktopProgram.Current?.ShowKeys();

    /// <summary>The window has opened with its data.</summary>
    public void Opened() => Handle(OnOpen);

    /// <summary>The window is about to close: its on close runs, then it is reported closed.</summary>
    public void Closing()
    {
        Handle(OnClose);
        OpenWindows.Close(Instance);
    }

    /// <summary>A window this one opened has closed: the place to reload a list.</summary>
    public void Returned() => Handle(OnReturn);

    /// <summary>
    /// Draws the window's canvases anew, each by its draw handler on an
    /// empty drawing (conformance/drawing.json): after every handler, and
    /// when the window opens.
    /// </summary>
    protected virtual void Redraw() { }

    protected virtual void OnOpen() { }

    protected virtual void OnClose() { }

    protected virtual void OnReturn() { }
}
