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

    /// <summary>The messages on screen, newest last; the shell's, once it shows the window.</summary>
    public ObservableCollection<Message> Messages { get; internal set; } = [];

    /// <summary>A window this one asks to open.</summary>
    public event Action<WindowViewModel>? OpenRequested;

    /// <summary>The window asks to be closed.</summary>
    public event Action? CloseRequested;

    public void Say(Level level, Text text) => Messages.Add(new Message(level, text));

    /// <summary>Runs a handler; a refusal or another failure is shown as an error message.</summary>
    public void Handle(Action handler)
    {
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
    }

    /// <summary>The row selected in a table or list.</summary>
    protected static R Selected<R>(PartState part) where R : class =>
        part.SelectedItem as R ?? throw new InvalidValueException($"no row of {part.Name} is selected");

    protected void Open(WindowViewModel other) => OpenRequested?.Invoke(other);

    protected void Close() => CloseRequested?.Invoke();

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
