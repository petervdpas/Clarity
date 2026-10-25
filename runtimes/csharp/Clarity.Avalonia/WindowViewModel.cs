// The base of every window's ViewModel. Handlers run through Handle, so what
// goes wrong in one becomes an error message instead of a crash; opening and
// closing windows go through the Shell that shows them.

using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public abstract class WindowViewModel(Db db) : ObservableObject
{
    /// <summary>The databases of the application.</summary>
    public Db Db { get; } = db;

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
    public bool Handle(Action handler)
    {
        running++;
        var ran = false;
        try
        {
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
        finally
        {
            running--;
        }
        if (running > 0)
            return ran;
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
    protected static void QuitApplication()
    {
        if (global::Avalonia.Application.Current?.ApplicationLifetime is global::Avalonia.Controls.ApplicationLifetimes.IClassicDesktopStyleApplicationLifetime desktop)
            desktop.Shutdown();
    }

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
