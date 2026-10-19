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

    public void Say(Level level, Text text) => Messages.Add(new Message(level, text));

    /// <summary>Windows a running handler asked to open, and whether it asked to close this one.</summary>
    readonly List<(WindowViewModel Window, bool NewWindow)> opening = [];
    bool closing;
    int running;

    /// <summary>
    /// Runs a handler; a refusal or another failure is shown as an error message.
    /// What it asks of windows happens when it is done, on every stack alike:
    /// this window closes first, then the windows it opened open.
    /// </summary>
    public void Handle(Action handler)
    {
        running++;
        try
        {
            handler();
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
            return;
        var close = closing;
        var open = opening.ToList();
        closing = false;
        opening.Clear();
        if (close)
            CloseRequested?.Invoke();
        foreach (var (other, newWindow) in open)
            OpenRequested?.Invoke(other, newWindow);
    }

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
            CloseRequested?.Invoke();
    }

    /// <summary>Ends the application: the built-in <c>quit</c>.</summary>
    protected static void QuitApplication()
    {
        if (global::Avalonia.Application.Current?.ApplicationLifetime is global::Avalonia.Controls.ApplicationLifetimes.IClassicDesktopStyleApplicationLifetime desktop)
            desktop.Shutdown();
    }

    /// <summary>The window has opened with its data.</summary>
    public void Opened() => Handle(OnOpen);

    /// <summary>The window is about to close.</summary>
    public void Closing() => Handle(OnClose);

    /// <summary>A window this one opened has closed: the place to reload a list.</summary>
    public void Returned() => Handle(OnReturn);

    protected virtual void OnOpen() { }

    protected virtual void OnClose() { }

    protected virtual void OnReturn() { }
}
